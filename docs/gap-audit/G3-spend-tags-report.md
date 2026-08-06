# G3 Complement: `/spend/tags` Endpoint Report

**Status:** Complete

**Date:** 2026-08-06

## Summary

Implemented `/spend/tags` endpoint to return spend aggregated by implicit tags (`team_id` and `api_key_id`), following LiteLLM's `/spend/tags` pattern.

## Files Modified

1. `crates/godwit-api/src/admin/spend_tags.rs` — New module with endpoint implementation (376 lines)
2. `crates/godwit-api/src/admin/mod.rs` — Registered `spend_tags` module

## Implementation Details

### Endpoint

- **Path:** `GET /api/v1/spend/tags`
- **Query Parameters:**
  - `from: DateTime<Utc>` (optional) — Start of time range
  - `to: DateTime<Utc>` (optional) — End of time range
  - `organization_id: Uuid` (optional) — Filter by organization (super-admin only)

### Response Format

```json
{
  "by_team": [
    { "team_id": "uuid-or-null", "spend_usd": "12.34" },
    { "team_id": null, "spend_usd": "5.67" }
  ],
  "by_api_key": [
    { "api_key_id": "uuid-or-null", "spend_usd": "56.78" }
  ]
}
```

### RBAC

- **Super Admin:** Sees all data, can filter by `organization_id`
- **Org Admin:** Sees only own organization's data
- **User/Team Admin:** Sees only own data (filtered by `user_id`)

### SQL Queries

Two aggregation queries:
1. Team aggregation: `SELECT team_id, SUM(cost_usd) FROM request_logs GROUP BY team_id`
2. API key aggregation: `SELECT api_key_id, SUM(cost_usd) FROM request_logs GROUP BY api_key_id`

Both queries respect `from`/`to` datetime filters and organization/user scoping.

## Test Coverage

### Unit Tests (4)
- `spend_tags_response_serializes_correctly` — JSON serialization
- `spend_tags_scope_forces_org_admin_to_own_org` — RBAC org scoping
- `spend_tags_scope_forces_user_to_self` — RBAC user scoping
- `spend_tags_scope_leaves_super_admin_unscoped` — RBAC super-admin passthrough

**All 4 unit tests: ✅ PASS**

### Database Tests (3)
- `spend_tags_by_team_aggregates_correctly` — Team aggregation
- `spend_tags_by_api_key_aggregates_correctly` — API key aggregation
- `spend_tags_respects_from_to_filters` — Datetime filtering

**Database tests:** Require `DATABASE_URL` to run (expected per testing quirks)

**Total:** 7 tests (4 passing without DB, 3 require DB)

## Verification Commands

```bash
# Compile check
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace --tests
# Result: ✅ No compilation errors

# Unit tests (no DB required)
cargo test -p godwit-api --lib -- admin::spend_tags
# Result: 4 unit tests PASS

# Database tests (requires PostgreSQL)
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib -- admin::spend_tags
# Result: 3 DB tests will run with valid DATABASE_URL
```

## Design Decision: Implicit Tags

Per G3 spec, `request_logs` table does not have a `tags` column. Following **Option B** from the gap analysis:

- Use `team_id` and `api_key_id` as implicit tags
- This avoids schema migration and backfill requirements
- Aligns with existing RBAC model (users belong to teams, API keys belong to users)
- LiteLLM compatibility: response structure matches `/spend/tags` semantics

## Commits

1. `15f42eb G3.complement: Add /spend/tags endpoint for tag-based spend aggregation`
   - Files: `crates/godwit-api/src/admin/spend_tags.rs` (new), `crates/godwit-api/src/admin/mod.rs` (modified)
   - Changes: 376 insertions
   - Includes: endpoint implementation, RBAC, SQL queries, 7 tests

## Code Quality

- ✅ Follows existing patterns from `spend.rs` and `spend_logs.rs`
- ✅ Uses `rust_decimal::Decimal` for all cost values
- ✅ Parameterized SQL queries (no injection risk)
- ✅ Proper `sqlx::FromRow` derives for type-safe queries
- ✅ Clean separation: fetch functions, scoping function, handler
- ✅ Comprehensive test coverage (serialization, RBAC, aggregation, filtering)

## Future Enhancements (Not in MVP)

- Add explicit `tags TEXT[]` column to `request_logs` if use cases require arbitrary tagging
- Support for custom tag extraction from request metadata
- Caching layer for frequently-accessed tag aggregations
