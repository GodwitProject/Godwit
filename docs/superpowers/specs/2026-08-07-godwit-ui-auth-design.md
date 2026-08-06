# Godwit UI — JWT Cookie Auth Flow Design

**Date:** 2026-08-07
**Version:** 1.0.0
**Status:** Approved
**Author:** Godwit Team

---

## 1. Overview

The Godwit admin API requires JWT authentication (`Authorization: Bearer`), but the Next.js admin UI currently has no login flow — every admin request returns 401. This doc specifies a secure authentication flow using **httpOnly cookies**, automatic token refresh, and a dedicated auth module in the UI.

### 1.1 Decisions (confirmed with user)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Token storage | **httpOnly cookies** | Immune to XSS token theft |
| Expiry handling | **Auto-refresh + automatic retry** | Seamless UX, no forced logouts |
| Auth module | **Dedicated `lib/auth.ts`** | Clean separation, testable |
| Deployment topology | **Same-origin reverse proxy** | Simplifies cookies (SameSite) & CSRF |

---

## 2. Architecture

### 2.1 Same-origin topology (reverse proxy)

**Current (broken for cookies):**
```
Browser ──> NEXT_PUBLIC_API_URL=http://localhost:3000  (cross-origin :3000)
```

**Target (same-origin via Next.js rewrites):**
```
Browser ──> http://localhost:3001/api/v1/*   ──rewrite──>  http://localhost:3000/api/v1/*
Browser ──> http://localhost:3001/*          (UI pages, no rewrite)
```

All browser requests go to the **UI origin** (`:3001` in dev, `:3002` in docker). Next.js rewrites API paths to the backend. Because everything is same-origin:
- Cookies use `SameSite=Strict` / `SameSite=Lax` (no `Secure` needed in dev)
- No CORS layer required
- Much simpler CSRF posture

**`next.config.js` change:**
```js
const API_ORIGIN = process.env.NEXT_PUBLIC_API_ORIGIN || 'http://localhost:3000';

module.exports = {
  output: 'standalone',
  async rewrites() {
    return [
      { source: '/api/v1/:path*', destination: `${API_ORIGIN}/api/v1/:path*` },
      { source: '/health', destination: `${API_ORIGIN}/health` },
      { source: '/metrics', destination: `${API_ORIGIN}/metrics` },
      // v1/utils routes live at root (not under /api):
      { source: '/v1/utils/:path*', destination: `${API_ORIGIN}/v1/utils/:path*` },
    ];
  },
};
```

### 2.2 Cookie design

| Cookie | Value | httpOnly | SameSite | Path | Max-Age |
|--------|-------|----------|----------|------|---------|
| `godwit_access` | JWT access token | ✅ | `Strict` | `/` | access TTL (15 min) |
| `godwit_refresh` | refresh plaintext | ✅ | `Strict` | `/api/v1/auth` | refresh TTL (7 days) |

- `godwit_access` is sent with **every** admin request (path `/`).
- `godwit_refresh` is only sent to the `/api/v1/auth/*` endpoints (narrow path) → reduces exposure.
- Both `HttpOnly` → JS cannot read them.
- `Secure` flag only set in production (behind HTTPS); dev is plain HTTP.

### 2.3 Data flow

```
Login:
  1. UI POST /api/v1/auth/login {email, password}   (credentials: "include")
  2. Backend validates, sets godwit_access + godwit_refresh cookies, returns {user}
  3. UI stores user in auth state → navigate to /

Authenticated request:
  1. UI fetch('/api/v1/...', { credentials: 'include' })  → cookie auto-sent
  2. Backend jwt_auth reads godwit_access cookie → 200 or 401

Expired access (401):
  1. Fetch wrapper intercepts 401
  2. POST /api/v1/auth/refresh  (godwit_refresh cookie auto-sent)  → new pair, new cookies
  3. Retry original request once
  4. On refresh failure → logout → redirect /login

Logout:
  1. POST /api/v1/auth/logout  → backend clears both cookies
  2. UI clears auth state → /login
```

---

## 3. Backend Changes (Rust)

### 3.1 `issue_token_pair` — set cookies

`crates/godwit-api/src/admin/auth.rs`:
- Change `issue_token_pair` to also set two `Set-Cookie` headers on the response.
- Use `axum-extra`/`cookie` crate OR build `Set-Cookie` header strings manually (simplest, no new dep).

Login/refresh handlers return `(HeaderMap/Headers, Json)` so the `Set-Cookie` headers are included.

Manual `Set-Cookie` value format:
```
godwit_access=<jwt>; HttpOnly; Path=/; SameSite=Strict; Max-Age=900
godwit_refresh=<plaintext>; HttpOnly; Path=/api/v1/auth; SameSite=Strict; Max-Age=604800
```
Production adds `; Secure`.

### 3.2 `logout` — clear cookies

`POST /api/v1/auth/logout`:
- Keep the refresh_token deletion (single-use).
- Also emit `Set-Cookie` clearing both cookies: `godwit_access=; HttpOnly; Path=/; Max-Age=0` etc.

### 3.3 `jwt_auth` middleware — accept cookie

`crates/godwit-api/src/middleware.rs` `jwt_auth`:
- Try `Authorization: Bearer` first (backward compatible).
- If absent, read the `cookie` header, parse `godwit_access=<jwt>`.
- Extract token from cookie, `verify(...)`.
- 401 if neither present/valid.

### 3.4 CSRF mitigation

With `SameSite=Strict`/`Lax` cookies and the same-origin topology, the primary CSRF vectors are covered. Add a lightweight defense-in-depth:
- The admin API only accepts **state-changing** methods (POST/PUT/PATCH/DELETE). For these, `jwt_auth` (or a small check) verifies the request `Origin` header (if present) matches the configured allowed origin; reject cross-origin state changes that arrive with cookies.
- Config: `auth.allowed_cookie_origin` (default empty → skip origin check in dev).

### 3.5 New endpoint: `GET /api/v1/auth/me`

Returns the current authenticated user from the JWT claims (id, org, role, email) so the UI can show identity + guard routes.
- Reuses `jwt_auth` — reads cookie.
- Response: `{ "user": { "id", "email", "role", "organization_id" } }`.

### 3.6 Config additions (`AuthConfig`)

```rust
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    pub cookie_secure: bool,              // default false (dev), true in prod
    pub allowed_cookie_origin: String,    // default "" (CSRF origin check skipped)
}
```

---

## 4. Frontend Changes (Next.js)

### 4.1 `src/lib/auth.ts` — dedicated auth module

```ts
export interface AuthUser {
  id: string;
  email: string;
  role: string;
  organization_id: string | null;
}

export async function login(email: string, password: string): Promise<AuthUser>;
export async function logout(): Promise<void>;
export async function fetchMe(): Promise<AuthUser>;  // GET /auth/me
export function isAuthenticated(): boolean;  // reads auth store
```

- Uses `fetch(..., { credentials: 'include' })`.
- `login` calls `/api/v1/auth/login`.
- `logout` calls `/api/v1/auth/logout` then clears state.
- `fetchMe` calls `/api/v1/auth/me` for route guards / header identity.

### 4.2 Auth state store — `src/store/auth.ts` (Zustand)

```ts
interface AuthStore {
  user: AuthUser | null;
  status: 'unknown' | 'authenticated' | 'unauthenticated';
  setUser(user: AuthUser | null): void;
}
```

- Persist `user` (non-sensitive metadata only) in sessionStorage/localStorage for a snappy guard; **never** store tokens (they're httpOnly cookies).

### 4.3 Fetch wrapper with auto-refresh — `src/lib/api.ts` rework

Central fetch helper (currently scattered `fetch` calls):

```ts
let refreshPromise: Promise<boolean> | null = null;

async function apiFetch(path, init = {}) {
  const res = await fetch(`${API_BASE}${path}`, { ...init, credentials: 'include' });
  if (res.status !== 401) return res;

  // Attempt refresh once (dedup concurrent refreshes)
  if (!refreshPromise) {
    refreshPromise = doRefresh().finally(() => { refreshPromise = null; });
  }
  const ok = await refreshPromise;
  if (!ok) throw new UnauthorizedError();

  const retry = await fetch(`${API_BASE}${path}`, { ...init, credentials: 'include' });
  return retry;
}

async function doRefresh(): Promise<boolean> {
  const r = await fetch(`${API_BASE}/auth/refresh`, { method: 'POST', credentials: 'include' });
  if (!r.ok) { await logout(); return false; }
  return true;
}
```

- All `lib/keys.ts`, `logs.ts`, `providers.ts`, `api.ts` functions route through `apiFetch`.
- `credentials: 'include'` on every call (no manual bearer header — tokens are cookies now).

### 4.4 Login page — `src/app/login/page.tsx`

- Centered card (Godwit design system).
- Email + password inputs, "Sign in" button.
- Loading state, inline error message on failure (401 → "Invalid credentials").
- On success → router.push('/').
- Full-width background `bg-surface-container-low`.

### 4.5 Route guard

**Approach: client component wrapper** (simplest, works with the standalone output):
- `src/components/auth/RequireAuth.tsx` — checks auth store; if `unauthenticated`, `<Redirect to /login>`; shows a splash while `status === 'unknown'` (calling `fetchMe`).
- Wrap each protected page (`dashboard`, `providers`, `keys`, `logs`) by composing inside the page index or a `(protected)` route group.

**Optionally** a root `middleware.ts` for server-side guard — deferred to keep this iteration light (client guard is sufficient and testable).

### 4.6 Header identity + logout

- `Header.tsx`: show user email + a "Sign out" button (calls `logout()` then navigates to `/login`).
- Show a subtle "Signed in as …" caption.

### 4.7 Docker/env

- `docker-compose.yml` `ui` service: set `NEXT_PUBLIC_API_ORIGIN` build arg → the api service (e.g. `http://api:8000`), so the rewrite proxy targets the docker backend.
- Keep browser-facing URL on the UI origin (host `:3002`).

---

## 5. Testing Strategy

### Backend (Rust)
- `auth` unit tests:
  - `login` sets `godwit_access` + `godwit_refresh` `Set-Cookie` headers.
  - `logout` clears both cookies.
  - `jwt_auth` middleware accepts token via cookie (no Bearer header).
  - `jwt_auth` still accepts Bearer header (backward compat).
  - `/auth/me` returns current user from cookie.
- Existing 401 tests updated: now use cookie OR header.

### Frontend (Vitest + Testing Library)
- `auth.test.ts`: `login`/`logout`/`fetchMe` call correct endpoints with `credentials: 'include'` (mock fetch).
- `api.test.ts` (extend): `apiFetch` adds `credentials: 'include'`; on 401 triggers refresh once and retries; dedups concurrent refresh; on refresh failure raises `UnauthorizedError`.
- `login page` test: form submit → success navigates, failure shows error.
- `RequireAuth` test: renders children when authenticated, redirects to `/login` when not.
- `Header` test: shows signed-in user + sign-out calls logout.

---

## 6. Rollout / Verification

1. Backend cookie changes → `cargo test` (auth) + integration tests.
2. UI auth module + wrapper → `npm run type-check` + `npm test`.
3. E2E (manual / Playwright): login, navigate pages, let access token expire (short TTL), confirm auto-refresh keeps session, logout clears.
4. Docker: `docker compose up --build`, verify same-origin proxy + cookies work in container.

---

## 7. Open/Deferred

- **Remember-me / session persistence**: cookies already persist; no separate toggle needed for this iteration.
- **OIDC/SSO**: backend has OIDC routes; wiring them into the UI login is a follow-up (out of scope here).
- **Server-side middleware guard**: deferred; client `RequireAuth` is sufficient now.
- **CSRF origin check**: implement a simple `Origin` check in backend; full CSRF token auth can be a later hardening step.

---

**END OF DESIGN SPEC**
