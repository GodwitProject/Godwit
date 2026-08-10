# Front ↔ Backend Coverage Grid

> Single source of truth: [`contract/routes.json`](../../contract/routes.json). This table is derived from it and checked at build time by the backend contract test (`crates/godwit-api/tests/route_contract.rs`) and the frontend contract test (`apps/ui/tests/route-contract.test.ts`). The UI test asserts each `ui`-scoped route is called by the new UI.

**Status:** `covered` — new UI (`apps/ui`) lib consumes the route (verified) · `backend-only` — live backend route with no frontend consumer yet · `sdk-only` — exposed to SDK/proxy backend clients, not a frontend.

| Scope | Method | Path | FE lib | FE fn | BE module | BE fn | Status |
|-------|--------|------|--------|-------|-----------|-------|--------|
| ui | POST | `/api/v1/auth/login` | apps/ui/src/lib/auth.ts | login | crates/godwit-api/src/admin/auth.rs | login | covered |
| backend-only | POST | `/api/v1/auth/refresh` |  |  | crates/godwit-api/src/admin/auth.rs | refresh | backend-only |
| ui | POST | `/api/v1/auth/logout` | apps/ui/src/lib/auth.ts | logout | crates/godwit-api/src/admin/auth.rs | logout | covered |
| ui | GET | `/api/v1/auth/me` | apps/ui/src/lib/auth.ts | fetchMe | crates/godwit-api/src/admin/mod.rs | auth::me | covered |
| backend-only | POST | `/api/v1/auth/sessions/revoke-all` |  |  | crates/godwit-api/src/admin/auth.rs | revoke_all_sessions | backend-only |
| backend-only | GET | `/api/v1/auth/oidc/{provider}` |  |  | crates/godwit-api/src/admin/auth.rs | oidc_start | backend-only |
| backend-only | GET | `/api/v1/auth/oidc/{provider}/callback` |  |  | crates/godwit-api/src/admin/auth.rs | oidc_callback | backend-only |
| backend-only | POST | `/api/v1/auth/saml/{provider}/acs` |  |  | crates/godwit-api/src/admin/auth.rs | saml_acs | backend-only |
| backend-only | POST | `/api/v1/auth/change-password` |  |  | crates/godwit-api/src/admin/password.rs | change_password | backend-only |
| backend-only | POST | `/api/v1/auth/change-required` |  |  | crates/godwit-api/src/admin/password.rs | change_required | backend-only |
| backend-only | POST | `/api/v1/auth/admin/reset-password` |  |  | crates/godwit-api/src/admin/password.rs | admin_reset_password | backend-only |
| backend-only | GET | `/api/v1/api-keys` |  |  | crates/godwit-api/src/admin/api_keys.rs | list_api_keys | backend-only |
| backend-only | POST | `/api/v1/api-keys` |  |  | crates/godwit-api/src/admin/api_keys.rs | create_api_key | backend-only |
| backend-only | GET | `/api/v1/api-keys/{id}` |  |  | crates/godwit-api/src/admin/api_keys.rs | get_api_key | backend-only |
| backend-only | POST | `/api/v1/api-keys/{id}/block` |  |  | crates/godwit-api/src/admin/api_keys.rs | block_key | backend-only |
| backend-only | POST | `/api/v1/api-keys/{id}/unblock` |  |  | crates/godwit-api/src/admin/api_keys.rs | unblock_key | backend-only |
| backend-only | POST | `/api/v1/api-keys/{id}/regenerate` |  |  | crates/godwit-api/src/admin/api_keys.rs | regenerate_key | backend-only |
| backend-only | POST | `/api/v1/api-keys/{id}/reset_spend` |  |  | crates/godwit-api/src/admin/api_keys.rs | reset_spend | backend-only |
| backend-only | DELETE | `/api/v1/api-keys/{id}` |  |  | crates/godwit-api/src/admin/api_keys.rs | delete_api_key | backend-only |
| ui | GET | `/api/v1/models` | apps/ui/src/lib/models.ts | listModels | crates/godwit-api/src/admin/models.rs | list_models | covered |
| ui | POST | `/api/v1/models` | apps/ui/src/lib/models.ts | createModel | crates/godwit-api/src/admin/models.rs | create_model | covered |
| backend-only | GET | `/api/v1/models/{id}` |  |  | crates/godwit-api/src/admin/models.rs | get_model | backend-only |
| ui | PATCH | `/api/v1/models/{id}` | apps/ui/src/lib/models.ts | updateModel | crates/godwit-api/src/admin/models.rs | update_model | covered |
| ui | DELETE | `/api/v1/models/{id}` | apps/ui/src/lib/models.ts | deleteModel | crates/godwit-api/src/admin/models.rs | delete_model | covered |
| ui | GET | `/api/v1/provider-profiles` | apps/ui/src/lib/providerProfiles.ts | listProviderProfiles | crates/godwit-api/src/admin/provider_profiles.rs | list_profiles | covered |
| ui | POST | `/api/v1/provider-profiles` | apps/ui/src/lib/providerProfiles.ts | createProviderProfile | crates/godwit-api/src/admin/provider_profiles.rs | create_profile | covered |
| ui | PATCH | `/api/v1/provider-profiles/{id}` | apps/ui/src/lib/providerProfiles.ts | updateProviderProfile | crates/godwit-api/src/admin/provider_profiles.rs | update_profile | covered |
| ui | DELETE | `/api/v1/provider-profiles/{id}` | apps/ui/src/lib/providerProfiles.ts | deleteProviderProfile | crates/godwit-api/src/admin/provider_profiles.rs | delete_profile | covered |
| backend-only | GET | `/api/v1/organizations` |  |  | crates/godwit-api/src/admin/organizations.rs | list_organizations | backend-only |
| backend-only | POST | `/api/v1/organizations` |  |  | crates/godwit-api/src/admin/organizations.rs | create_organization | backend-only |
| backend-only | GET | `/api/v1/organizations/{id}` |  |  | crates/godwit-api/src/admin/organizations.rs | get_organization | backend-only |
| backend-only | PATCH | `/api/v1/organizations/{id}` |  |  | crates/godwit-api/src/admin/organizations.rs | update_organization | backend-only |
| backend-only | DELETE | `/api/v1/organizations/{id}` |  |  | crates/godwit-api/src/admin/organizations.rs | delete_organization | backend-only |
| backend-only | GET | `/api/v1/teams` |  |  | crates/godwit-api/src/admin/teams.rs | list_teams | backend-only |
| backend-only | POST | `/api/v1/teams` |  |  | crates/godwit-api/src/admin/teams.rs | create_team | backend-only |
| backend-only | GET | `/api/v1/teams/{id}` |  |  | crates/godwit-api/src/admin/teams.rs | get_team | backend-only |
| backend-only | PATCH | `/api/v1/teams/{id}` |  |  | crates/godwit-api/src/admin/teams.rs | update_team | backend-only |
| backend-only | DELETE | `/api/v1/teams/{id}` |  |  | crates/godwit-api/src/admin/teams.rs | delete_team | backend-only |
| backend-only | POST | `/api/v1/teams/{id}/members` |  |  | crates/godwit-api/src/admin/teams.rs | add_member | backend-only |
| backend-only | DELETE | `/api/v1/teams/{id}/members/{user_id}` |  |  | crates/godwit-api/src/admin/teams.rs | remove_member | backend-only |
| backend-only | GET | `/api/v1/users` |  |  | crates/godwit-api/src/admin/users.rs | list_users | backend-only |
| backend-only | POST | `/api/v1/users` |  |  | crates/godwit-api/src/admin/users.rs | create_user | backend-only |
| backend-only | GET | `/api/v1/users/{id}` |  |  | crates/godwit-api/src/admin/users.rs | get_user | backend-only |
| backend-only | PATCH | `/api/v1/users/{id}` |  |  | crates/godwit-api/src/admin/users.rs | update_user | backend-only |
| backend-only | DELETE | `/api/v1/users/{id}` |  |  | crates/godwit-api/src/admin/users.rs | delete_user | backend-only |
| backend-only | GET | `/api/v1/end-users` |  |  | crates/godwit-api/src/admin/end_users.rs | list_end_users | backend-only |
| backend-only | POST | `/api/v1/end-users` |  |  | crates/godwit-api/src/admin/end_users.rs | create_end_user | backend-only |
| backend-only | GET | `/api/v1/end-users/{user_id}` |  |  | crates/godwit-api/src/admin/end_users.rs | get_end_user | backend-only |
| backend-only | PATCH | `/api/v1/end-users/{user_id}` |  |  | crates/godwit-api/src/admin/end_users.rs | update_end_user | backend-only |
| backend-only | DELETE | `/api/v1/end-users/{user_id}` |  |  | crates/godwit-api/src/admin/end_users.rs | delete_end_user | backend-only |
| backend-only | GET | `/api/v1/model-aliases` |  |  | crates/godwit-api/src/admin/model_aliases.rs | list_aliases | backend-only |
| backend-only | POST | `/api/v1/model-aliases` |  |  | crates/godwit-api/src/admin/model_aliases.rs | create_alias | backend-only |
| backend-only | DELETE | `/api/v1/model-aliases/{id}` |  |  | crates/godwit-api/src/admin/model_aliases.rs | delete_alias | backend-only |
| backend-only | GET | `/api/v1/spend` |  |  | crates/godwit-api/src/admin/spend.rs | get_spend | backend-only |
| backend-only | GET | `/api/v1/spend/logs` |  |  | crates/godwit-api/src/admin/spend_logs.rs | get_spend_logs | backend-only |
| backend-only | GET | `/api/v1/spend/tags` |  |  | crates/godwit-api/src/admin/spend_tags.rs | get_spend_tags | backend-only |
| backend-only | GET | `/api/v1/admin/stats` |  |  | crates/godwit-api/src/admin/stats.rs | get_stats | backend-only |
| backend-only | GET | `/api/v1/admin/recent-activity` |  |  | crates/godwit-api/src/admin/stats.rs | get_recent_activity | backend-only |
| backend-only | GET | `/api/v1/circuit-breakers` |  |  | crates/godwit-api/src/admin/circuit_breakers.rs | list_circuit_breakers | backend-only |
| backend-only | GET | `/health` |  |  | crates/godwit-api/src/health.rs | health_check | backend-only |
| backend-only | GET | `/health/ready` |  |  | crates/godwit-api/src/health.rs | health_ready | backend-only |
| backend-only | GET | `/metrics` |  |  | crates/godwit-api/src/metrics_endpoint.rs | metrics_handler | backend-only |
| backend-only | GET | `/api/v1/ws/metrics` |  |  | crates/godwit-api/src/admin/metrics_ws.rs | ws_handler | backend-only |
| proxy | POST | `/v1/chat/completions` |  |  | crates/godwit-api/src/proxy.rs | chat_completions | sdk-only |
| proxy | POST | `/v1/messages` |  |  | crates/godwit-api/src/anthropic_proxy.rs | messages | sdk-only |
| proxy | POST | `/v1/embeddings` |  |  | crates/godwit-api/src/proxy.rs | embeddings | sdk-only |
| proxy | GET | `/v1/models` |  |  | crates/godwit-api/src/proxy.rs | list_models | sdk-only |
| proxy | POST | `/v1/images/generations` |  |  | crates/godwit-api/src/proxy.rs | image_generations | sdk-only |
| proxy | POST | `/v1/images/edits` |  |  | crates/godwit-api/src/proxy.rs | image_edits | sdk-only |
| proxy | POST | `/v1/audio/speech` |  |  | crates/godwit-api/src/proxy.rs | audio_speech | sdk-only |
| proxy | POST | `/v1/audio/transcriptions` |  |  | crates/godwit-api/src/proxy.rs | audio_transcriptions | sdk-only |
| proxy | POST | `/v1/batches` |  |  | crates/godwit-api/src/proxy.rs | create_batch | sdk-only |
| proxy | GET | `/v1/batches` |  |  | crates/godwit-api/src/proxy.rs | list_batches | sdk-only |
| proxy | DELETE | `/v1/batches` |  |  | crates/godwit-api/src/proxy.rs | delete_batch | sdk-only |
| proxy | GET | `/v1/batches/{id}` |  |  | crates/godwit-api/src/proxy.rs | get_batch | sdk-only |
| proxy | POST | `/v1/batches/{id}/cancel` |  |  | crates/godwit-api/src/proxy.rs | cancel_batch | sdk-only |
| proxy | GET | `/v1/batches/{id}/results` |  |  | crates/godwit-api/src/proxy.rs | get_batch_results | sdk-only |
| proxy | POST | `/v1/moderations` |  |  | crates/godwit-api/src/moderation.rs | moderations | sdk-only |
| proxy | POST | `/v1/rerank` |  |  | crates/godwit-api/src/rerank.rs | rerank | sdk-only |
| proxy | POST | `/v1/utils/token_counter` |  |  | crates/godwit-api/src/utils.rs | token_counter | sdk-only |
| proxy | GET | `/v1/utils/model_info/{model_id}` |  |  | crates/godwit-api/src/utils.rs | model_info | sdk-only |
| proxy | GET | `/v1/utils/health` |  |  | crates/godwit-api/src/utils.rs | health | sdk-only |

## Notes

- **`covered`** = new UI (`apps/ui`) consumes the route; **`backend-only`** = live backend endpoint not yet surfaced in the new UI; **`sdk-only`** = proxy/SDK surface.
- The legacy `apps/admin` console has been removed; routes that were previously `admin-covered` are now `backend-only` until the new UI implements those features.
- `/v1/*` proxy routes remain **SDK-only** (`scope: proxy`): not consumed by any frontend but required for provider SDK compatibility.
