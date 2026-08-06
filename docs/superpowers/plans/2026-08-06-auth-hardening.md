# Auth Backend Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the Godwit backend auth: add CSRF protection to cookie-authenticated `/auth/refresh` and `/auth/logout`, IP-keyed brute-force rate limiting on `/auth/login`, per-user session revocation, and config hardening with E2E tests.

**Architecture:** Extends the existing axum auth surface. Factors the origin check out of `jwt_auth` into a reusable helper behind a new `cookie_csrf` middleware for the two cookie-auth routes. Adds a `LoginLimiter` (reusing `TokenBucket`) keyed by IP with forward-safe extraction (`trust_proxy`). Adds `delete_all_for_user` to the refresh-token repo and a self-service revoke endpoint under the protected router. All changes are backward-compatible via `#[serde(default)]` fields.

**Tech Stack:** Rust, axum 0.7, sqlx 0.7, Postgres 15, tokio. Tests via `#[sqlx::test]` + `tower::ServiceExt::oneshot` on the real router.

## Global Constraints

- `cargo`/`rustup` path: **always** `export PATH="/usr/local/opt/rustup/bin:$PATH"` before any cargo command. Do **not** use `~/.cargo/env`.
- DB tests need `DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit`.
- 10 pre-existing DB test failures (rate_limit/circuit_breaker/spend_tags) are **unrelated** and must not be chased; a task's tests passing is judged against auth-specific tests, not the whole suite.
- All new `AuthConfig` fields must carry `#[serde(default)]` and be backward-compatible (old config files parse unchanged).
- No changes to Rust production code outside `godwit-api`, `godwit-core` (AuthConfig), `godwit-db` (repo), and `godwit-bin` (main.rs wiring).
- `docs/` is git-ignored; specs/plans added with `git add -f`.
- No comments unless necessary.

---

### Task 1: `AuthConfig` new fields + `LoginLimiter` module

**Files:**
- Modify: `crates/godwit-core/src/lib.rs:215-225` (AuthConfig)
- Create: `crates/godwit-api/src/login_rate_limit.rs`
- Modify: `crates/godwit-api/src/lib.rs` (declare module)
- Modify: `crates/godwit-api/src/state.rs` (add `login_limiter` field)

**Interfaces:**
- Consumes: `TokenBucket` from `crates/godwit-api/src/rate_limit.rs` (public struct with `deficit_retry_after(&mut self, now, amount) -> Option<u64>`, `debit(&mut self, amount)`).
- Produces: `AuthConfig.login_max_attempts_per_minute: i64` (default 10), `AuthConfig.trust_proxy: bool` (default false); `LoginLimiter::new(capacity: u32) -> Self`, `LoginLimiter::attempt_allowed(&self, ip: &str, debit_on_fail: bool) -> Option<u64>` (returns `Some(retry_after)` if blocked, `None` if allowed after debit); `AppState.login_limiter: LoginLimiter`.

- [ ] **Step 1: Write the failing unit test + final `LoginLimiter` implementation**

Create `crates/godwit-api/src/login_rate_limit.rs`. This is the **complete final** module (struct + implementation + tests). Add the public `capacity()` accessor to `TokenBucket` first (Step 2 defines `module` wiring and `TokenBucket::capacity` must exist — the accessor is added in this task's Step 2, and the module below compiles once both exist; the tests are the fixture):

```rust
use dashmap::DashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::rate_limit::TokenBucket;

pub struct LoginLimiter {
    capacity: u32,
    buckets: DashMap<String, Mutex<TokenBucket>>,
}

impl LoginLimiter {
    pub fn new(capacity: u32) -> Self {
        let capacity = if capacity > 0 { capacity } else { 0 };
        Self { capacity, buckets: DashMap::new() }
    }

    pub fn attempt_allowed(&self, ip: &str, debit_on_fail: bool) -> Option<u64> {
        if self.capacity == 0 {
            return None; // disabled
        }
        let entry = self.buckets.entry(ip.to_string()).or_insert_with(|| {
            Mutex::new(TokenBucket::new(self.capacity))
        });
        let mut bucket = entry.value().lock().expect("login limiter bucket poisoned");
        let retry_after = bucket.deficit_retry_after(Instant::now(), 1);
        if retry_after.is_none() && debit_on_fail {
            bucket.debit(1);
        }
        retry_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_capacity_then_blocks() {
        let limiter = LoginLimiter::new(2);
        assert!(limiter.attempt_allowed("1.2.3.4", true).is_none());
        assert!(limiter.attempt_allowed("1.2.3.4", true).is_none());
        let retry = limiter.attempt_allowed("1.2.3.4", true);
        assert!(retry.is_some(), "expected rate limited");
    }

    #[test]
    fn separate_ip_has_own_bucket() {
        let limiter = LoginLimiter::new(1);
        assert!(limiter.attempt_allowed("1.1.1.1", true).is_none());
        assert!(limiter.attempt_allowed("1.1.1.1", true).is_some());
        assert!(limiter.attempt_allowed("2.2.2.2", true).is_none());
    }

    #[test]
    fn non_debit_check_does_not_consume() {
        let limiter = LoginLimiter::new(1);
        assert!(limiter.attempt_allowed("5.5.5.5", false).is_none());
        assert!(limiter.attempt_allowed("5.5.5.5", false).is_none());
    }

    #[test]
    fn zero_capacity_disables() {
        let limiter = LoginLimiter::new(0);
        for _ in 0..10 {
            assert!(limiter.attempt_allowed("9.9.9.9", true).is_none());
        }
    }
}
```

- [ ] **Step 2: Add the `capacity()` accessor to `TokenBucket` + wire the module**

In `crates/godwit-api/src/rate_limit.rs`, `TokenBucket` has a private `capacity: u32` field. Add a public accessor:

```rust
impl TokenBucket {
    /// Exposes the configured capacity (0 = unlimited/disabled).
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    // ... existing methods unchanged
}
```

Declare `pub mod login_rate_limit;` in `crates/godwit-api/src/lib.rs` (add among the existing module declarations near the top).

- [ ] **Step 3: Run the new tests, expect failure (module not yet in AppState / state.rs)**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo check -p godwit-api 2>&1 | grep -i "login_rate_limit\|AppState\|login_limiter" | head`
Expected: FAIL / compiler errors referencing `login_limiter` field absent from `AppState`. (Alternatively the module may not be referenced yet and compiles standalone — in that case running `cargo test -p godwit-api login_rate_limit` should PASS its unit tests. The intended red state is that `AppState` lacks `login_limiter` once we wire it in Step 4; verify Step 4's failing first.)

- [ ] **Step 4: Wire `AppState`**

Add `login_limiter: LoginLimiter` to `AppState` in `crates/godwit-api/src/state.rs`:

```rust
use crate::login_rate_limit::LoginLimiter;
// in struct AppState { ... }
pub login_limiter: LoginLimiter,
```

Initialize it in every `AppState` construction site (grep `AppState {` — `crates/godwit-api/tests/router_integration.rs::build_app`, `crates/godwit-api/src/admin/auth.rs::tests::test_state`, `crates/godwit-api/src/middleware.rs::auth_tests::test_state`). Add to each:

```rust
login_limiter: LoginLimiter::new(10),
```

- [ ] **Step 5: Run the new unit tests, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo test -p godwit-api login_rate_limit`
Expected: all `login_rate_limit` tests PASS.

- [ ] **Step 6: Add `AuthConfig` fields**

In `crates/godwit-core/src/lib.rs::AuthConfig` (lines 215-225), add:

```rust
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    #[serde(default)]
    pub cookie_secure: bool,
    #[serde(default)]
    pub allowed_cookie_origin: String,
    #[serde(default = "default_login_max_attempts")]
    pub login_max_attempts_per_minute: i64,
    #[serde(default)]
    pub trust_proxy: bool,
    pub oidc_providers: Vec<OidcProviderConfig>,
    pub saml_providers: Vec<SamlProviderConfig>,
}
```

Add a module-level default fn near `AuthConfig`:

```rust
fn default_login_max_attempts() -> i64 {
    10
}
```

- [ ] **Step 7: Update the two test config constructors**

Both `crates/godwit-api/tests/router_integration.rs::test_config()` and `crates/godwit-api/src/admin/auth.rs::tests::test_state()` (and `crates/godwit-api/src/middleware.rs::tests/` / `auth_tests::test_state`) construct `AuthConfig` literally and will now fail to compile. Add the two new fields to each literal:

```rust
login_max_attempts_per_minute: 10,
trust_proxy: false,
```

Find all `AuthConfig {` literals with `grep -rn "AuthConfig {" crates/` and add both fields to every one.

- [ ] **Step 8: Run `cargo check -p godwit-api`**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo check -p godwit-api`
Expected: compiles (only pre-existing warnings).

- [ ] **Step 9: Commit**

```bash
git add crates/godwit-core/src/lib.rs crates/godwit-api/src/login_rate_limit.rs crates/godwit-api/src/lib.rs crates/godwit-api/src/state.rs crates/godwit-api/src/rate_limit.rs crates/godwit-api/src/admin/auth.rs crates/godwit-api/tests/router_integration.rs crates/godwit-api/src/middleware.rs
git commit -m "feat(auth): LoginLimiter per-IP bucket + AuthConfig rate-limit/trust_proxy fields"
```

---

### Task 2: IP extraction + brute-force rate limit on `/auth/login`

**Files:**
- Modify: `crates/godwit-bin/src/main.rs:148` (ConnectInfo wiring)
- Modify: `crates/godwit-api/src/admin/auth.rs` (login handler + IP helper)
- Modify: `crates/godwit-api/src/state.rs` (already has `login_limiter` from Task 1)
- Modify: `crates/godwit-api/tests/router_integration.rs` (test)

**Interfaces:**
- Consumes: `AppState.login_limiter: LoginLimiter`, `AppState.config.auth.trust_proxy: bool`, `AppState.config.auth.login_max_attempts_per_minute: i64`, `LoginLimiter::attempt_allowed(&self, ip: &str, debit_on_fail: bool) -> Option<u64>`.
- Produces: `fn client_ip(headers: &HeaderMap, connect_info: Option<SocketAddr>, trust_proxy: bool) -> String` (in `auth.rs` or `middleware.rs`, `pub` for tests).

- [ ] **Step 1: Write the failing integration test**

Add to `crates/godwit-api/tests/router_integration.rs`. First add a helper to build an app with custom auth config (mirroring the existing `set_origin` pattern in `middleware.rs`):

```rust
fn build_app_with_auth(pool: PgPool, mut auth: AuthConfig) -> Router {
    // Clone of build_app, but injects `auth` into config.
    // Duplicate the body of build_app; the only difference: config uses this auth.
    // (Task 1 added login_max_attempts_per_minute/trust_proxy to AuthConfig.)
    todo!()
}
```

Replace the `todo!()` with the real duplicated `build_app` body (see the existing `build_app` in the file, lines ~96-136). Set `auth` into `config.auth`.

Then the test:

```rust
#[sqlx::test]
async fn login_rate_limits_after_repeated_failures(pool: PgPool) {
    let auth = AuthConfig {
        jwt_secret: JWT_SECRET.to_string(),
        access_token_ttl_minutes: 15,
        refresh_token_ttl_days: 7,
        cookie_secure: false,
        allowed_cookie_origin: "".to_string(),
        login_max_attempts_per_minute: 2,
        trust_proxy: true,
        oidc_providers: vec![],
        saml_providers: vec![],
    };
    let app = build_app_with_auth(pool.clone(), auth);
    let seed_user_org(pool.clone()).await; // create an org + user with known credentials

    // 2 failed logins allowed
    for _ in 0..2 {
        let resp = oneshot_login(&app, "1.1.1.1", "wrong-email@example.com", "bad").await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
    // 3rd is rate limited
    let resp = oneshot_login(&app, "1.1.1.1", "wrong-email@example.com", "bad").await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    // Different IP unaffected
    let resp = oneshot_login(&app, "2.2.2.2", "wrong-email@example.com", "bad").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

Add helpers to the test file:

```rust
async fn oneshot_login(app: &Router, ip: &str, email: &str, password: &str) -> axum::response::Response {
    use axum::http::header::FORWARDED;
    let body = axum::body::Body::from(serde_json::json!({ "email": email, "password": password }).to_string());
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", ip)
        .body(body)
        .unwrap();
    app.clone().oneshot(req).await.unwrap()
}
```

> Note: the test routes `/api/v1/auth/login` (the admin router is nested at `/api/v1`). Verify the actual route against `admin::router()` usage — `login` is `POST /api/v1/auth/login`. Adjust `uri` if the nesting differs.

- [ ] **Step 2: Run it to confirm FAIL**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration login_rate_limits`
Expected: FAIL (login handler does not rate limit yet; 3rd request still returns UNAUTHORIZED, not 429).

- [ ] **Step 3: Implement IP extraction + rate limit in the login handler**

In `crates/godwit-api/src/admin/auth.rs`, add an IP helper:

```rust
use axum::extract::ConnectInfo;
use axum::http::header::HeaderMap;

/// Resolve the client IP for login rate limiting. When `trust_proxy` is set, reads
/// the first entry of `X-Forwarded-For`; otherwise the real peer address from
/// `ConnectInfo`; falls back to a sentinel for environments without either.
pub fn client_ip(
    headers: &HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    trust_proxy: bool,
) -> String {
    if trust_proxy {
        if let Some(xff) = headers.get(axum::http::header::FORWARDED).or_else(|| {
            headers.get("x-forwarded-for")
        }) {
            let v = xff.to_str().unwrap_or("").trim();
            if let Some(first) = v.split(',').next() {
                return first.trim().to_string();
            }
        }
    }
    if let Some(ConnectInfo(addr)) = connect_info {
        return addr.ip().to_string();
    }
    "unknown".to_string()
}
```

Refactor the `login` handler signature to accept `Headers` and an optional `ConnectInfo`, then rate limit:

```rust
async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let ip = client_ip(&headers, connect_info, state.config.auth.trust_proxy);
    let user = match state.user_repo.get_by_email(&req.email).await {
        Ok(u) => u,
        Err(_) => {
            // Unknown user counts as a failed attempt
            if let Some(retry_after) =
                state.login_limiter.attempt_allowed(&ip, true)
            {
                return Err(crate::error::ApiError::RateLimited(Some(retry_after)));
            }
            return Err(crate::error::ApiError::Unauthorized);
        }
    };
    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or(crate::error::ApiError::Unauthorized)?;
    if !verify_password(&req.password, password_hash) {
        if let Some(retry_after) = state.login_limiter.attempt_allowed(&ip, true) {
            return Err(crate::error::ApiError::RateLimited(Some(retry_after)));
        }
        return Err(crate::error::ApiError::Unauthorized);
    }
    let (set_cookie_headers, body) = issue_token_pair(&state, &user).await?;
    Ok((set_cookie_headers, body))
}
```

> Note: there is a correctness subtlety — the `attempt_allowed(ip, true)` call both checks AND debits. On a *blocked* attempt the bucket is exhausted so no further debit occurs (returns Some without consuming additional beyond the existing exhaustion). On the 3rd attempt the bucket has 0 tokens, `deficit_retry_after` returns the deficit without debiting (in `TokenBucket` the tentative `deficit_retry_after` never debits — see `rate_limit.rs:40-54`). So the debit-on-fail path is correct: attempts 1 and 2 debit; attempt 3 checks and is blocked without extra debit.

- [ ] **Step 4: Wire `ConnectInfo` in `main.rs`**

In `crates/godwit-bin/src/main.rs:148`, change:

```rust
    axum::serve(listener, app).await?;
```

to:

```rust
    use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
    use axum::extract::ConnectInfo;
    use std::net::SocketAddr;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
```

- [ ] **Step 5: Initialize `login_limiter` with configured capacity at router build**

In `crates/godwit-api/src/admin/mod.rs` (or `auth.rs`), when building the router state, ensure `login_limiter` gets the configured capacity. In `main.rs`/`build_app`, construct:

```rust
let login_limiter = LoginLimiter::new(config.auth.login_max_attempts_per_minute.max(0) as u32);
```

and put it in `AppState`. If `login_max_attempts_per_minute <= 0`, capacity is 0 → limiter disabled (no-op).

- [ ] **Step 6: Run tests, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration login_rate_limits`
Expected: PASS.

- [ ] **Step 7: Run existing auth tests for regression**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib admin::auth`
Expected: PASS (login still works; `login_max_attempts_per_minute: 10` default doesn't trigger).

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-bin/src/main.rs crates/godwit-api/src/admin/auth.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(auth): IP rate-limit login brute-force failures"
```

---

### Task 3: CSRF on `/auth/refresh` and `/auth/logout`

**Files:**
- Modify: `crates/godwit-api/src/middleware.rs` (factor origin check)
- Modify: `crates/godwit-api/src/admin/auth.rs` (sub-router + cookie_csrf)
- Modify: `crates/godwit-api/tests/router_integration.rs` (tests)
- Modify: `crates/godwit-api/src/admin/mod.rs` (if router restructuring requires)

**Interfaces:**
- Consumes: `AppState.config.auth.allowed_cookie_origin: String`, `axum::middleware::from_fn_with_state`.
- Produces: `pub fn origin_allowed(state: &AppState, method: &Method, headers: &HeaderMap) -> bool` in `middleware.rs`; `cookie_csrf` middleware used via `from_fn_with_state` on the refresh/logout sub-router.

- [ ] **Step 1: Write failing integration test**

Add to `crates/godwit-api/tests/router_integration.rs`:

```rust
#[sqlx::test]
async fn csrf_blocks_refresh_without_matching_origin(pool: PgPool) {
    let auth = AuthConfig { allowed_cookie_origin: "https://app.example.com".to_string(), ..base_auth() };
    let app = build_app_with_auth(pool.clone(), auth);
    // Perform a login to obtain a refresh cookie.
    let login_resp = oneshot_login(&app, "10.0.0.1", "user@example.com", "password").await;
    assert_eq!(login_resp.status(), StatusCode::OK);
    let cookie = login_resp.headers().get(SET_COOKIE).unwrap().to_str().unwrap().to_string();

    // Refresh with a WRONG origin must be 403 and NOT rotate.
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header(COOKIE, cookie.clone())
        .header("origin", "https://evil.example.com")
        .body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Refresh with NO origin must be 403.
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header(COOKIE, cookie.clone())
        .body(Body::empty()).unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::FORBIDDEN);

    // Correct origin passes.
    let req3 = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header(COOKIE, cookie.clone())
        .header("origin", "https://app.example.com")
        .body(Body::empty()).unwrap();
    let resp3 = app.clone().oneshot(req3).await.unwrap();
    assert_eq!(resp3.status(), StatusCode::OK);
}
```

Add a `base_auth()` helper that returns `AuthConfig` with defaults, so tests can use `..base_auth()` and override fields:

```rust
fn base_auth() -> AuthConfig {
    AuthConfig {
        jwt_secret: JWT_SECRET.to_string(),
        access_token_ttl_minutes: 15,
        refresh_token_ttl_days: 7,
        cookie_secure: false,
        allowed_cookie_origin: "".to_string(),
        login_max_attempts_per_minute: 10,
        trust_proxy: false,
        oidc_providers: vec![],
        saml_providers: vec![],
    }
}
```

Refactor `test_config()` to use `base_auth()`. Also add a `login_refresh_cookie` helper that performs login and returns the `godwit_refresh` cookie value (`integration_test` already has such a round-trip — reuse it; see `refresh_and_logout_work_from_refresh_cookie_alone_without_body`).

- [ ] **Step 2: Run it to confirm FAIL**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration csrf_blocks_refresh`
Expected: FAIL — refresh succeeds with wrong origin today (200, not 403), because no CSRF on these routes.

- [ ] **Step 3: Factor `origin_allowed` in `middleware.rs`**

Replace the inline origin logic in `jwt_auth` (middleware.rs:88-101) with a call to the new helper:

```rust
use axum::http::Method;

/// True when a state-changing request carries an `Origin` matching `allowed_cookie_origin`,
/// or when `allowed_cookie_origin` is empty (check disabled). No-op for non-state-changing methods.
pub fn origin_allowed(state: &AppState, method: &Method, headers: &HeaderMap) -> bool {
    let allowed_origin = state.config.auth.allowed_cookie_origin.as_str();
    if allowed_origin.is_empty() || !is_state_changing(method) {
        return true;
    }
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .map(|origin| origin == allowed_origin)
        .unwrap_or(false)
}
```

Update `jwt_auth`:

```rust
pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !origin_allowed(&state, req.method(), req.headers()) {
        return Err(StatusCode::FORBIDDEN);
    }
    // ... token extraction unchanged ...
}
```

- [ ] **Step 4: Add `cookie_csrf` middleware**

In `middleware.rs`, add a new middleware (no token check):

```rust
pub async fn cookie_csrf(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !origin_allowed(&state, req.method(), req.headers()) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}
```

Note: `Next` middleware receives `Request<Body>`; keep signatures consistent with `jwt_auth` (uses `Request`). If `Next` requires `Request<Body>`, mirror `jwt_auth`'s `mut req: Request` type.

- [ ] **Step 5: Apply `cookie_csrf` to refresh/logout sub-router**

In `crates/godwit-api/src/admin/auth.rs::router()`:

```rust
use axum::middleware::from_fn_with_state;
use crate::middleware::cookie_csrf;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Cookie-authenticated routes: apply CSRF origin check (no token check).
    let cookie_routes = Router::new()
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route_layer(from_fn_with_state(state.clone(), cookie_csrf));

    Router::new()
        .merge(cookie_routes)
        .route("/auth/login", post(login))
        .route("/auth/oidc/:provider", get(oidc_start))
        .route("/auth/oidc/:provider/callback", get(oidc_callback))
        .route("/auth/saml/:provider/acs", post(saml_acs))
}
```

> Note: `admin::router(state)` calls `auth::router(state)` (see `admin/mod.rs` line ~56: `Router::new().merge(auth::router()).merge(protected)`). So `auth::router` must now take `state: Arc<AppState>` instead of `()`. Update the call site in `admin/mod.rs`.

- [ ] **Step 6: Update `auth::router` call site signature in `admin/mod.rs`**

```rust
Router::new().merge(auth::router(state.clone())).merge(protected)
```

(The `protected` router already receives `state` for `jwt_auth`; pass a clone to `auth::router` too.)

- [ ] **Step 7: Run tests, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration csrf_blocks_refresh`
Expected: PASS.

- [ ] **Step 8: Run existing auth integration tests for regression**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration`
Expected: pre-existing auth ROUND-trip + cookie tests still pass (dev origin empty → no-op). The 10 unrelated failures may still appear.

- [ ] **Step 9: Commit**

```bash
git add crates/godwit-api/src/middleware.rs crates/godwit-api/src/admin/auth.rs crates/godwit-api/src/admin/mod.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(auth): CSRF origin check on cookie-auth refresh/logout"
```

---

### Task 4: Per-user session revocation — repo + endpoint

**Files:**
- Modify: `crates/godwit-db/src/repositories/refresh_tokens.rs` (repo method + test)
- Modify: `crates/godwit-api/src/admin/mod.rs` (register route)
- Modify: `crates/godwit-api/src/admin/auth.rs` (handler)
- Modify: `crates/godwit-api/tests/router_integration.rs` (test)

**Interfaces:**
- Consumes: `RefreshTokenRepository::delete_by_hash`, `hash_refresh_token`, `Extension<Claims>` pattern from `me`, `SET_COOKIE` clear-cookie helper from `logout`.
- Produces: `RefreshTokenRepository::delete_all_for_user(user_id: Uuid) -> Result<u64, PasteurError>`; `POST /api/v1/auth/sessions/revoke-all` handler returning `(HeaderMap, Json<{revoked: n}>)`.

- [ ] **Step 1: Write failing repo test**

In `crates/godwit-db/src/repositories/refresh_tokens.rs` tests, add:

```rust
#[sqlx::test]
async fn delete_all_for_user_removes_only_that_user(pool: PgPool) {
    use crate::repositories::refresh_tokens::RefreshTokenRepository;
    let users = UserRepository::new(pool.clone());
    let user_a = users.create("aaa@example.com", None, UserRole::User, None)
        .await.expect("create user a");
    let user_b = users.create("bbb@example.com", None, UserRole::User, None)
        .await.expect("create user b");

    let repo = RefreshTokenRepository::new(pool.clone());
    let exp = chrono::Utc::now() + chrono::Duration::days(7);
    repo.create(user_a.id, "hash-a1", exp).await.expect("a1");
    repo.create(user_a.id, "hash-a2", exp).await.expect("a2");
    repo.create(user_b.id, "hash-b1", exp).await.expect("b1");

    let n = repo.delete_all_for_user(user_a.id).await.expect("delete all a");
    assert_eq!(n, 2);

    assert!(repo.get_by_hash("hash-a1").await.is_err());
    assert!(repo.get_by_hash("hash-a2").await.is_err());
    // user B unaffected
    assert!(repo.get_by_hash("hash-b1").await.is_ok());
}
```

- [ ] **Step 2: Run it, expect FAIL**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-db delete_all_for_user`
Expected: FAIL (method does not exist).

- [ ] **Step 3: Implement the repo method**

In `crates/godwit-db/src/repositories/refresh_tokens.rs`, add:

```rust
pub async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, PasteurError> {
    let res = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))?;
    Ok(res.rows_affected())
}
```

- [ ] **Step 4: Run repo test, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-db delete_all_for_user`
Expected: PASS.

- [ ] **Step 5: Write failing integration test for the endpoint**

Add to `crates/godwit-api/tests/router_integration.rs`:

```rust
#[sqlx::test]
async fn revoke_all_signs_out_all_devices(pool: PgPool) {
    let app = build_app(pool.clone());
    // Login from two IPs -> two refresh tokens
    let r1 = oneshot_login(&app, "10.0.0.1", "user@example.com", "password").await;
    let r2 = oneshot_login(&app, "10.0.0.2", "user@example.com", "password").await;
    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);
    let cookie1 = r1.headers().get(SET_COOKIE).unwrap().to_str().unwrap().to_string();

    // Call revoke-all using an access-token-bearing request (via /auth/me pattern).
    // Need an access token: derive from a login. Build an auth header.
    let token = extract_access_token(&r1); // helper: parse godwit_access cookie or body access_token
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/sessions/revoke-all")
        .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // Response clears cookies
    assert!(resp.headers().contains_key(SET_COOKIE));

    // Refresh with cookie1 must now fail.
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header(COOKIE, cookie1)
        .body(Body::empty()).unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);
}
```

Add helper `extract_access_token(resp)` that reads the `access_token` from the login JSON body (the `issue_token_pair` returns `{access_token, refresh_token}` in the body — use that, it's simpler than parsing the cookie).

> Note: `oneshot_login` (defined in Task 2) returns a `Response`; adapt it to also allow reading the body for `extract_access_token`. If the body must be consumed, split the helper or read `r1` body bytes.

- [ ] **Step 6: Run it, expect FAIL**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration revoke_all`
Expected: FAIL (404 — route does not exist).

- [ ] **Step 7: Implement the handler + register route**

In `crates/godwit-api/src/admin/auth.rs`, add a handler (mirrors `me`):

```rust
pub async fn revoke_all_sessions(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .ok()
        .ok_or(crate::error::ApiError::Unauthorized)?;
    let revoked = state
        .refresh_token_repo
        .delete_all_for_user(user_id)
        .await
        .map_err(crate::error::ApiError::Core)?;

    let mut headers = HeaderMap::new();
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str("godwit_access=; HttpOnly; Path=/; Max-Age=0")
            .map_err(|_| crate::error::ApiError::Internal)?,
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str("godwit_refresh=; HttpOnly; Path=/api/v1/auth; Max-Age=0")
            .map_err(|_| crate::error::ApiError::Internal)?,
    );
    Ok((headers, Json(serde_json::json!({ "revoked": revoked }))))
}
```

In `crates/godwit-api/src/admin/mod.rs`, register under the protected router (beside `/auth/me`):

```rust
.route("/auth/sessions/revoke-all", axum::routing::post(auth::revoke_all_sessions))
```

Add it inside the `protected` `Router::new()` construction, alongside `.route("/auth/me", ...)`.

- [ ] **Step 8: Run tests, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration revoke_all`
Expected: PASS.

- [ ] **Step 9: Run `cargo check -p godwit-api` + `cargo check -p godwit-db`**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo check -p godwit-api && cargo check -p godwit-db`
Expected: both compile (pre-existing warnings only).

- [ ] **Step 10: Commit**

```bash
git add crates/godwit-db/src/repositories/refresh_tokens.rs crates/godwit-api/src/admin/mod.rs crates/godwit-api/src/admin/auth.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(auth): per-user session revocation endpoint"
```

---

### Task 5: Config example + boot-time validation warning

**Files:**
- Modify: `config.example.yaml` (auth block)
- Modify: `crates/godwit-bin/src/bootstrap.rs` or `main.rs` (validation warning — pick the location where config is loaded; `load_config` in `main.rs:152-157`)

**Interfaces:**
- Consumes: `AppConfig.auth` fields from Task 1.
- Produces: none (log-only).

- [ ] **Step 1: Update `config.example.yaml`**

Find the `auth:` block. Add/document the fields:

```yaml
auth:
  jwt_secret: "change-me"
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  # Mark Cookie attr Secure when served over HTTPS.
  cookie_secure: false
  # Empty for same-origin (UI-rewrite) deployments. Set to the UI origin to
  # require an Origin header match on cookie-authenticated POSTs.
  allowed_cookie_origin: ""
  # Max failed login attempts per client IP per minute (per-IP bucket). 0 = disabled.
  login_max_attempts_per_minute: 10
  # Read X-Forwarded-For for client IP when behind a trusted reverse proxy (e.g. docker).
  trust_proxy: false
  oidc_providers: []
  saml_providers: []
```

Preserve whatever the current `auth:` keys actually are — align the field names exactly with the `AuthConfig` fields.

- [ ] **Step 2: Add the boot-time warning**

In `crates/godwit-bin/src/main.rs` `load_config()`, after loading, log a warning when `cookie_secure && allowed_cookie_origin.is_empty()`:

```rust
fn load_config() -> anyhow::Result<AppConfig> {
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    let file = std::fs::File::open(&path)?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    if config.auth.cookie_secure && config.auth.allowed_cookie_origin.is_empty() {
        tracing::warn!(
            "auth: cookie_secure=true but allowed_cookie_origin empty; relying on same-origin rewrites (informational)"
        );
    }
    Ok(config)
}
```

- [ ] **Step 3: Verify it compiles**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo check -p godwit-bin`
Expected: compiles.

- [ ] **Step 4: Verify config.example.yaml parses**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cp config.example.yaml /tmp/config-check.yaml && CONFIG_PATH=/tmp/config-check.yaml cargo run --bin godwit 2>&1 | head -5 || true`
Expected: config loads (server may fail to bind but no serde parse error). Then remove `/tmp/config-check.yaml`.

- [ ] **Step 5: Commit**

```bash
git add config.example.yaml crates/godwit-bin/src/main.rs
git commit -m "docs(conf): auth hardening config example + boot-time warning"
```

---

### Task 6: Unit tests for the factored CSRF helper + login limiter

**Files:**
- Modify: `crates/godwit-api/src/middleware.rs` (tests for `origin_allowed`)
- Modify: `crates/godwit-api/src/login_rate_limit.rs` (tests already in Task 1; add disabled/zero-capacity check)

**Interfaces:**
- Consumes: `origin_allowed(state, method, headers)` from Task 3.
- Produces: none (test-only).

- [ ] **Step 1: Add unit tests for `origin_allowed`**

In `crates/godwit-api/src/middleware.rs` `mod tests` (or the existing `mod auth_tests`), add a non-DB-friendly test using the helper directly (avoid DB by constructing a minimal `AppState` — but `origin_allowed` only reads `config.auth.allowed_cookie_origin`, so a test can reuse a helper that builds state; if building full `AppState` is heavy, extract a small helper `fn origin_allowed_from_cfg(allowed: &str, method, headers)` that `origin_allowed` delegates to, making it unit-testable without the whole state):

```rust
fn origin_allowed_from_config(allowed: &str, method: &Method, headers: &HeaderMap) -> bool {
    if allowed.is_empty() || !is_state_changing(method) {
        return true;
    }
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .map(|origin| origin == allowed)
        .unwrap_or(false)
}
```

Refactor `origin_allowed` to call `origin_allowed_from_config(&state.config.auth.allowed_cookie_origin, method, headers)`. Then unit test `origin_allowed_from_config`:

```rust
#[test]
fn origin_check_empty_allowed_is_noop() {
    let headers = HeaderMap::new();
    assert!(origin_allowed_from_config("", &Method::POST, &headers));
}

#[test]
fn origin_check_requires_match_for_state_changing() {
    let mut headers = HeaderMap::new();
    assert!(!origin_allowed_from_config("https://app.example.com", &Method::POST, &headers)); // missing
    headers.insert(axum::http::header::ORIGIN, "https://evil.example.com".parse().unwrap());
    assert!(!origin_allowed_from_config("https://app.example.com", &Method::POST, &headers));
    headers.insert(axum::http::header::ORIGIN, "https://app.example.com".parse().unwrap());
    assert!(origin_allowed_from_config("https://app.example.com", &Method::POST, &headers));
}

#[test]
fn origin_check_ignores_gets() {
    let mut headers = HeaderMap::new();
    assert!(origin_allowed_from_config("https://app.example.com", &Method::GET, &headers)); // GET not state-changing
}
```

- [ ] **Step 2: Run unit tests, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo test -p godwit-api origin_allowed`
Expected: PASS.

- [ ] **Step 3: Add zero-capacity (disabled) test for LoginLimiter**

In `login_rate_limit.rs` tests:

```rust
#[test]
fn zero_capacity_disables() {
    let limiter = LoginLimiter::new(0);
    for _ in 0..10 {
        assert!(limiter.attempt_allowed("9.9.9.9", true).is_none());
    }
}
```

- [ ] **Step 4: Run, expect PASS**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit && cargo test -p godwit-api login_rate_limit`
Expected: PASS.

- [ ] **Step 5: Full auth test sweep**

Run:
```bash
export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit
cargo check --workspace
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-db --lib
```
Expected: auth-related tests pass; the 10 pre-existing unrelated failures (rate_limit/circuit_breaker/spend_tags) may appear and are acceptable.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/middleware.rs crates/godwit-api/src/login_rate_limit.rs
git commit -m "test(auth): unit coverage for CSRF helper and login limiter disabled path"
```

---

## End-of-plan verification (after all tasks)

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH" && cd /home/thomas/work/Godwit
cargo check --workspace
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test router_integration
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --lib
DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-db --lib
```
