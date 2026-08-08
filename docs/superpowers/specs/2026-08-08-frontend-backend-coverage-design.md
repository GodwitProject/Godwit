# Front↔Backend Coverage Grid & Route Contract Design

**Date:** 2026-08-08
**Version:** 1.0.0
**Status:** Approved
**Author:** Godwit Team

---

## 1. Overview

The Godwit backend exposes a large set of HTTP routes (proxy `v1/*`, public health/metrics, and admin `api/v1/*`), while the Next.js admin UI consumes only a subset (auth, dashboard/stats, keys, models, providers, logs, metrics). There is no single source of truth that ties a frontend call to its backend route, so regressions can silently break the UI (e.g. the WebSocket `/api/v1/ws/metrics` that the UI calls but the backend never exposed).

This design builds an **exhaustive front↔backend coverage grid**, backed by a **shared route contract** (`contract/routes.json`) that is consumed by both a **backend test** (proving each contract route actually exists in the real router) and **frontend tests** (proving each UI lib points at a contract route). It also implements the missing WebSocket endpoint and leaves the whole suite at **zero test failures**.

### 1.1 Decisions (confirmed with user)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Ambition | Exhaustive coverage grid front↔backend | "Absolutely no bugs", both directions covered |
| Scope | **UI-actual coverage** (auth, stats, keys, models, providers, logs, metrics) | Proxy `v1/*` is SDK-only; uncovered admin routes stay documented out-of-scope |
| WebSocket | **Implement** `/api/v1/ws/metrics` in backend | Cover the FE realtime call instead of leaving it broken |
| Cross tests | **Shared route contract** + verification on both sides | Single source of truth; FE pins URLs, BE proves existence |
| Session order | **Green first → grid → WS** | Zero failures first, then grid, then WS |
| Router refactor | **Expose `app(state)` in `godwit-api`** | Makes the real router testable; removes duplication from `godwit-bin` |

---

## 2. Technical Constraint: Axum has no route introspection

Investigation confirmed **axum 0.7.9 does not expose any public route-introspection API** (`routes()`, `RouteRecord` are private). A backend test therefore cannot "read" the routes off the router.

**Resolution:** The backend contract test mounts the real router in-process, and for each contract route issues a real HTTP request of the declared method against the declared path, then asserts the response is **not** axum's "no route matched" 404 (empty body). Any other status (401, 403, 400, 200…) proves the route exists. This works regardless of how routes are scattered across modules.

To mount the identical router used in production, the root router construction (today private in `crates/godwit-bin/src/main.rs`, lines 124–138) must move into `godwit-api` as a public function `app(state: Arc<AppState>) -> Router<AppState>`. `main.rs` and the tests both call it.

**Additional motivation (found during design review):** `crates/godwit-api/tests/router_integration.rs` already hand-rolls `build_app(pool)` (lines 105–144) to mirror `main.rs`, but it silently **omits** several real routes (`health::router()`, `metrics_endpoint::router()`, `utils::router()`, `moderation::router()`, `rerank::router()`). A shared `app(state)` absorbs this duplication and ends the divergence — the contract test then proves existence against the *same* router production serves.

The shared `app(state)` must assemble, in order (matching today's `main.rs` semantics):
- `health::router()`, `metrics_endpoint::router()` (no auth)
- `proxy::router()`, `anthropic_proxy::router()`, `moderation::router()`, `rerank::router()` with `api_key_auth` route-layer applied (proxy group only)
- `utils::router()` (no auth)
- `.nest("/api/v1", admin::router(state))` (JWT-protected)
- final `.with_state(state)`

---

## 3. Phase 1 — Remission to green (zero failures)

**Goal:** The complete suite (backend Rust + frontend Vitest) passes with zero failures before building anything new.

Known sub-tasks:
1. **Fix 4 failing `budget_check_*` SQLx tests** in `crates/godwit-api/src/rate_limit.rs`:
   - `budget_check_blocks_when_exceeded` (~line 418), `budget_check_allows_when_under_budget` (~460), `budget_check_team_blocks_when_exceeded` (~541), `budget_check_team_allows_when_under_budget` (~585).
   - Root cause: they insert a `request_logs` row with `api_key_id = Uuid::new_v4()` (random), violating FK `request_logs_api_key_id_fkey` now that the SQLx tests run real migrations.
   - Fix: create a real `api_keys` row via `ApiKeyRepository::create(user_id, org_id, name, key_prefix, key_hash, scopes, allowed_models, budget_limit_usd, rate_limit, rate_limit_tokens_per_minute)` and bind `api_key.id` in the `request_logs` insert (same pattern as `seed_api_key` elsewhere).
2. **Full verification sweep:** run `cargo test --workspace` (with `DATABASE_URL`), the integration-test compiles, and the FE Vitest suite; fix any residual failure found.

**Exit criteria:** `cargo test --workspace` green, FE `npm test` green, existing integration tests compile.

---

## 4. Phase 2 — Route contract + coverage grid

### 4.1 The shared contract: `contract/routes.json`

A single JSON file as the single source of truth. One entry per backend route:

```json
[
  {
    "id": "admin.list_api_keys",
    "method": "GET",
    "path": "/api/v1/api-keys",
    "frontend": { "lib": "apps/ui/src/lib/keys.ts", "fn": "listApiKeys" },
    "backend": { "module": "crates/godwit-api/src/admin/api_keys.rs", "fn": "list_api_keys" },
    "scope": "ui"
  }
]
```

Fields:
- `id` — unique slug.
- `method` / `path` — the HTTP method and full path as exposed (verbatim, e.g. `/api/v1/...`, `/v1/...`).
- `frontend` — the UI lib + exported function that calls this route (`null` for SDK-only or uncovered).
- `backend` — the Rust module + handler function (or handler-owning module pair).
- `scope` — one of:
  - `"ui"` — consumed by the current UI; must be enforced by both tests.
  - `"proxy"` — SDK-only routes (`/v1/chat/completions`, etc.); documented, not enforced by UI tests.
  - `"uncovered"` — backend admin routes the UI does not currently consume (organizations, teams, users, end-users, spend/tags, circuit-breakers, model-aliases, etc.); documented as out-of-scope, existence still enforced by the backend test.

### 4.2 Backend contract test — `crates/godwit-api/tests/route_contract.rs`

- Builds the router via the shared `app(state)` — the same offer production serves (health, metrics, utils, proxy group, admin nest).
- Uses a `#[sqlx::test]` pool to construct `AppState` (same pattern as `router_integration.rs` fixtures). Existence checks need no seed data: protected routes return 401/403 before DB logic runs; the assertion is simply "not the route-missing 404".
- Reads `contract/routes.json` (path resolved relative to the workspace root).
- For **every** entry (all scopes), sends a request of `method` on `path` using `tower::ServiceExt::oneshot`.
- Asserts the response status is **not** axum's empty-body "route not found" 404; any non-empty body / non-404 status proves existence.
- Parametrized paths use a concrete placeholder segment (e.g. `{id}` → `00000000-0000-0000-0000-000000000000`).
- Marked `#[ignore]`? No — it is DB-backed via `#[sqlx::test]` and runs with the rest of the suite; it does not require a live server.

### 4.3 Frontend contract test — `apps/ui/tests/route-contract.test.ts`

- Reads `contract/routes.json` (from the workspace root via a relative import).
- Collects all entries with `scope: "ui"`.
- Mechanism (matches existing FE test pattern in e.g. `keys.test.ts`): **mock `fetch`**, invoke the lib function named in the entry's `frontend.fn`, capture the requested URL + method, and assert that `path` (expanded for the parametrized `:id`/`{id}` segments used in the test) and `method` match the contract.
- This turns the contract into an enforced boundary: if a UI function's URL or method drifts from the contract, the test fails; and any contract route without a corresponding UI call is flagged.

### 4.3.1 Contract-entry ↔ FE-call mapping notes

- URL literals live inline per function (e.g. `keys.ts` `blockKey` → `/api/v1/api-keys/${id}/block`), so the test captures them via mocked `fetch` rather than reading a central route table (none exists by design).
- Parametrized entries (websocket aside) normally appear as `/…/{id}/…` in the contract; the FE test instantiates a concrete `id` to exercise the real interpolation.
- The `websocket.ts` entry is special: it is asserted in `websocket.test.ts` (already exists) plus the contract existence check; it is added to the contract with `scope: "ui"`.

### 4.4 Coverage grid document — `docs/coverage/frontend-backend.md`

Rendered table from `contract/routes.json`: route ↔ method ↔ FE lib/fn ↔ BE module/fn ↔ scope ↔ status (covered/uncovered/proxy). Acts as the human-readable contract.

---

## 5. Phase 3 — WebSocket `/api/v1/ws/metrics`

### 5.1 Backend

- New handler module `crates/godwit-api/src/admin/metrics_ws.rs` providing a `GET /ws/metrics` route, mounted under `/api/v1` (thus protected by the same JWT auth as the rest of admin).
- Uses axum `WebSocketUpgrade` to upgrade the connection.
- Rejects the upgrade without valid auth (upgrade attempt returns the protected-route error).

**Protocol (must match `apps/ui/src/lib/websocket.ts` exactly):**
- Client sends on open: `{ "type": "subscribe", "channel": "metrics" }`.
- Server pushes (optionally on `subscribe`, defaults to unsolicited on open) frames of shape:
  ```json
  { "type": "metrics:update", "data": {
      "requestsTotal": 0, "tokensTotal": 0, "costUsdTotal": 0,
      "activeRequests": 0, "timestamp": "ISO-8601" } }
  ```
- Note **camelCase** keys — the WebSocket payload is **not** Prometheus text and differs from `GET /metrics` (Prometheus). The handler derives the four counters from the DB/repo (same source `GET /metrics` uses) but serializes them in the camelCase shape above.
- Interval: fixed (configurable); pushes periodically while the socket is open.

### 5.2 Frontend

- `apps/ui/src/lib/websocket.ts` already targets `ws://…/api/v1/ws/metrics`, sends the `subscribe` handshake, parses `metrics:update`, and falls back to polling `fetchPrometheusMetrics()` ✅. No FE change required beyond confirming the WebSocket now succeeds; keep the polling fallback for resilience.

### 5.3 Tests

- Backend: unit/integration test that a WebSocket upgrade on `/api/v1/ws/metrics` yields at least one metric message over the socket; and that it is protected (no auth → rejected). Route added to `contract/routes.json` with `scope: "ui"`.
- Frontend: extend `websocket.test.ts` to cover the contract entry.

---

## 6. Global Success Criteria

1. **Zero failures:** `cargo test --workspace` and FE `npm test` both green; integration tests compile.
2. **`contract/routes.json`** enumerates every backend route with correct scope; every `"ui"` route maps to a real FE lib/fn and a real BE handler.
3. **Backend contract test** proves every contract route exists in the real `app(state)` router (no 404s).
4. **Frontend contract test** proves every `"ui"` FE call targets a contract route with the correct method/path.
5. **`/api/v1/ws/metrics`** works with auth, is covered by contract + tests, and the FE realtime metrics path no longer 404s.
6. **Coverage grid** documented in `docs/coverage/frontend-backend.md`.

---

## 7. Out of Scope

- Building new UI for uncovered admin routes (organizations, teams, users, end-users, spend/tags, circuit-breakers, model-aliases, key regenerate/reset_spend/sub-routes not yet consumed).
- Extending the proxy `v1/*` group (SDK-facing; not UI).
- OIDC/SSO wiring in the UI (backend routes exist; follow-up).

---

## 8. Open Questions / Risks

- **`app(state)` state construction:** `AppState` requires a `PgPool` + repos; resolved via a `#[sqlx::test]` pool and the same assembly used by `router_integration.rs`. The refactor also fixes the existing `build_app` divergence (missing health/metrics/utils/moderation/rerank).
- **Axum WebSocket interval config:** whether the metrics-stream interval is hardcoded or config-driven; resolved during implementation (small, non-breaking).
- **Contract location & multi-package access:** `contract/routes.json` at workspace root; backend test reads via path relative to CARGO_MANIFEST_DIR, FE test via a relative import. Cross-language path resolution is an implementation detail pinned in the plan.
- **Proxy-route existence without valid API key:** proxy routes return 401 without an API key — that still proves the route exists (non-404). Fine for the grid.

---

**END OF DESIGN SPEC**
