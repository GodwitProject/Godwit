# Password Management — Design

## Status

Approved by user (all sections). Scope: full-stack password lifecycle for Godwit, in a single cycle (including SMTP email for "forgot password"). Template theming is explicitly **out of scope** this cycle (deferred to a global UI-wide topic).

## Goal

Provide fine-grained password management on top of the existing Argon2 password login:

1. **Self-service change password** (user knows current password).
2. **Admin reset password** (super/org admin forces a reset on a user in org).
3. **Forgot password** (self-service reset via email token) — requires building SMTP email infra from scratch.
4. **Password policy** (length + complexity), configurable via `config.yaml`, plus blocking of known-weak passwords via an embedded offline list.
5. **Password expiry** — "forced change at login" (non-blocking: user is authenticated but must pick a new password).
6. **First-login force** — admin-created/reset accounts get `must_change_password = true`.
7. **Password reuse prevention** — reject the last N passwords.

## Decisions captured

- Scope: everything in one cycle including email.
- Who: self-service + admin.
- Policy: length + complexity, configurable via `config.yaml`, + embedded local common-password list (no network).
- Expiry behavior: forced change at login (non-blocking).
- First login: `must_change_password = true`.
- Forgot-password: SMTP via `lettre`; returns 200 even on failure/absence of SMTP (anti-email-enumeration); token logged in dev.
- Email templates: kept simple (embedded `tera`, `reset_password` + `password_changed`); **no theme/logo customization this cycle** (global UI topic later). `username`/`password` SMTP fields are **optional**.
- New endpoints mounted under `/api/v1/auth`; login response gains a `must_change_password` flag.

## Architecture

### 1. Configuration (`AuthConfig`)

New fields under `auth` in `config.yaml` (all `#[serde(default)]`, backward compatible):

```yaml
auth:
  # ... existing (jwt_secret, ttl, cookie, oidc, saml, rate limit) ...
  mail:
    from: "Godwit <no-reply@example.com>"
    host: smtp.example.com
    port: 587
    username: ""          # optional
    password: ""          # optional
    tls: starttls         # none | starttls | tls
    app_url: "https://app.example.com"   # base for building reset links
  password_policy:
    min_length: 10
    require_upper: true
    require_lower: true
    require_digit: true
    require_symbol: true
    max_reuse: 5
    days_to_expire: 90    # 0 = never
    block_common: true    # enable embedded common-password list
```

- `MailConfig`: `from`, `host`, `port`, `username` (option/`Option<String>`), `password` (option, `SecretString`-style not required now but never logged), `tls`, `app_url`.
- `PasswordPolicy` struct with all fields; defaults provided so existing configs without the section keep working.

### 2. Data model (new migration)

`crates/godwit-db/migrations/20260809000003_password_mgmt.up.sql` (mirrors `refresh_tokens` pattern):

```sql
ALTER TABLE users
  ADD COLUMN password_changed_at TIMESTAMPTZ,
  ADD COLUMN password_expires_at TIMESTAMPTZ,
  ADD COLUMN must_change_password BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE password_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_password_history_user ON password_history(user_id, created_at);

CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at TIMESTAMPTZ
);
CREATE INDEX idx_password_reset_tokens_hash ON password_reset_tokens(token_hash);
```

Backfill (in the same migration): set `password_changed_at = NOW()`, `password_expires_at = NULL` (interpreted as "never expires"), `must_change_password = FALSE` for existing users so an upgrade does not force everyone to change immediately. Rationale: a SQL migration cannot read the YAML `days_to_expire`; `NULL = never` lets the policy take effect from the user's next password change onward. Down migration drops the columns/tables.

### 3. Logic layer (`crates/godwit-api/src/admin/password.rs`)

Pure, testable logic separated from handlers:

- `validate_password(policy, password, history: &[String]) -> Result<(), PolicyError>`
  - length, upper/lower/digit/symbol per policy
  - reject if in embedded common list (`block_common`)
  - reject if equal (re-hash compare) to any of the last `max_reuse` history hashes
- `apply_password_change(user, new_hash, policy, repos)`:
  - push hash to `password_history`; purge entries older than `max_reuse`
  - update `users.password_hash`, `password_changed_at = NOW()`, `password_expires_at = NOW() + days_to_expire`, `must_change_password = FALSE`
  - revoke all refresh tokens for the user (`delete_all_for_user`) — a password change signs out other sessions
- `PasswordState { Valid, Expired, ForcedChange }` — computed from `must_change_password` and `password_expires_at` (`NULL` = never expires). If `Expired`/`ForcedChange` → login response includes `must_change_password: true`.

Common-password list: embedded via `include_str!("common_passwords.txt")` (a top-N list, e.g. SecLists 10k-most-common), compiled in. Empty/disabled when `block_common` is false.

### 4. Repositories (`godwit-db`)

- `PasswordHistoryRepository`: `push`, `get_last_n(user_id, n)`, `purge_older_than(user_id, keep_n)`.
- `PasswordResetTokenRepository`: `create`, `get_by_hash`, `mark_used`, `delete_for_user`.
- `UserRepository`: add `update_password(user_id, hash, changed_at, expires_at, must_change)`.

### 5. Email layer (`crates/godwit-api/src/mail.rs`)

- Crate **`lettre`** (`AsyncSmtpTransport`, `no_tls`/`starttls`/`relay` per `tls` config). Credentials optional: build transport without `credentials()` when username/password absent.
- Templates: embedded `tera` templates (`include_str!`), two emails: `reset_password` (link) and `password_changed` (confirmation). Simple layout, no theme/logo customization this cycle.
- `trait SendEmail { async fn send(&self, msg: Message) -> Result<()> }` — implemented by real SMTP `Mailer` and by a test fake that captures messages (enables full-flow tests without an SMTP server).
- `AppState.mailer: Option<Arc<dyn SendEmail>>` — `None` when no `auth.mail`.
- Tolerance: `POST /auth/forgot-password` always returns 200. If mailer is `None` or send fails, log the failure; in dev (non-prod) log the reset token for local testing. Never log the token in prod.

### 6. Endpoints (`/api/v1/auth`)

| Method | Path | Auth | Body | Notes |
|---|---|---|---|---|
| POST | `/change-password` | JWT (self) | `{ current_password, new_password }` | verify current, policy, rotate; 401 if current wrong |
| POST | `/admin/reset-password` | JWT (super/org admin) | `{ user_id, new_password }` | same-org check + not-self + not-on-super-admin; set mdp + `must_change_password=true` + revoke sessions; forbidden on own account |
| POST | `/forgot-password` | public | `{ email }` | create token, send email; always 200 |
| POST | `/reset-password` | public (token) | `{ token, new_password }` | validate token (hash, expiry, unused), policy, rotate, mark used, revoke sessions |
| POST | `/change-required` | JWT (self, must_change) | `{ new_password }` | one-time: no current password needed when `must_change_password` is set |

Login modification: after password verify, compute `PasswordState`; append to JSON body:

```json
{ "access_token": "...", "refresh_token": "...", "must_change_password": true }
```

(flag only when `Expired`/`ForcedChange`).

Admin reset RLS reuse: follow existing `users.rs` `require_role` + `check_same_org` + `check_not_acting_on_super_admin` helpers. Routing: new endpoints registered from `password.rs` and merged into the existing auth router (outside the `protected` JWT layer for forgot/reset; under JWT for change/change-required/admin). Ratelimit forgot-password to prevent email-bombing (reuse `LoginLimiter`-style bucket keyed by IP/email).

### 7. Error handling

- Invalid current password → 401.
- Policy violation → 422/400 with a machine-readable reason (e.g. `{ "error": "policy", "reason": "TOO_SHORT"|"NEEDS_DIGIT"|"COMMON"|"REUSED"... }`).
- Invalid/expired/used reset token → 400/410 with a generic message (no token-origin leak).
- `forgot-password` → always 200 (on unknown email and on SMTP failure).

### 8. Contract & frontend

- Add routes to `contract/routes.json` (`auth.change-password`, `auth.admin.reset-password`, `auth.forgot-password`, `auth.reset-password`, `auth.change-required`) with scope + FE pointers; extend the `apps/ui` and `apps/admin` route-contract tests.
- **`apps/ui`** (new UI): pages `src/app/forgot-password/page.tsx`, `src/app/reset-password/page.tsx`, `src/app/change-password/page.tsx` (settings), and a "change-required" screen driven by `must_change_password` at login. New lib fns in `src/lib/auth.ts` (`forgotPassword`, `resetPassword`, `changePassword`, `changeRequired`) + hooks.
- **`apps/admin`** (legacy): same surfaces via server actions (`app/(auth)/forgot-password`, `app/(auth)/reset-password`, change-password in profile), and force-redirect on `must_change_password`.

### 9. Testing strategy

- Unit: `validate_password` (each policy rule, common-list, reuse), `apply_password_change` (history rotation, expire timestamp, session revocation), `PasswordState` computation.
- DB (repo): `password_history` push/purge, `password_reset_tokens` create/get/mark-used, `users.update_password`.
- Router integration (real router + `oneshot`, following `router_integration.rs` pattern with `seed_password_user`): full flows —
  - change-password success + wrong-current (401) + policy rejection
  - admin reset (same-org, not-self, role guards)
  - forgot → 200 + (fake mailer) token capture → reset-password → login with new + old rejected
  - reset token expiry/used rejection
  - login with expired/forced account → `must_change_password: true` → change-required path
  - password_hash never leaked (existing guard re-verified on new responses)
- Email: fake `SendEmail` captures rendered `Message`; assert reset link contains `app_url`/token and `password_changed` is delivered on rotation.

## Out of scope (this cycle)

- Email template theming / logo / brand customization (deferred — global UI topic).
- HIBP / network weak-password check (embedded offline list only).
- Hard lockout on expiry (forced change chosen instead).
- Rotation-family / replay-detection for refresh tokens.
- Dependency upgrades (axum 0.8, sqlx 0.9, etc.) — blocking note: `lettre` and `tera` are new deps; verify they compile with the current toolchain.
