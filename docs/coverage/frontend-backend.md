# Front ↔ Backend Coverage Grid

> Single source of truth: [`contract/routes.json`](../../contract/routes.json). This table is derived from it and checked at build time by the backend contract test (`crates/godwit-api/tests/route_contract.rs`) and the frontend contract tests (`apps/ui/tests/route-contract.test.ts` and `apps/admin/tests/route-contract.test.ts`). The UI test asserts each `ui`-scoped route is called by the new UI; the admin test asserts every backend call made by the legacy admin console is declared in the contract.

**Status:** `covered` — new UI (`apps/ui`) lib consumes the route (verified) · `admin-covered` — legacy admin console (`apps/admin`) server actions consume the route (verified) · `sdk-only` — exposed to SDK/proxy backend clients, not a frontend · `backend-only` — live backend route with no frontend consumer yet.

| Scope | Method | Path | FE lib | FE fn | BE module | BE fn | Status |
|-------|--------|------|--------|-------|-----------|-------|--------|
| ui | GET | `/metrics` | api.ts | fetchPrometheusMetrics | metrics_endpoint.rs | metrics_handler | covered |
| ui | GET | `/api/v1/admin/stats` | api.ts | fetchStats | stats.rs | get_stats | covered |
| ui | GET | `/api/v1/api-keys` | keys.ts | fetchKeys | api_keys.rs | list_api_keys | covered |
| ui | POST | `/api/v1/api-keys` | keys.ts | createKey | api_keys.rs | create_api_key | covered |
| ui | DELETE | `/api/v1/api-keys/{id}` | keys.ts | deleteKey | api_keys.rs | delete_api_key | covered |
| ui | POST | `/api/v1/api-keys/{id}/block` | keys.ts | blockKey | api_keys.rs | block_key | covered |
| ui | POST | `/api/v1/api-keys/{id}/unblock` | keys.ts | unblockKey | api_keys.rs | unblock_key | covered |
| ui | POST | `/api/v1/auth/login` | auth.ts | login | auth.rs | login | covered |
| ui | POST | `/api/v1/auth/logout` | auth.ts | logout | auth.rs | logout | covered |
| ui | GET | `/api/v1/auth/me` | auth.ts | fetchMe | mod.rs | auth::me | covered |
| ui | POST | `/api/v1/auth/change-password` | auth.ts | changePassword | password.rs | change_password | covered |
| ui | POST | `/api/v1/auth/change-required` | auth.ts | changeRequired | password.rs | change_required | covered |
| ui | POST | `/api/v1/auth/refresh` | http.ts | doRefresh | auth.rs | refresh | covered |
| ui | GET | `/api/v1/models` | models.ts | fetchModels | models.rs | list_models | covered |
| ui | POST | `/api/v1/models` | models.ts | createModel | models.rs | create_model | covered |
| ui | GET | `/api/v1/provider-profiles` | providers.ts | fetchProviders | provider_profiles.rs | list_profiles | covered |
| ui | PATCH | `/api/v1/provider-profiles/{id}` | providers.ts | setProviderEnabled | provider_profiles.rs | update_profile | covered |
| ui | GET | `/api/v1/spend` | api.ts | fetchSpend | spend.rs | get_spend | covered |
| ui | GET | `/api/v1/spend/logs` | logs.ts | fetchLogs | spend_logs.rs | get_spend_logs | covered |
| ui | GET | `/api/v1/ws/metrics` | websocket.ts | MetricsSocket | metrics_ws.rs | ws_handler | covered |
| admin | GET | `/api/v1/admin/recent-activity` | admin/page.tsx | DashboardPage | stats.rs | get_recent_activity | admin-covered |
| admin | GET | `/api/v1/api-keys/{id}` | api-keys/actions.ts | getApiKey | api_keys.rs | get_api_key | admin-covered |
| admin | GET | `/api/v1/auth/oidc/{provider}` | login/actions.ts | loginWithSSO | auth.rs | oidc_start | admin-covered |
| admin | GET | `/api/v1/auth/oidc/{provider}/callback` | auth/callback/actions.ts | exchangeOIDCCode | auth.rs | oidc_callback | admin-covered |
| admin | POST | `/api/v1/auth/forgot-password` | forgot-password/actions.ts | requestPasswordReset | password.rs | forgot_password | admin-covered |
| admin | POST | `/api/v1/auth/reset-password` | reset-password/actions.ts | performPasswordReset | password.rs | reset_password | admin-covered |
| admin | POST | `/api/v1/auth/admin/reset-password` | users/actions.ts | resetUserPassword | password.rs | admin_reset_password | admin-covered |
| admin | GET | `/api/v1/models/{id}` | models/actions.ts | getModel | models.rs | get_model | admin-covered |
| admin | PATCH | `/api/v1/models/{id}` | models/actions.ts | updateModel | models.rs | update_model | admin-covered |
| admin | DELETE | `/api/v1/models/{id}` | models/actions.ts | deleteModel | models.rs | delete_model | admin-covered |
| admin | GET | `/api/v1/organizations` | organizations/actions.ts | listOrganizations | organizations.rs | list_organizations | admin-covered |
| admin | POST | `/api/v1/organizations` | organizations/actions.ts | createOrganization | organizations.rs | create_organization | admin-covered |
| admin | GET | `/api/v1/organizations/{id}` | organizations/actions.ts | getOrganization | organizations.rs | get_organization | admin-covered |
| admin | PATCH | `/api/v1/organizations/{id}` | organizations/actions.ts | updateOrganization | organizations.rs | update_organization | admin-covered |
| admin | DELETE | `/api/v1/organizations/{id}` | organizations/actions.ts | deleteOrganization | organizations.rs | delete_organization | admin-covered |
| admin | GET | `/api/v1/teams` | teams/actions.ts | listTeams | teams.rs | list_teams | admin-covered |
| admin | POST | `/api/v1/teams` | teams/actions.ts | createTeam | teams.rs | create_team | admin-covered |
| admin | GET | `/api/v1/teams/{id}` | teams/actions.ts | getTeam | teams.rs | get_team | admin-covered |
| admin | PATCH | `/api/v1/teams/{id}` | teams/actions.ts | updateTeam | teams.rs | update_team | admin-covered |
| admin | DELETE | `/api/v1/teams/{id}` | teams/actions.ts | deleteTeam | teams.rs | delete_team | admin-covered |
| admin | GET | `/api/v1/users` | users/actions.ts | listUsers | users.rs | list_users | admin-covered |
| admin | POST | `/api/v1/users` | users/actions.ts | createUser | users.rs | create_user | admin-covered |
| admin | GET | `/api/v1/users/{id}` | users/actions.ts | getUser | users.rs | get_user | admin-covered |
| admin | PATCH | `/api/v1/users/{id}` | users/actions.ts | updateUser | users.rs | update_user | admin-covered |
| admin | DELETE | `/api/v1/users/{id}` | users/actions.ts | deleteUser | users.rs | delete_user | admin-covered |
| backend-only | GET | `/health` |  |  | health.rs | health_check | backend-only |
| backend-only | GET | `/health/ready` |  |  | health.rs | health_ready | backend-only |
| backend-only | GET | `/api/v1/api-keys/{id}/regenerate` |  |  | api_keys.rs | regenerate_key | backend-only |
| backend-only | POST | `/api/v1/api-keys/{id}/reset_spend` |  |  | api_keys.rs | reset_spend | backend-only |
| backend-only | POST | `/api/v1/auth/saml/{provider}/acs` |  |  | auth.rs | saml_acs | backend-only |
| backend-only | POST | `/api/v1/auth/sessions/revoke-all` |  |  | auth.rs | revoke_all_sessions | backend-only |
| backend-only | GET | `/api/v1/circuit-breakers` |  |  | circuit_breakers.rs | list_circuit_breakers | backend-only |
| backend-only | GET | `/api/v1/end-users` |  |  | end_users.rs | list_end_users | backend-only |
| backend-only | POST | `/api/v1/end-users` |  |  | end_users.rs | create_end_user | backend-only |
| backend-only | GET | `/api/v1/end-users/{user_id}` |  |  | end_users.rs | get_end_user | backend-only |
| backend-only | PATCH | `/api/v1/end-users/{user_id}` |  |  | end_users.rs | update_end_user | backend-only |
| backend-only | DELETE | `/api/v1/end-users/{user_id}` |  |  | end_users.rs | delete_end_user | backend-only |
| backend-only | GET | `/api/v1/model-aliases` |  |  | model_aliases.rs | list_aliases | backend-only |
| backend-only | POST | `/api/v1/model-aliases` |  |  | model_aliases.rs | create_alias | backend-only |
| backend-only | DELETE | `/api/v1/model-aliases/{id}` |  |  | model_aliases.rs | delete_alias | backend-only |
| backend-only | POST | `/api/v1/provider-profiles` |  |  | provider_profiles.rs | create_profile | backend-only |
| backend-only | GET | `/api/v1/spend/tags` |  |  | spend_tags.rs | get_spend_tags | backend-only |
| backend-only | POST | `/api/v1/teams/{id}/members` |  |  | teams.rs | add_member | backend-only |
| backend-only | DELETE | `/api/v1/teams/{id}/members/{user_id}` |  |  | teams.rs | remove_member | backend-only |
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

## Notes

- **`covered`** = new UI (`apps/ui`) consumes the route; **`admin-covered`** = legacy admin console (`apps/admin`) consumes it. The two frontends share some routes (`auth/login`, `auth/logout`, `auth/refresh`, `models.list/create`, `api-keys.list/create/delete`, `spend.total`, `admin.stats`); these are tagged by the consumer that defined the contract entry.
- OIDC SSO: `apps/admin` now targets the real backend routes (`GET /api/v1/auth/oidc/{provider}` and `GET .../callback`). The backend owns the code exchange and issues the token pair (HttpOnly cookies + JSON body); the admin stores the returned bearer tokens. The IdP's provider `redirect_uri` must point at the admin `/auth/callback` page for the exchange to complete. This is the known, partially-wired follow-up from the auth-hardening design.
- The new UI (`apps/ui`) keys form still uses a hard-coded `MOCK_MODELS` list for model selection instead of `/api/v1/models`; the admin console fetches real models.

## Out of scope

- `/v1/*` proxy routes are **SDK-only** (`scope: proxy`): not consumed by any frontend but must stay wired for provider SDKs.
- `backend-only` routes (health, `/api/v1/circuit-breakers`, `/api/v1/end-users*`, `/api/v1/model-aliases*`, `api-keys.regenerate/reset_spend`, `provider-profiles.create`, `spend.tags`, team member add/remove, SAML ACS, session revoke-all) are live backend endpoints not yet surfaced in any frontend.
