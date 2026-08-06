# Auth Backend Hardening — Design

## Status

Approved by user (all sections). This is the design spec for the "Bloc 1 : Hardening auth backend" plan.

## Goal

Harden the Godwit backend's cookie/JWT auth surface after the UI auth feature shipped to `main`. Four independent, in-scope hardening tracks:

1. **CSRF** on `/auth/refresh` and `/auth/logout` (currently outside CSRF protection).
2. **Brute-force rate limit** on `/auth/login`, keyed by client IP.
3. **Per-user session revocation** (self-service "sign out all devices").
4. **Production config hardening + E2E tests** covering the above.

Deliberately out of scope: rotation-family / replay-detection (per-user revocation only), UI work (separate Bloc 2), and dependency upgrades (separate Bloc 3).

## Current state (verified)

- Routes registered in `crates/godwit-api/src/admin/auth.rs::router()`: `/auth/login`, `/auth/refresh`, `/auth/logout`, `/auth/oidc/:provider`, `/auth/oidc/:provider/callback`, `/auth/saml/:provider/acs`.
- These are merged in `admin::router()` **outside** the `protected` router, so `/auth/refresh` and `/auth/logout` are **not** under `jwt_auth`, and the CSRF origin-check (which lives inside `jwt_auth`, `middleware.rs:88-101`) does not cover them.
- `/auth/refresh` and `/auth/logout` authenticate via the `godwit_refresh` httpOnly cookie (scoped `Path=/api/v1/auth`) and must work **without** a valid access token — so they cannot simply be moved under `jwt_auth`.
- Refresh tokens are single-use: `refresh` deletes the used row and issues a new one; stored hashed (`token_hash`) in `refresh_tokens` with `user_id`, `expires_at`, `created_at`. No per-user delete exists (`RefreshTokenRepository` only has `delete`, `delete_by_hash`).
- `RateLimiter` (`rate_limit.rs`) is keyed by `(Uuid api_key, String model)` for chat — not reusable as-is for login.
- `ApiError::RateLimited(Option<u64>)` → 429 with `RETRY_AFTER` already exists.
- Server mounts with `axum::serve(listener, app)` (**no** `ConnectInfo`). No existing IP extraction; no `trust_proxy` handling.
- Integration tests in `crates/godwit-api/tests/router_integration.rs` drive the real assembled router via `tower::ServiceExt::oneshot` (no TCP listener → no real `ConnectInfo`). `build_app` builds config from `test_config()`; tests can mutate config (e.g. existing `set_origin` pattern in `middleware.rs` tests).

## Design

### 1. CSRF on `/auth/refresh` and `/auth/logout`

**Problem:** `/auth/refresh` and `/auth/logout` are cookie-authenticated, state-changing (POST) routes that currently lack the origin check that protects the rest of the admin API.

**Approach:**
- **Factor the existing origin-check** in `jwt_auth` (`middleware.rs`) into a reusable helper, e.g.:

  ```rust
  pub fn origin_allowed(state: &AppState, method: &Method, headers: &HeaderMap) -> bool
  ```

  Behavior unchanged: no-op when `allowed_cookie_origin` is empty; for state-changing methods (POST/PUT/PATCH/DELETE) require an `Origin` header exactly equal to `allowed_cookie_origin`, else deny. `jwt_auth` calls this helper (same behavior, no regression).

- **New middleware `cookie_csrf`** (`from_fn_with_state`) that runs the origin check but **does not** validate any token. Return `403 Forbidden` on failure.

- **Apply** `cookie_csrf` via `route_layer` only to `/auth/refresh` and `/auth/logout`. `login`, `oidc_start/callback`, `saml_acs` remain unaffected (login is a form submission without cross-site CSRF benefit; OIDC start redirects to another origin legitimately).

- **Ordering:** the check must run before the handler logic so a rejected cross-site request never consumes (rotates) a refresh token or deletes rows.

- **Regression safety:** with the default empty `allowed_cookie_origin` (dev/same-origin topology), the check is a no-op → zero behavior change.

**Migration of `auth::router()`:** restructure so `/auth/refresh` and `/auth/logout` live in a sub-router that has the `cookie_csrf` layer, while the other auth routes do not.

### 2. Brute-force rate limit on `/auth/login` (by IP)

**Problem/decision:** prevent password brute-force without allowing an attacker to lock out a targeted account → key only by client IP.

**Approach:**
- **New `LoginLimiter`** reusing the existing `TokenBucket` (`rate_limit.rs`), backed by `DashMap<String, Mutex<TokenBucket>>` keyed by IP string, stored in `AppState`. Capability = `login_max_attempts_per_minute`, refill at capacity-per-minute (reuse `TokenBucket` semantics).

- **IP extraction** — forward-safe by default:
  - Add `trust_proxy: bool` (default `false`, `#[serde(default)]`).
  - Resolution order:
    1. If `trust_proxy` is true, take the first entry of `X-Forwarded-For`.
    2. Else, use the `ConnectInfo<SocketAddr>` extension if present (real peer address when the app is served by `axum::serve`).
    3. Else (e.g. in `oneshot` tests without `ConnectInfo`), fall back to a sentinel default key so the limiter is still exercised.
  - Because `oneshot` tests have no TCP peer, tests enable `trust_proxy: true` and set `X-Forwarded-For` to control the bucket key.

- **When to debit:** debit the bucket **only on failed login** (bad password or unknown user). Successful logins do not consume tokens, so legitimate users are never throttled by the limiter.

- **Plumbing:** add `login_max_attempts_per_minute: i64` (default `10`; `<=0` disables) and `trust_proxy: bool` (default `false`) to `AuthConfig`. Expose `ConnectInfo` by wiring `axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())` in `main.rs`. The `login` handler extracts IP, checks+debits the limiter on failure, returns `ApiError::RateLimited(Some(retry_after))` when exhausted.

### 3. Per-user session revocation (self-service)

**Problem:** a user should be able to revoke all their sessions (all refresh tokens), e.g. after suspected compromise.

**Approach:**
- **Repo:** add `RefreshTokenRepository::delete_all_for_user(user_id: Uuid) -> Result<u64, PasteurError>` running `DELETE FROM refresh_tokens WHERE user_id = $1` and returning the count.
- **Endpoint:** `POST /api/v1/auth/sessions/revoke-all` under the **protected** router (behind `jwt_auth`, beside `/auth/me`). It:
  - reads the current user id from `Extension<Claims>.sub`,
  - calls `delete_all_for_user`,
  - returns `{ "revoked": <n> }` **plus the clear-cookie headers** for both `godwit_access` and `godwit_refresh` (so the calling device is also signed out locally — consistent with the existing `logout` handler).
- **No** `user_id` in the request body → no privilege escalation; the action scopes to the authenticated user.
- **CSRF:** because it is under the protected router and is a state-changing POST, the existing `jwt_auth` origin-check already applies.

### 4. Production config hardening + E2E tests

**Config** (`AuthConfig`, all `#[serde(default)]`, backward compatible):
- `login_max_attempts_per_minute: i64` (default `10`)
- `trust_proxy: bool` (default `false`)
- existing `cookie_secure: bool` (default `false`), `allowed_cookie_origin: String` (default `""`)

**`config.example.yaml`:** document every field with safe dev defaults and inline comments (cookie_secure false over HTTP, allowed_cookie_origin empty for same-origin, login_max_attempts_per_minute 10, trust_proxy false).

**Boot-time validation (warning, not hard fail):** when `cookie_secure: true` and `allowed_cookie_origin` is empty, log a warning (same-origin rewrite deployments are valid without `allowed_cookie_origin`, so this is informational only). No hard failure to avoid breaking the existing same-origin docker deployment.

**E2E tests** in `crates/godwit-api/tests/router_integration.rs` (pattern: assemble the real router via `build_app` + `oneshot`, mutate config via a helper like the existing CSRF tests):
- CSRF: with `allowed_cookie_origin` set, `POST /auth/refresh` and `/auth/logout` with missing/wrong `Origin` → 403, **and** no token rotation / no row deletion; correct `Origin` → 200.
- Rate limit: 10 failed logins → 11th returns 429 + `RETRY_AFTER`; successful login does not consume; `trust_proxy: true` + `X-Forwarded-For` controls the key.
- Revocation: two devices logged in (two refresh tokens) → `revoke-all` → both `/auth/refresh` calls fail; endpoint returns clear-cookie headers for the caller.
- Regression: default dev config (origin empty, trust_proxy false) → login/refresh/logout behave exactly as before (no CSRF, no 429).
- Unit: `delete_all_for_user` repo test (delete + cascade), `origin_allowed` helper tests, `LoginLimiter` bucket tests.

## Data flow

```
login:  POST /auth/login ──► (resolve IP) ──► LoginLimiter.check ──► verify password
                                 │  fail: debit, if exhausted → 429
                                 ▼  success: issue token pair (cookies)

refresh: POST /auth/refresh ──► cookie_csrf (origin check) ──► rotate (delete old, issue new)

logout:  POST /auth/logout ──► cookie_csrf (origin check) ──► delete_by_hash + clear cookies

revoke-all: POST /api/v1/auth/sessions/revoke-all ──► jwt_auth (token + origin) ──► delete_all_for_user + clear cookies
```

## Error handling

- CSRF failure → `403 Forbidden` (no body consumption).
- Rate limit → `429` + `RETRY_AFTER` via existing `ApiError::RateLimited(Some(seconds))`.
- Revoke-all with invalid `sub` → `401 Unauthorized` (same safe path as `me`).

## Testing strategy

Unit + router-integration tests as detailed in track 4. All new tests follow the existing patterns (`#[sqlx::test]` with live DB, `tower::ServiceExt::oneshot` on the real router). The 10 pre-existing unrelated DB failures (rate_limit/circuit_breaker/spend_tags) are out of scope and must not be chased.

## Out of scope

- Rotation-family replay detection.
- Any UI work (login layout, settings page, OIDC/SAML buttons) — Bloc 2.
- Dependency upgrades (axum 0.8, sqlx 0.9, etc.) — Bloc 3.
- Fixing the 10 pre-existing unrelated DB test failures.
