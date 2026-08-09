# Front ↔ Backend Coverage Grid

> Single source of truth: [`contract/routes.json`](../../contract/routes.json). This table is derived from it and checked at build time by the backend contract test (`crates/godwit-api/tests/route_contract.rs`) and the frontend contract test (`apps/ui/tests/route-contract.test.ts`).

**Status:** `covered` — UI lib consumes the route (verified both sides) · `sdk-only` — exposed to SDK/proxy backend clients, not the UI · `backend-only` — admin route the UI does not currently consume.

| Scope | Method | Path | FE lib | FE fn | BE module | BE fn | Status |
|-------|--------|------|--------|-------|-----------|-------|--------|
| ui | GET | `/api/v1/admin/stats` | api.ts | fetchStats | stats.rs | get_stats | covered |
| ui | GET | `/api/v1/api-keys` | keys.ts | fetchKeys | api_keys.rs | list_api_keys | covered |
| ui | POST | `/api/v1/api-keys` | keys.ts | createKey | api_keys.rs | create_api_key | covered |
| ui | DELETE | `/api/v1/api-keys/{id}` | keys.ts | deleteKey | api_keys.rs | delete_api_key | covered |
| ui | POST | `/api/v1/api-keys/{id}/block` | keys.ts | blockKey | api_keys.rs | block_key | covered |
| ui | POST | `/api/v1/api-keys/{id}/unblock` | keys.ts | unblockKey | api_keys.rs | unblock_key | covered |
| ui | POST | `/api/v1/auth/login` | auth.ts | login | auth.rs | login | covered |
| ui | POST | `/api/v1/auth/logout` | auth.ts | logout | auth.rs | logout | covered |
| ui | GET | `/api/v1/auth/me` | auth.ts | fetchMe | mod.rs | auth::me | covered |
| ui | POST | `/api/v1/auth/refresh` | http.ts | doRefresh | auth.rs | refresh | covered |
| ui | GET | `/api/v1/models` | models.ts | fetchModels | models.rs | list_models | covered |
| ui | POST | `/api/v1/models` | models.ts | createModel | models.rs | create_model | covered |
| ui | GET | `/api/v1/provider-profiles` | providers.ts | fetchProviders | provider_profiles.rs | list_profiles | covered |
| ui | PATCH | `/api/v1/provider-profiles/{id}` | providers.ts | setProviderEnabled | provider_profiles.rs | update_profile | covered |
| ui | GET | `/api/v1/spend` | api.ts | fetchSpend | spend.rs | get_spend | covered |
| ui | GET | `/api/v1/spend/logs` | logs.ts | fetchLogs | spend_logs.rs | get_spend_logs | covered |
| ui | GET | `/api/v1/ws/metrics` | websocket.ts | MetricsSocket | metrics_ws.rs | ws_handler | covered |
| ui | GET | `/metrics` | api.ts | fetchPrometheusMetrics | metrics_endpoint.rs | metrics_handler | covered |
| uncovered | GET | `/api/v1/admin/recent-activity` |  |  | stats.rs | get_recent_activity | backend-only |
| uncovered | GET | `/api/v1/api-keys/{id}` |  |  | api_keys.rs | get_api_key | backend-only |
| uncovered | POST | `/api/v1/api-keys/{id}/regenerate` |  |  | api_keys.rs | regenerate_key | backend-only |
| uncovered | POST | `/api/v1/api-keys/{id}/reset_spend` |  |  | api_keys.rs | reset_spend | backend-only |
| uncovered | GET | `/api/v1/auth/oidc/{provider}` |  |  | auth.rs | oidc_start | backend-only |
| uncovered | GET | `/api/v1/auth/oidc/{provider}/callback` |  |  | auth.rs | oidc_callback | backend-only |
| uncovered | POST | `/api/v1/auth/saml/{provider}/acs` |  |  | auth.rs | saml_acs | backend-only |
| uncovered | POST | `/api/v1/auth/sessions/revoke-all` |  |  | auth.rs | revoke_all_sessions | backend-only |
| uncovered | GET | `/api/v1/circuit-breakers` |  |  | circuit_breakers.rs | list_circuit_breakers | backend-only |
| uncovered | GET | `/api/v1/end-users` |  |  | end_users.rs | list_end_users | backend-only |
| uncovered | POST | `/api/v1/end-users` |  |  | end_users.rs | create_end_user | backend-only |
| uncovered | GET | `/api/v1/end-users/{user_id}` |  |  | end_users.rs | get_end_user | backend-only |
| uncovered | PATCH | `/api/v1/end-users/{user_id}` |  |  | end_users.rs | update_end_user | backend-only |
| uncovered | DELETE | `/api/v1/end-users/{user_id}` |  |  | end_users.rs | delete_end_user | backend-only |
| uncovered | GET | `/api/v1/model-aliases` |  |  | model_aliases.rs | list_aliases | backend-only |
| uncovered | POST | `/api/v1/model-aliases` |  |  | model_aliases.rs | create_alias | backend-only |
| uncovered | DELETE | `/api/v1/model-aliases/{id}` |  |  | model_aliases.rs | delete_alias | backend-only |
| uncovered | GET | `/api/v1/models/{id}` |  |  | models.rs | get_model | backend-only |
| uncovered | PATCH | `/api/v1/models/{id}` |  |  | models.rs | update_model | backend-only |
| uncovered | DELETE | `/api/v1/models/{id}` |  |  | models.rs | delete_model | backend-only |
| uncovered | GET | `/api/v1/organizations` |  |  | organizations.rs | list_organizations | backend-only |
| uncovered | POST | `/api/v1/organizations` |  |  | organizations.rs | create_organization | backend-only |
| uncovered | GET | `/api/v1/organizations/{id}` |  |  | organizations.rs | get_organization | backend-only |
| uncovered | PATCH | `/api/v1/organizations/{id}` |  |  | organizations.rs | update_organization | backend-only |
| uncovered | DELETE | `/api/v1/organizations/{id}` |  |  | organizations.rs | delete_organization | backend-only |
| uncovered | POST | `/api/v1/provider-profiles` |  |  | provider_profiles.rs | create_profile | backend-only |
| uncovered | GET | `/api/v1/spend/tags` |  |  | spend_tags.rs | get_spend_tags | backend-only |
| uncovered | GET | `/api/v1/teams` |  |  | teams.rs | list_teams | backend-only |
| uncovered | POST | `/api/v1/teams` |  |  | teams.rs | create_team | backend-only |
| uncovered | GET | `/api/v1/teams/{id}` |  |  | teams.rs | get_team | backend-only |
| uncovered | PATCH | `/api/v1/teams/{id}` |  |  | teams.rs | update_team | backend-only |
| uncovered | DELETE | `/api/v1/teams/{id}` |  |  | teams.rs | delete_team | backend-only |
| uncovered | POST | `/api/v1/teams/{id}/members` |  |  | teams.rs | add_member | backend-only |
| uncovered | DELETE | `/api/v1/teams/{id}/members/{user_id}` |  |  | teams.rs | remove_member | backend-only |
| uncovered | GET | `/api/v1/users` |  |  | users.rs | list_users | backend-only |
| uncovered | POST | `/api/v1/users` |  |  | users.rs | create_user | backend-only |
| uncovered | GET | `/api/v1/users/{id}` |  |  | users.rs | get_user | backend-only |
| uncovered | PATCH | `/api/v1/users/{id}` |  |  | users.rs | update_user | backend-only |
| uncovered | DELETE | `/api/v1/users/{id}` |  |  | users.rs | delete_user | backend-only |
| uncovered | GET | `/health` |  |  | health.rs | health_check | backend-only |
| uncovered | GET | `/health/ready` |  |  | health.rs | health_ready | backend-only |
| proxy | POST | `/v1/audio/speech` |  |  | proxy.rs | audio_speech | sdk-only |
| proxy | POST | `/v1/audio/transcriptions` |  |  | proxy.rs | audio_transcriptions | sdk-only |
| proxy | POST | `/v1/batches` |  |  | proxy.rs | create_batch | sdk-only |
| proxy | GET | `/v1/batches` |  |  | proxy.rs | list_batches | sdk-only |
| proxy | DELETE | `/v1/batches` |  |  | proxy.rs | delete_batch | sdk-only |
| proxy | GET | `/v1/batches/{id}` |  |  | proxy.rs | get_batch | sdk-only |
| proxy | POST | `/v1/batches/{id}/cancel` |  |  | proxy.rs | cancel_batch | sdk-only |
| proxy | GET | `/v1/batches/{id}/results` |  |  | proxy.rs | get_batch_results | sdk-only |
| proxy | POST | `/v1/chat/completions` |  |  | proxy.rs | chat_completions | sdk-only |
| proxy | POST | `/v1/embeddings` |  |  | proxy.rs | embeddings | sdk-only |
| proxy | POST | `/v1/images/edits` |  |  | proxy.rs | image_edits | sdk-only |
| proxy | POST | `/v1/images/generations` |  |  | proxy.rs | image_generations | sdk-only |
| proxy | POST | `/v1/messages` |  |  | anthropic_proxy.rs | messages | sdk-only |
| proxy | GET | `/v1/models` |  |  | proxy.rs | list_models | sdk-only |
| proxy | POST | `/v1/moderations` |  |  | moderation.rs | moderations | sdk-only |
| proxy | POST | `/v1/rerank` |  |  | rerank.rs | rerank | sdk-only |
| proxy | GET | `/v1/utils/health` |  |  | utils.rs | health | sdk-only |
| proxy | GET | `/v1/utils/model_info/{model_id}` |  |  | utils.rs | model_info | sdk-only |
| proxy | POST | `/v1/utils/token_counter` |  |  | utils.rs | token_counter | sdk-only |

## Out of scope

- `/v1/*` proxy routes are **SDK-only** (`scope: proxy`): they are not consumed by the current UI but must stay wired for provider SDKs.
- `backend-only` routes (`/api/v1/model-aliases*`, `/api/v1/organizations*`, `/api/v1/teams*`, `/api/v1/users*`, `/api/v1/end-users*`, `/api/v1/circuit-breakers`, B2B auth/OIDC/SAML, admin tooling) are live admin/API endpoints not yet surfaced in the UI.
