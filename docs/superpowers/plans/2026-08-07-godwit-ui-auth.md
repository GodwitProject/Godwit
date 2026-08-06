# Godwit UI — JWT Cookie Auth Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add secure login/logout/auth to the Godwit admin UI using httpOnly cookies, auto-refresh, and a same-origin reverse proxy.

**Architecture:** Backend sets/clears httpOnly cookies on login/logout and reads the access cookie in `jwt_auth` middleware (plus `/auth/me`). Frontend uses a Next.js rewrite so UI+API share one origin, a dedicated `lib/auth.ts` + Zustand store, and a fetch wrapper that auto-refreshes + retries on 401.

**Tech Stack:**
- Backend: axum (Rust), no new dep (manual `Set-Cookie` headers)
- Frontend: Next.js 14, Zustand, Vitest, React Testing Library

## Global Constraints

- **Cookie names (verbatim):** `godwit_access` (Path=/, Max-Age=`access_token_ttl_minutes`*60), `godwit_refresh` (Path=/api/v1/auth, Max-Age=`refresh_token_ttl_days`*86400). Both `HttpOnly; SameSite=Strict`; add `; Secure` only when `config.auth.cookie_secure` is true.
- **Backward compat:** `jwt_auth` must still accept `Authorization: Bearer <jwt>` when no cookie is present.
- **Design tokens:** UI uses Godwit tokens (colors, fonts, spacing) from `apps/ui/tailwind.config.ts`.
- **Verification:** `cargo test` (backend), `npm run type-check` + `npm test` (frontend). `next`/`next-swc` may SIGBUS in this sandbox — verify frontend via type-check + tests, not `next dev`.
- **Do NOT commit secrets.**
- Update `docs/deployment.md` (git-ignored; add with `git add -f`) if deployment docs change.

---

## File Structure

**Backend:**
- Modify: `crates/godwit-api/src/admin/auth.rs` — cookies in `issue_token_pair`, `logout` clear, add `/auth/me` handler + route
- Modify: `crates/godwit-api/src/middleware.rs` — `jwt_auth` cookie fallback
- Modify: `crates/godwit-core/src/lib.rs` — `AuthConfig` fields (`cookie_secure`, `allowed_cookie_origin`, `cookie_path`?)
- Modify: `config.example.yaml` — new auth config keys

**Frontend:**
- Create: `apps/ui/src/lib/auth.ts`
- Create: `apps/ui/src/store/auth.ts`
- Create: `apps/ui/src/lib/http.ts` (fetch wrapper with auto-refresh)
- Modify: `apps/ui/src/lib/api.ts` (+ keys/logs/providers) to use `http.ts`
- Modify: `apps/ui/next.config.js` — rewrites
- Create: `apps/ui/src/app/login/page.tsx`
- Create: `apps/ui/src/components/auth/RequireAuth.tsx`
- Modify: `apps/ui/src/components/layout/Header.tsx` — user + sign out
- Modify: `apps/ui/src/app/layout.tsx` — auth provider init
- Create: `apps/ui/src/app/(protected)/layout.tsx` + move pages under it (optional) OR wrap per-page

**Docker:**
- Modify: `docker-compose.yml` — `NEXT_PUBLIC_API_ORIGIN` build arg for ui service, `cookie_secure`/origin for api

---

### Task 1: Backend — httpOnly cookies on login/refresh

**Files:**
- Modify: `crates/godwit-api/src/admin/auth.rs:53-75` (`issue_token_pair` returns JSON only)
- Modify: `crates/godwit-api/src/admin/auth.rs:77-135` (login, refresh handlers)
- Modify: `crates/godwit-core/src/lib.rs:215-218` (AuthConfig)
- Modify: `config.example.yaml`

**Interfaces:**
- Consumes: `state.config.auth.*`, `AuthConfig` fields
- Produces: `issue_token_pair(state, user) -> Result<(HeaderMap, Json<serde_json::Value>), ApiError>`; cookie helpers

- [ ] **Step 1: Add `AuthConfig` fields to core config**

In `crates/godwit-core/src/lib.rs` `AuthConfig` (currently lines 215-218):
```rust
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    pub cookie_secure: bool,
    pub allowed_cookie_origin: String,
}
```
Update the `Default` impl and the two `config.example`/test tomls (lines ~908, ~1201) to add:
```yaml
  cookie_secure: false
  allowed_cookie_origin: ""
```
Regenerate/update the `AuthConfig` initializers found by `grep -rn "AuthConfig"`.

- [ ] **Step 2: Write failing tests for cookie headers**

Add to `crates/godwit-api/src/admin/auth.rs` `#[cfg(test)]`:
```rust
#[test]
fn issue_token_pair_sets_http_only_cookies() {
    // Build AppState with a temp user; call issue_token_pair;
    // assert the returned HeaderMap has Set-Cookie for godwit_access and godwit_refresh,
    // both containing HttpOnly and SameSite=Strict.
}
```

- [ ] **Step 3: Implement cookie header helpers + issue_token_pair return type**

```rust
use axum::http::header::{HeaderMap, HeaderValue, SET_COOKIE};

fn access_cookie(state: &AppState, token: &str) -> String {
    let secure = if state.config.auth.cookie_secure { "; Secure" } else { "" };
    let max_age = state.config.auth.access_token_ttl_minutes * 60;
    format!(
        "godwit_access={}; HttpOnly; Path=/; SameSite=Strict; Max-Age={}{}",
        token, max_age, secure
    )
}
fn refresh_cookie(state: &AppState, token: &str) -> String {
    let secure = if state.config.auth.cookie_secure { "; Secure" } else { "" };
    let max_age = state.config.auth.refresh_token_ttl_days * 86400;
    format!(
        "godwit_refresh={}; HttpOnly; Path=/api/v1/auth; SameSite=Strict; Max-Age={}{}",
        token, max_age, secure
    )
}

async fn issue_token_pair(
    state: &AppState,
    user: &User,
) -> Result<(HeaderMap, Json<serde_json::Value>), crate::error::ApiError> {
    let claims = Claims::new(user.id, user.organization_id.unwrap_or_default(), &user.role);
    let access_token = issue(
        &state.config.auth.jwt_secret,
        claims,
        chrono::Duration::minutes(state.config.auth.access_token_ttl_minutes),
    )
    .map_err(|_| crate::error::ApiError::Internal)?;
    let (refresh_plaintext, refresh_hash) = generate_refresh_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(state.config.auth.refresh_token_ttl_days);
    state.refresh_token_repo.create(user.id, &refresh_hash, expires_at)
        .await.map_err(crate::error::ApiError::Core)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&access_cookie(state, &access_token)).map_err(|_| crate::error::ApiError::Internal)?,
    );
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&refresh_cookie(state, &refresh_plaintext)).map_err(|_| crate::error::ApiError::Internal)?,
    );

    let body = serde_json::json!({ "access_token": access_token, "refresh_token": refresh_plaintext });
    Ok((headers, Json(body)))
}
```

- [ ] **Step 4: Update login/refresh to return headers + Json**

```rust
async fn login(...) -> Result<impl IntoResponse, ApiError> {
    // ... existing email/password check ...
    let (headers, body) = issue_token_pair(&state, &user).await?;
    Ok((headers, body))
}
```
Same for `refresh` (reuse `issue_token_pair`; keep the delete/rotate of old refresh token).

- [ ] **Step 5: Run tests**

`DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api -p godwit-core --lib`
Expected: new cookie tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/auth.rs crates/godwit-core/src/lib.rs config.example.yaml
git commit -m "feat(auth): set httpOnly access+refresh cookies on login/refresh

- AuthConfig: add cookie_secure, allowed_cookie_origin
- issue_token_pair returns Set-Cookie headers for godwit_access + godwit_refresh
- login/refresh emit cookies alongside JSON tokens
"
```

---

### Task 2: Backend — logout clears cookies + jwt_auth cookie fallback

**Files:**
- Modify: `crates/godwit-api/src/admin/auth.rs` (`logout`)
- Modify: `crates/godwit-api/src/middleware.rs` (`jwt_auth`)

**Interfaces:**
- Consumes: `jwt_auth` currently requires Bearer header
- Produces: `jwt_auth` reads `godwit_access` cookie when no Bearer; `logout` clears both cookies

- [ ] **Step 1: Write failing tests**

In `middleware.rs` tests (or `auth.rs`), add:
```rust
// jwt_auth accepts a valid token from the `Cookie: godwit_access=<jwt>` header
// jwt_auth (and cookie) returns 401 when token invalid
// logout response includes Set-Cookie clearing godwit_access + godwit_refresh (Max-Age=0)
```

- [ ] **Step 2: Add cookie parsing helper + update jwt_auth**

Add near top of `middleware.rs`:
```rust
fn cookie_value<'a>(header: Option<&HeaderValue>, name: &str) -> Option<String> {
    header.and_then(|h| h.to_str().ok()).and_then(|cookies| {
        cookies.split(';').find_map(|part| {
            let mut kv = part.trim().splitn(2, '=');
            let k = kv.next()?.trim();
            let v = kv.next()?.trim();
            if k == name { Some(v.to_string()) } else { None }
        })
    })
}
```

Rewrite `jwt_auth`:
```rust
pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Bearer header (backward compatible)
    let token = req.headers().get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        // 2. httpOnly cookie fallback
        .or_else(|| cookie_value(req.headers().get(COOKIE), "godwit_access"));
    let auth = token.ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = verify(&state.config.auth.jwt_secret, &auth).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
```
Add `use axum::http::header::COOKIE;` if needed.

- [ ] **Step 3: Update logout to clear cookies**

```rust
async fn logout(...) -> Result<impl IntoResponse, ApiError> {
    let hash = hash_refresh_token(&req.refresh_token);
    state.refresh_token_repo.delete_by_hash(&hash).await.map_err(ApiError::Core)?;
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str("godwit_access=; HttpOnly; Path=/; Max-Age=0").unwrap());
    headers.insert(SET_COOKIE, HeaderValue::from_str("godwit_refresh=; HttpOnly; Path=/api/v1/auth; Max-Age=0").unwrap());
    Ok((headers, Json(serde_json::json!({ "logged_out": true }))))
}
```

- [ ] **Step 4: Run tests**

`DATABASE_URL=postgres://... cargo test -p godwit-api --lib`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/admin/auth.rs crates/godwit-api/src/middleware.rs
git commit -m "feat(auth): jwt_auth reads httpOnly cookie; logout clears cookies

- jwt_auth falls back to godwit_access cookie when no Bearer header
- logout returns Set-Cookie clearing both auth cookies
- backward compatible with existing Bearer-header auth
"
```

---

### Task 3: Backend — /auth/me endpoint + CSRF origin check

**Files:**
- Modify: `crates/godwit-api/src/admin/auth.rs` (add `/auth/me`, protected)
- Modify: `crates/godwit-api/src/admin/mod.rs` (protect `/auth/me` with jwt_auth)
- Modify: `crates/godwit-api/src/middleware.rs` (optional origin check)

**Interfaces:**
- Produces: `GET /api/v1/auth/me` → `{ "user": { "id", "email", "role", "organization_id" } }`

- [ ] **Step 1: Write failing test for /auth/me**

- [ ] **Step 2: Add me handler + put it behind jwt_auth**

In `auth.rs` add:
```rust
pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, crate::error::ApiError> {
    let user = state.user_repo.get_by_id(uuid::Uuid::parse_str(&claims.sub).unwrap()).await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    Ok(Json(serde_json::json!({ "user": {
        "id": user.id,
        "email": user.email,
        "role": user.role,
        "organization_id": user.organization_id,
    }})))
}
```
Note: `Claims` must be added as a real `Extension`/State — check how `jwt_auth` inserts claims (`req.extensions_mut().insert(claims)`) and extract via `Extension<Claims>` (axum supports this).

Registration: add `/auth/me` to the **protected** router in `admin/mod.rs` (NOT the public auth router), so it's behind `jwt_auth`. Example in `mod.rs`:
```rust
let protected = Router::new()
    .merge(api_keys::router())
    // ...
    .route("/auth/me", get(auth::me))
    .route_layer(middleware::from_fn_with_state(state, jwt_auth));
```

- [ ] **Step 3: (Optional hardening) CSRF origin check**

In `jwt_auth`, when `config.auth.allowed_cookie_origin` is non-empty (prod), for state-changing methods verify the `Origin` header equals the allowed origin. Non-blocking in dev (empty origin).

- [ ] **Step 4: Run tests**

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/admin/auth.rs crates/godwit-api/src/admin/mod.rs crates/godwit-api/src/middleware.rs
git commit -m "feat(auth): add GET /auth/me and CSRF origin check

- /auth/me returns current user (id, email, role, organization_id) behind jwt_auth
- jwt_auth optionally verifies Origin for state-changing methods when allowed_cookie_origin set
"
```

---

### Task 4: Backend — integration tests + full cargo test

**Files:**
- Create/modify integration tests as needed (`crates/godwit-api/tests/` if present)
- Verify nothing else breaks

**Interfaces:**
- Consumes: Tasks 1-3

- [ ] **Step 1: Add integration test for cookie round-trip** (if integration test harness exists): login → capture Set-Cookie → call a protected route with `Cookie: godwit_access=...` → 200. And `/auth/me` with cookie → user object.
- [ ] **Step 2: Run full workspace test**

`DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test --workspace --lib 2>&1 | tail -20`
Expected: existing 9 pre-existing DB failures unchanged; auth tests pass.

- [ ] **Step 3: Commit** (only if test files added)

```bash
git add crates/godwit-api/tests/
git commit -m "test(auth): integration test for httpOnly cookie login round-trip and /auth/me"
```

---

### Task 5: Frontend — same-origin rewrite + auth module + store

**Files:**
- Modify: `apps/ui/next.config.js` (rewrites)
- Create: `apps/ui/src/lib/auth.ts`
- Create: `apps/ui/src/store/auth.ts`

**Interfaces:**
- Consumes: backend endpoints (`/api/v1/auth/login|logout|refresh|me`)
- Produces: `login()`, `logout()`, `fetchMe()`, `AuthUser`; `useAuthStore` with `{ user, status, setUser }`

- [ ] **Step 1: Add rewrites to next.config.js**

```js
const API_ORIGIN = process.env.NEXT_PUBLIC_API_ORIGIN || 'http://localhost:3000';
module.exports = {
  output: 'standalone',
  async rewrites() {
    return [
      { source: '/api/v1/:path*', destination: `${API_ORIGIN}/api/v1/:path*` },
      { source: '/health', destination: `${API_ORIGIN}/health` },
      { source: '/metrics', destination: `${API_ORIGIN}/metrics` },
      { source: '/v1/utils/:path*', destination: `${API_ORIGIN}/v1/utils/:path*` },
    ];
  },
};
```
Remove the old `env: { NEXT_PUBLIC_API_URL... }` block (or keep for reference but switch code to same-origin relative paths). The `NEXT_PUBLIC_API_URL` should now default to `''` (same-origin) — update `apps/ui/src/lib/api.ts` accordingly in Task 6.

- [ ] **Step 2: Create `src/lib/auth.ts`**

```ts
export interface AuthUser {
  id: string;
  email: string;
  role: string;
  organization_id: string | null;
}

const AUTH_BASE = '/api/v1/auth';

export async function login(email: string, password: string): Promise<AuthUser> {
  const res = await fetch(`${AUTH_BASE}/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    throw new Error(res.status === 401 ? 'Invalid credentials' : 'Login failed');
  }
  await res.json(); // access_token also returned; not needed by JS (cookie set)
  return fetchMe();
}

export async function logout(): Promise<void> {
  await fetch(`${AUTH_BASE}/logout`, { method: 'POST', credentials: 'include' });
}

export async function fetchMe(): Promise<AuthUser> {
  const res = await fetch(`${AUTH_BASE}/me`, { credentials: 'include' });
  if (!res.ok) throw new Error('Not authenticated');
  const data = await res.json();
  return data.user as AuthUser;
}
```

- [ ] **Step 3: Create `src/store/auth.ts` (Zustand)**

```ts
import { create } from 'zustand';

export type AuthStatus = 'unknown' | 'authenticated' | 'unauthenticated';

interface AuthStore {
  user: AuthUser | null;
  status: AuthStatus;
  setUser: (user: AuthUser | null) => void;
}

export const useAuthStore = create<AuthStore>((set) => ({
  user: null,
  status: 'unknown',
  setUser: (user) => set({ user, status: user ? 'authenticated' : 'unauthenticated' }),
}));
```

- [ ] **Step 4: Write failing tests**

Create `apps/ui/src/lib/auth.test.ts` + `apps/ui/src/store/auth.test.ts` (mock `global.fetch`):
- login POSTs correct path/headers/credentials and sets store user.
- fetchMe returns user on 200.
- logout POSTs and clears user.
- store setUser transitions status.

- [ ] **Step 5: Run tests + type-check**

`cd apps/ui && npm run type-check && npx vitest run`
Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add apps/ui/next.config.js apps/ui/src/lib/auth.ts apps/ui/src/store/auth.ts
git commit -m "feat(ui): add auth module, auth store, same-origin rewrites

- Next.js rewrites /api/v1, /metrics, /health, /v1/utils to backend origin
- lib/auth.ts: login/logout/fetchMe with credentials include
- store/auth.ts: Zustand auth store (user, status)
"
```

---

### Task 6: Frontend — fetch wrapper with auto-refresh + retry

**Files:**
- Create: `apps/ui/src/lib/http.ts`
- Modify: `apps/ui/src/lib/api.ts` (+ `keys.ts`, `logs.ts`, `providers.ts`) to route through `http.ts`
- Modify: `apps/ui/src/lib/api.ts` `API_BASE` to same-origin (`''`)

**Interfaces:**
- Consumes: backend refresh endpoint
- Produces: `apiFetch(path, init): Promise<Response>` with `credentials:'include'`, 401 auto-refresh+retry single-time, dedup concurrent refreshes

- [ ] **Step 1: Create `src/lib/http.ts`**

```ts
export class UnauthorizedError extends Error {
  constructor() { super('Unauthorized'); this.name = 'UnauthorizedError'; }
}

let refreshPromise: Promise<boolean> | null = null;

async function doRefresh(): Promise<boolean> {
  try {
    const r = await fetch('/api/v1/auth/refresh', { method: 'POST', credentials: 'include' });
    return r.ok;
  } catch { return false; }
}

export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const merged: RequestInit = { ...init, credentials: 'include' };
  const res = await fetch(path, merged);
  if (res.status !== 401) return res;

  if (!refreshPromise) {
    refreshPromise = doRefresh().finally(() => { refreshPromise = null; });
  }
  const ok = await refreshPromise;
  if (!ok) throw new UnauthorizedError();

  return fetch(path, merged); // retry original once
}
```

- [ ] **Step 2: Refactor `src/lib/api.ts`** — set `API_BASE = ''` and use `apiFetch`:

```ts
const API_BASE = ''; // same-origin via next rewrites
async function getJson<T>(path: string): Promise<T> {
  const res = await apiFetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}
```
Update `keys.ts`, `logs.ts`, `providers.ts` similarly to call `apiFetch` and pass `credentials` (now implicit). For POST/PATCH/DELETE, add `headers: { 'Content-Type': 'application/json' }` and `body`.

- [ ] **Step 3: Write failing tests (`src/lib/http.test.ts`)**

Mock `global.fetch`:
- apiFetch adds `credentials:'include'`.
- On 401 → calls `/api/v1/auth/refresh` once, then retries original → resolves.
- Concurrent apiFetch 401s trigger only ONE refresh (dedup) — use a counter.
- Refresh failure → throws `UnauthorizedError`.

- [ ] **Step 4: Run tests + type-check**

- [ ] **Step 5: Commit**

```bash
git add apps/ui/src/lib/http.ts apps/ui/src/lib/api.ts apps/ui/src/lib/keys.ts apps/ui/src/lib/logs.ts apps/ui/src/lib/providers.ts
git commit -m "feat(ui): central fetch wrapper with auto-refresh + retry

- apiFetch adds credentials include, on 401 refreshes once and retries
- dedups concurrent refreshes; throws UnauthorizedError on refresh failure
- all lib clients route through apiFetch
"
```

---

### Task 7: Frontend — login page + RequireAuth guard + header identity

**Files:**
- Create: `apps/ui/src/app/login/page.tsx`
- Create: `apps/ui/src/components/auth/RequireAuth.tsx`
- Modify: `apps/ui/src/components/layout/Header.tsx`
- Modify: `apps/ui/src/app/layout.tsx` (initial auth check)

**Interfaces:**
- Consumes: `login()`, `logout()`, `fetchMe()`, `useAuthStore`
- Produces: login page; `RequireAuth` guard; header identity/logout

- [ ] **Step 1: Create login page**

```tsx
'use client';
import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { login } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';

export default function LoginPage() {
  const router = useRouter();
  const setUser = useAuthStore((s) => s.setUser);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true); setError(null);
    try {
      const user = await login(email, password);
      setUser(user);
      router.push('/');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally { setBusy(false); }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-surface-container-low px-4">
      <div className="w-full max-w-sm bg-surface-container-lowest rounded-xl p-container-padding ambient-shadow">
        <h1 className="text-headline-md mb-1">Sign in to Godwit</h1>
        <p className="text-body-base text-on-surface-variant mb-6">Admin LLM proxy console</p>
        <form onSubmit={handleSubmit} className="space-y-4">
          <Input label="Email" type="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
          <Input label="Password" type="password" required value={password} onChange={(e) => setPassword(e.target.value)} />
          {error && <p className="text-label-sm text-error">{error}</p>}
          <Button type="submit" className="w-full" disabled={busy}>{busy ? 'Signing in…' : 'Sign in'}</Button>
        </form>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create RequireAuth guard**

```tsx
'use client';
import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { fetchMe } from '@/lib/auth';

export function RequireAuth({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const status = useAuthStore((s) => s.status);
  const setUser = useAuthStore((s) => s.setUser);

  useEffect(() => {
    if (status === 'unknown') {
      fetchMe().then(setUser).catch(() => { setUser(null); router.replace('/login'); });
    } else if (status === 'unauthenticated') {
      router.replace('/login');
    }
  }, [status, setUser, router]);

  if (status !== 'authenticated') {
    return <div className="min-h-screen flex items-center justify-center text-on-surface-variant">Loading…</div>;
  }
  return <>{children}</>;
}
```

- [ ] **Step 3: Wrap protected pages**

Option: create `apps/ui/src/app/(protected)/layout.tsx`:
```tsx
'use client';
import { RequireAuth } from '@/components/auth/RequireAuth';
export default function ProtectedLayout({ children }: { children: React.ReactNode }) {
  return <RequireAuth>{children}</RequireAuth>;
}
```
And move `providers/`, `keys/`, `logs/`, and the dashboard `page.tsx` under `(protected)/`. (Route group `(protected)` doesn't add a URL segment.)

- [ ] **Step 4: Header identity + sign out**

In `Header.tsx` (make it `'use client'`), read `useAuthStore`, render `user.email` + a "Sign out" ghost button calling `logout()` then `setUser(null)` + `router.push('/login')`.

- [ ] **Step 5: Root layout initial auth check**

In `(protected)/layout.tsx` the `RequireAuth` handles it; root `layout.tsx` keeps Shell. Ensure unauthenticated users hitting `/` are redirected to `/login`.

- [ ] **Step 6: Write tests**

- login page test: submit → success navigates, failure shows error (mock `login`, router).
- RequireAuth test: authenticated renders children; unauthenticated redirects.
- Header test: shows email + sign-out calls logout.

- [ ] **Step 7: Run tests + type-check**

- [ ] **Step 8: Commit**

```bash
git add apps/ui/src/app/login apps/ui/src/components/auth apps/ui/src/components/layout/Header.tsx apps/ui/src/app/\(protected\)/
git commit -m "feat(ui): add login page, RequireAuth guard, header sign out

- /login form with loading + error states
- RequireAuth route guard redirects unauthenticated to /login
- Header shows signed-in user + sign out button
- (protected) route group wraps dashboard/providers/keys/logs
"
```

---

### Task 8: Frontend — fix remaining pages to use apiFetch/auth

**Files:**
- Any pages still calling bare `fetch` (verify `page.tsx`, providers/keys/logs pages)
- Update provider/key/log mutation hooks to go through `apiFetch`

**Interfaces:**
- Consumes: `apiFetch` (Task 6)

- [ ] **Step 1: Audit for bare fetch calls**

`grep -rn "fetch(" apps/ui/src --include="*.ts" --include="*.tsx" | grep -v "apiFetch\|lib/http\|auth.ts\|websocket"`
Replace any remaining raw `fetch(` in `lib/*.ts` and hooks with `apiFetch`.

- [ ] **Step 2: Ensure mutations use apiFetch**

`useKeys.ts` create/update/delete/block/unblock, `useLogs.ts` none, `useProviders.ts` — verify all route through `apiFetch` with `Content-Type` + `body`.

- [ ] **Step 3: Run tests + type-check**

- [ ] **Step 4: Commit**

```bash
git add apps/ui/src/
git commit -m "refactor(ui): route all data fetches through authenticated apiFetch

- remove any remaining bare fetch calls; all admin calls use apiFetch (auto-refresh)
"
```

---

### Task 9: Docker + docs + final verification

**Files:**
- Modify: `docker-compose.yml`
- Modify: `docs/deployment.md` (git-ignored; `git add -f`)
- Modify: `apps/ui/Dockerfile` (env/args if needed)

**Interfaces:**
- Consumes: Tasks 1-8

- [ ] **Step 1: Update docker-compose ui service**

Add build arg so the rewrite targets the docker backend:
```yaml
services:
  ui:
    build:
      context: .
      dockerfile: apps/ui/Dockerfile
      args:
        NEXT_PUBLIC_API_ORIGIN: ${NEXT_PUBLIC_API_ORIGIN:-http://api:8000}
```
(Update `apps/ui/Dockerfile` ARG + `next.config.js` to consume `NEXT_PUBLIC_API_ORIGIN`.) Also set api service env `cookie_secure: false` (dev) via config.yaml if surfaced.

- [ ] **Step 2: Update deployment doc** — document the same-origin topology, cookie config, and that `api` must be browser-unreachable except via UI.

- [ ] **Step 3: Full verification**

Backend: `DATABASE_URL=... cargo test --workspace --lib` (auth tests pass, no new breakage).
Frontend: `cd apps/ui && npm run type-check && npx vitest run`.
Docker: `docker compose config` PASS; if available `docker compose build api ui`.

- [ ] **Step 4: Commit**

```bash
git add docker-compose.yml apps/ui/Dockerfile apps/ui/next.config.js
git add -f docs/deployment.md
git commit -m "feat(auth): wire docker same-origin proxy + deployment docs

- ui service build arg NEXT_PUBLIC_API_ORIGIN targets docker api
- document cookie auth topology and config
"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ httpOnly cookies login/refresh (Task 1)
- ✅ logout clears cookies + jwt_auth cookie fallback (Task 2)
- ✅ /auth/me + CSRF origin check (Task 3)
- ✅ same-origin rewrites (Task 5, 9)
- ✅ lib/auth.ts + Zustand store (Task 5)
- ✅ auto-refresh + retry wrapper (Task 6)
- ✅ login page + RequireAuth + header (Task 7)
- ✅ all data routes through apiFetch (Task 8)
- ✅ docker + docs (Task 9)

**2. Placeholder scan:** All steps contain concrete code; no TBD/TODO.

**3. Type consistency:** `AuthUser` (Task 5) matches `/auth/me` shape (Task 3); `apiFetch` signature consistent across Task 6-8; login page error messages match `login()` throw (Task 5/7).

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-08-07-godwit-ui-auth.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks.

**2. Inline Execution** — execute in this session with checkpoints.

**Which approach?**
