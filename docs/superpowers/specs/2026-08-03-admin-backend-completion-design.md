# Admin Backend Completion — Design Specification

## 1. Goal

Complete the admin REST API so every resource it exposes is actually backed by real logic — today several endpoints are stubs (`GET /teams` always returns `[]`, `GET/PATCH/DELETE /users/:id` always return 404, `GET /spend` always returns `[]`, and there is no way to create an organization at all). This is a prerequisite for building an admin web UI (a separate, later sub-project): a UI can't manage what the API doesn't yet support.

This spec also adds a refresh-token flow, since the current 15-minute access token with no renewal mechanism would force a web UI to re-prompt for a password every 15 minutes.

## 2. Current State

- **Auth:** `POST /auth/login` (password) and OIDC issue a 15-minute JWT access token. No refresh token exists anywhere in the codebase.
- **Organizations:** `GET /organizations` only (`super_admin`-only, returns all orgs). No create/update.
- **Teams:** `GET /teams` is a stub — always returns `{ "data": [] }`, no DB query at all.
- **Users:** `GET /users` (list, filtered to `claims.organization_id`) and `POST /users` (create) work. `GET/PATCH/DELETE /users/:id` are stubs that always return `ApiError::NotFound`.
- **API keys:** `GET`/`POST /api-keys` work (unaffected by this spec).
- **Spend:** `GET /spend` is a stub — always returns `{ "data": [] }`, ignoring the `request_logs` table entirely, which already has `cost_usd`, `tokens_in`, `tokens_out`, `organization_id`, `team_id`, `user_id`, `api_key_id`, `created_at` populated on every proxy request.
- **RBAC quirk:** `GET /users` filters by `claims.organization_id` even for `super_admin` — inconsistent with `GET /organizations`, which is already global for `super_admin`.

## 3. Scope

### In scope
- Refresh-token flow (`POST /auth/refresh`, `POST /auth/logout`), rotating on each use.
- `POST /organizations`, `PATCH /organizations/:id`.
- Real `GET /teams`, `POST /teams`, `PATCH /teams/:id`, and team-membership management (`POST`/`DELETE /teams/:id/members[/:user_id]`).
- Real `GET /users/:id`, `PATCH /users/:id`, `DELETE /users/:id` (with cascading deletes).
- A consistent RBAC scoping model: `super_admin` becomes global (any org, or all orgs) for users/teams, matching its existing global scope for organizations; `org_admin` stays scoped to its own org; `team_admin`/`user` get self-service spend visibility only.
- Real `GET /spend`, aggregating `request_logs` with optional date range and `organization_id`/`team_id`/`user_id` filters.
- Migrations: `refresh_tokens` table; `api_keys.user_id` → `ON DELETE CASCADE`; `request_logs.user_id` → `ON DELETE SET NULL`.

### Out of scope
- The admin web UI itself (separate, later sub-project).
- SAML (already an unimplemented stub, untouched).
- Extending access-token lifetime (stays 15 minutes; the refresh token is what removes the friction).
- `team_admin`-scoped spend visibility (their own team's aggregate, not just their own usage) — not requested; `team_admin` gets the same self-only visibility as a plain `user` for now.
- A `group_by` parameter for spend (choosing which dimensions to aggregate over) — the response already returns all three dimensions (`organization_id`, `team_id`, `user_id`) per row, so a caller can roll up by summing; a dedicated parameter can be added later if that turns out to be insufficient.

## 4. RBAC Scoping Model

Applies to `GET /users`, `GET /users/:id` (implicitly), `GET/POST/PATCH /teams`, and `GET /spend`:

- **`super_admin`:** an optional `organization_id` query parameter filters to one org; omitted, the response spans every organization.
- **`org_admin`:** any `organization_id` parameter is ignored; always forced to `claims.organization_id`.
- **`team_admin` / `user`** (spend only): `organization_id`/`team_id`/`user_id` parameters are ignored; always forced to `user_id = claims.user_id` (and implicitly their own `organization_id`).

`PATCH /users/:id` additionally restricts `organization_id` reassignment to `super_admin` only — `org_admin` may edit `name`/`role` for a user in its own org but cannot move that user to a different org.

## 5. Refresh Tokens

New migration `20260803000004_refresh_tokens.up.sql` / `.down.sql`:

```sql
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

- Generation/hashing follows the same pattern as API keys (`godwit_auth::api_keys`): a random opaque token, only its hash stored.
- `POST /auth/login` and the OIDC callback now return `{ "access_token": "...", "refresh_token": "..." }` instead of just `access_token`. Refresh tokens are valid for 7 days.
- `POST /auth/refresh` — body `{ "refresh_token": "..." }`. Looks up the hash, checks `expires_at`, deletes the used row (rotation — a stolen-and-reused refresh token becomes detectable since the legitimate client's next refresh will fail), issues a new access token + a new refresh token row. Returns the same shape as login.
- `POST /auth/logout` — body `{ "refresh_token": "..." }`. Deletes the row. No error if the token doesn't exist (idempotent logout).
- Access-token lifetime is unchanged (15 minutes); only the renewal mechanism is new.

## 6. Organizations

```
POST  /organizations       { name, rate_limit_requests_per_minute? }   → super_admin
PATCH /organizations/:id   { name?, rate_limit_requests_per_minute? }  → super_admin
```

`OrganizationRepository::create` is extended to accept the optional rate limit (currently takes only `name`). A new `OrganizationRepository::update` method is added for the PATCH.

## 7. Teams

```
GET   /teams?organization_id=<uuid?>
POST  /teams               { name, organization_id? }
PATCH /teams/:id           { name }
```

Role gate (unchanged from today): `super_admin`/`org_admin` only (`Role::can_manage_users()`), same as the existing `GET`/`POST /users` gate. `team_admin`/`user` cannot list or manage teams at this level (they interact with their own team's membership only, via §7's memberships section below, and see their own spend via §9).

- `GET /teams` now genuinely queries the `teams` table (scoped per §4), replacing the current hardcoded-empty stub.
- `POST /teams`: `org_admin`'s `organization_id` field (if present) is ignored, forced to `claims.organization_id`; `super_admin` must supply `organization_id` to target an org (no implicit "own org" for a super_admin creating a team elsewhere).
- `PATCH /teams/:id`: renames only; `org_admin` may only patch a team belonging to its own org (checked by loading the team and comparing `organization_id`).

### Team memberships

```
POST   /teams/:id/members   { user_id, role: "team_admin" | "member" }
DELETE /teams/:id/members/:user_id
```

Authorization: `super_admin`, `org_admin` (of the team's own organization), **or** a `team_admin` who already holds `team_admin` in `team_memberships` for *this specific team* (checked via a DB lookup on `(user_id = claims.user_id, team_id)` — not just the user's global role). A `team_admin` of team A cannot manage team B's membership.

## 8. Users

```
GET    /users?organization_id=<uuid?>
GET    /users/:id
PATCH  /users/:id   { name?, role?, organization_id? }
DELETE /users/:id
```

Role gate (unchanged from today): `super_admin`/`org_admin` only (`Role::can_manage_users()`), for all four routes including the two new ones (`PATCH`/`DELETE`). `team_admin`/`user` cannot view or manage other users' accounts.

- `GET /users`/`GET /users/:id` scoped per §4.
- `PATCH /users/:id`: `org_admin` may change `name`/`role` for a user in its own org; `organization_id` reassignment requires `super_admin`.
- `DELETE /users/:id`: rejects with `400 Bad Request` if `claims.user_id == target id` (no self-deletion). Otherwise deletes the row; cascading effects come from the schema changes below, not application-level cleanup.

### Migration: cascade behavior on user deletion

New migration `20260803000005_user_delete_cascade.up.sql` / `.down.sql`:

```sql
ALTER TABLE api_keys DROP CONSTRAINT api_keys_user_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE request_logs DROP CONSTRAINT request_logs_user_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;
```

(`team_memberships.user_id` already has `ON DELETE CASCADE` from the initial schema — no change needed there.) Down migration restores the original (implicit `NO ACTION`) constraints — note that, as with the earlier instance-wide-catalog migration, a round-trip up→down does not undo any cascaded deletions that occurred while the `CASCADE`/`SET NULL` behavior was active; it only restores the constraint definitions.

## 9. Spend Tracking

```
GET /spend?from=<ISO8601?>&to=<ISO8601?>&organization_id=<uuid?>&team_id=<uuid?>&user_id=<uuid?>
```

Scoped per §4 (note: `team_admin`/`user` get the self-only rule here, not just `user`). Query aggregates `request_logs`:

```sql
SELECT organization_id, team_id, user_id,
       SUM(cost_usd) AS total_cost_usd,
       COUNT(*) AS request_count,
       SUM(tokens_in) AS tokens_in,
       SUM(tokens_out) AS tokens_out
FROM request_logs
WHERE created_at >= $from AND created_at <= $to  -- both bounds optional, omitted clause if not provided
  AND ($organization_id IS NULL OR organization_id = $organization_id)
  AND ($team_id IS NULL OR team_id = $team_id)
  AND ($user_id IS NULL OR user_id = $user_id)
GROUP BY organization_id, team_id, user_id
```

Response:
```json
{ "data": [
  { "organization_id": "...", "team_id": "...", "user_id": "...",
    "total_cost_usd": "12.3400", "request_count": 481,
    "tokens_in": 152340, "tokens_out": 48120 }
] }
```

A caller wanting an org-wide or team-wide total sums the relevant rows client-side; no separate aggregation endpoint is added.

## 10. Testing Strategy

Follows the repository's existing conventions (no new pattern introduced):
- **Unit/repository tests** (`sqlx::test`): each new repository method (`OrganizationRepository::{create,update}`, a new `TeamRepository`, `TeamMembershipRepository`, extended `UserRepository`, the refresh-token repository) gets create/get/update/delete coverage plus constraint-violation cases (e.g. deleting a user cascades api_keys, nulls request_logs.user_id).
- **Handler-level tests**: RBAC gates per role (matches the pattern already established for `provider-profiles`/`models` in the previous sub-project) — for every new/changed endpoint, at least one "authorized role succeeds" and one "wrong role → 403" test.
- **Integration tests**: extend `crates/godwit-api/tests/router_integration.rs` (already introduced in the provider-catalog sub-project) with end-to-end coverage for: login → refresh → access with new token → logout → refresh fails; org creation by super_admin; team creation + membership add/remove; user deletion cascading to api_keys; spend aggregation returning correct sums for seeded `request_logs` rows across multiple users/teams.

## 11. Acceptance Criteria

- [ ] `POST /auth/login` returns both an access and a refresh token; `POST /auth/refresh` exchanges a valid refresh token for a new pair and invalidates the old refresh token; `POST /auth/logout` invalidates a refresh token.
- [ ] `super_admin` can create and update organizations; `org_admin`/others cannot.
- [ ] `GET /teams` returns real data from the database, scoped per the RBAC model; teams and their memberships can be created, listed, and removed via the API.
- [ ] A `team_admin` can manage membership for their own team but not for a team they don't administer.
- [ ] `GET/PATCH/DELETE /users/:id` all work correctly, including RBAC scoping and self-deletion prevention.
- [ ] Deleting a user cascades: their API keys are deleted, their `request_logs` rows survive with `user_id = NULL`.
- [ ] `super_admin` can list users/teams across all organizations or filtered to one; `org_admin` is always scoped to its own organization.
- [ ] `GET /spend` returns real aggregated cost/token/request-count data from `request_logs`, filterable by date range and by organization/team/user, with `team_admin`/`user` restricted to their own usage.
- [ ] All unit, repository, and integration tests pass with `cargo test --workspace`.
