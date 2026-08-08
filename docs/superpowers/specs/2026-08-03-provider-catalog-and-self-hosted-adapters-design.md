# Instance-Wide Provider Catalog & Self-Hosted Adapters — Design Specification

## 1. Goal

Extend Godwit's provider layer in two directions inspired by LiteLLM:

1. An **instance-wide** model/provider catalog, managed by the `super_admin` role, that supports both explicit catalog entries (fixed pricing/capabilities) and **wildcard passthrough** routing (`openai/*`, `mistralai/*`-style — any upstream model works once a provider profile exists, no per-model row required).
2. Dedicated adapters for four **self-hosted inference backends**: vllm, sglang, llama.cpp, and ollama, alongside the existing OpenAI, Anthropic, and Gemini adapters.
3. Complete the proxy's multimodal HTTP surface: the OpenAI adapter already implements image generation, audio TTS/STT, and embeddings, but none of these are reachable today — only `/v1/chat/completions` and `/v1/models` are wired up. This design adds the missing routes and a new `ImageEdit` capability.

Provider credentials move from static `config.yaml` entries to database-backed, admin-manageable, encrypted-at-rest `provider_profiles`, so operators can add/rotate a provider without redeploying.

**Explicitly out of scope:** per-organization or per-team credential overrides (BYOK). Organizations and teams remain purely an internal grouping for users, API keys, budgets, and spend attribution — they have no bearing on which models/providers exist or which credentials are used. There is exactly one catalog and one set of credentials per protocol for the whole instance.

## 2. Current State (as of this design)

- `provider_profiles` and `models` are both scoped by `organization_id`; every organization maintains its own duplicate catalog.
- Provider credentials are static, loaded once at startup from `config.yaml` (`AppConfig.providers.{openai,anthropic,gemini}`), and baked into each `Adapter` struct at construction (`OpenAiProvider::new(api_key, base_url)`). The `&ProviderProfile` parameter threaded through every `Adapter` trait method is unused (`_profile`) — profile-based credentials were never wired up.
- Only three adapters exist: OpenAI, Anthropic, Gemini. `godwit_core::Protocol` already declares constructors for `vllm`, `sglang`, `llama_cpp`, `ollama`, `azure_openai`, `bedrock`, `cohere`, `mistral`, `groq`, `together`, but no adapters implement them.
- Model routing (`DbModelRouter::resolve`) requires an admin-created `Model` row (`public_id` + `provider_model_id`) for every exposed upstream model; ambiguity between providers offering the same `public_id` is resolved via a `profile_name/public_id` prefix. There is no passthrough mode.
- The admin API exposes `GET /api/v1/models` only — no create/update/delete for models, and no endpoints at all for `provider_profiles`.
- Only `/v1/models` and `/v1/chat/completions` are exposed as proxy routes. The OpenAI adapter already implements `image_generation`, `audio_tts`, `audio_stt`, and `embedding` (see `supported_capabilities()` in `openai.rs`), but none of them have an HTTP route — they are unreachable dead code today. Anthropic and Gemini adapters currently declare `Chat` only. There is no `ImageEdit` capability anywhere in the system.
- `config.yaml`'s `${VAR}` placeholders (e.g. `"${OPENAI_API_KEY}"`) are never substituted by any code path — a pre-existing gap, unrelated to this design, left as-is.

## 3. Scope

### In scope
- Drop `organization_id` from `provider_profiles` and `models`; both become instance-wide, `super_admin`-managed resources.
- Encrypt provider credentials at rest (AES-256-GCM) and admin-manage them via a new REST API instead of static config.
- Refactor `Adapter` implementations to be stateless (shared `reqwest::Client` only); credentials and base URL are resolved once per request into a `ResolvedProfile` and passed to every adapter call.
- Add `allow_wildcard` passthrough routing on `provider_profiles`, coexisting with the explicit catalog.
- Implement four new adapters — vllm, sglang, llama.cpp, ollama — each in its own module, covering chat, chat streaming, and embeddings (the only capabilities these engines actually expose today).
- Extend the admin API: full CRUD for `provider_profiles` and `models`.
- Add the missing multimodal proxy routes so already-implemented (and newly implemented) capabilities are actually reachable: `/v1/embeddings`, `/v1/images/generations`, `/v1/images/edits`, `/v1/audio/speech`, `/v1/audio/transcriptions` (see §9).
- Add a new `ImageEdit` capability (core enum + DB check constraint) and implement `Adapter::image_edit` for the OpenAI adapter — the only backend among the seven whose real API supports it.
- One-time startup bootstrap that seeds `provider_profiles` from `config.yaml` if the table is empty, and a migration that drops `organization_id` with naive de-duplication (first row per name/public_id wins — acceptable pre-production).

### Out of scope
- BYOK / per-organization or per-team credential overrides.
- Adapters for azure_openai, bedrock, cohere, mistral, groq, together (the `Protocol` constructors remain unused placeholders for future work).
- Image generation/edit, audio TTS/STT for the four new self-hosted adapters (vllm/sglang/llama.cpp/ollama) — none of these engines expose such endpoints today. This is unrelated to the route additions above, which only make the *existing* OpenAI capabilities reachable.
- `ImageEdit` for Anthropic, Gemini, or any of the four self-hosted adapters — none support it; they return `CapabilityNotSupported`.
- Automatic per-token pricing for wildcard-resolved requests (`cost_usd` stays `NULL` for passthrough calls; only explicit catalog entries get cost tracking).
- Master-key rotation tooling for the encryption-at-rest key itself.
- Fixing the pre-existing `${VAR}` config interpolation gap.

## 4. Data Model Changes

```sql
-- provider_profiles: drop per-org scoping, add wildcard flag
ALTER TABLE provider_profiles
    DROP COLUMN organization_id,
    ADD COLUMN allow_wildcard BOOLEAN NOT NULL DEFAULT false;
-- unique constraint becomes (name) instead of (organization_id, name)
-- auth JSONB now always holds {"ciphertext": "<base64>", "nonce": "<base64>"} or is absent/null

-- models: drop per-org scoping
ALTER TABLE models
    DROP COLUMN organization_id;
-- unique constraint becomes (provider_profile_id, public_id) instead of (organization_id, public_id)
```

Migration strategy: since no production tenants exist yet, de-duplicate naively — for each distinct `name` (profiles) / `(provider_profile_id, public_id)` (models), keep the first row encountered and drop the rest, then apply the `DROP COLUMN` / constraint changes.

`request_logs` is unaffected: `model`, `provider`, `provider_model_id` are already free-text columns with no foreign key to `models`, so logging and spend attribution for both catalog and wildcard-resolved requests continue to work unchanged, still keyed by the calling `api_key`'s `organization_id`/`team_id`.

## 5. Credential Encryption

New module `godwit-auth::credentials`:

```rust
pub struct EncryptedSecret { pub ciphertext: Vec<u8>, pub nonce: [u8; 12] }
pub fn encrypt_api_key(master_key: &[u8; 32], plaintext: &str) -> EncryptedSecret;
pub fn decrypt_api_key(master_key: &[u8; 32], secret: &EncryptedSecret) -> Result<String, PasteurError>;
```

- The 32-byte master key is read from the `CREDENTIAL_ENCRYPTION_KEY` environment variable (base64-encoded) at startup and held in `AppState`. It is independent of `config.yaml`'s (non-functional) `${VAR}` interpolation.
- The admin API accepts a plaintext `api_key` on create/update, encrypts it before persisting, and never returns plaintext or ciphertext back — read endpoints return `has_credentials: bool` only.
- `ResolvedProfile` (see below) is never `Serialize` and has a custom `Debug` impl that redacts the key (`api_key: Some("***redacted***")`), to prevent accidental leakage via tracing/logging.

## 6. Adapter & Router Architecture

```rust
pub struct ResolvedProfile {
    pub base_url: String,
    pub api_key: Option<String>, // plaintext, decrypted; None if the profile has no credentials configured
}
```

- All `Adapter` implementations (existing: OpenAI, Anthropic, Gemini; new: vllm, sglang, llama.cpp, ollama) drop their `api_key`/`base_url` fields and `new(api_key, base_url)` constructors. Each keeps only a shared `reqwest::Client` (connection pooling), built once at startup with no `new()` arguments.
- Every `Adapter` trait method's `profile: &ProviderProfile` parameter becomes `profile: &ResolvedProfile`, and is actually used now (`format!("{}/chat/completions", profile.base_url)`, optional `Authorization` header only `if let Some(key) = &profile.api_key`).
- `AdapterRegistry` is unchanged in shape (protocol → `Arc<dyn Adapter>`), just built with credential-free constructors.
- `DbModelRouter::resolve(model_ref: &str, requested_capability: Capability) -> Result<ResolvedModel, PasteurError>` (no `organization_id` parameter — the catalog is global):
  1. Parse `model_ref` into `(profile_name: Option<&str>, suffix: &str)` by splitting on `/`.
  2. **With a profile prefix:** look up the `ProviderProfile` by name (global). Look for a catalog `Model` row matching `provider_profile_id` + `public_id == suffix`.
     - Found → resolve normally; validate `requested_capability` against the model's declared `capabilities` (existing behavior).
     - Not found, and `profile.allow_wildcard` → synthesize an in-memory (non-persisted) `Model` with `public_id = model_ref`, `provider_model_id = suffix`, `capabilities = vec![requested_capability]` (trust the caller/upstream; no declared restriction is possible without a catalog row).
     - Not found, not wildcard → `PasteurError::NotFound`.
  3. **Without a prefix:** catalog-only lookup by bare `public_id` across the whole instance; zero matches → `NotFound`, one match → resolve, multiple matches → `PasteurError::Validation("ambiguous model ...; use 'profile_name/...'")` (existing behavior, now instance-wide instead of per-org).
  4. Decrypt the resolved profile's `auth` into a `ResolvedProfile`. If the profile has no stored credentials, error `PasteurError::Provider("no credentials configured for protocol {protocol}")` — no fallback layer once the DB bootstrap has run.

## 7. New Self-Hosted Adapters

Four new modules in `godwit-providers`: `vllm.rs`, `sglang.rs`, `llama_cpp.rs`, `ollama.rs`, each its own `Adapter` implementation (deliberately not sharing a generic base, to allow future divergence). `supported_capabilities()` returns `[Chat, Embedding]` for all four — this matches what these engines' OpenAI-compatible HTTP APIs actually expose today (`/v1/chat/completions` with streaming, `/v1/embeddings`); none expose image generation/edit, TTS, or STT. `chat`, `chat_stream`, and `embedding` are implemented following the same request/response mapping pattern as the existing `openai.rs` adapter (these engines' APIs are themselves OpenAI-compatible); `image_generation`, `video_generation`, `audio_tts`, `audio_stt` return `ProviderError::CapabilityNotSupported`, consistent with the existing pattern used when a model doesn't declare a capability.

Each is registered in `AdapterRegistry` under its own `Protocol` (already defined in `godwit-core`): `Protocol::vllm()`, `Protocol::sglang()`, `Protocol::llama_cpp()`, `Protocol::ollama()`.

This section is unaffected by the `ImageEdit`/route work in §8-§9 below: none of the four self-hosted engines gain new capabilities here, they stay at `[Chat, Embedding]`.

## 8. New Capability: `ImageEdit`

- `godwit_core::Capability` gains an `ImageEdit` variant (`as_str`/`from_str`/`Display` updated, same pattern as the other five variants).
- Migration extends the `chk_models_capabilities` check constraint: `CHECK (capabilities <@ ARRAY['chat','image_generation','video_generation','audio_tts','audio_stt','embedding','image_edit'])`.
- `Adapter` trait gains `image_edit`, following the existing `audio_stt` pattern for file uploads:
  ```rust
  async fn image_edit(
      &self,
      profile: &ResolvedProfile,
      model: &Model,
      request: ImageEditRequest, // prompt, n, size, response_format
      image_bytes: Vec<u8>,
      image_filename: String,
      mask_bytes: Option<Vec<u8>>,
  ) -> Result<(ProviderResponse, UsageReport), ProviderError>;
  ```
- Only the OpenAI adapter implements it for real (calling `POST {base_url}/images/edits`, multipart form). Anthropic, Gemini, and the four self-hosted adapters return `ProviderError::CapabilityNotSupported`.

## 9. Proxy Route Additions

All five routes follow the same shape as `/v1/chat/completions`: API-key auth, resolve the model via `DbModelRouter::resolve(model_ref, <capability>)`, call the matching `Adapter` method, log to `request_logs` the same way (`cost_usd` computed only where `compute_cost` already has a pricing formula for that capability; otherwise logged with `cost_usd = NULL`, matching today's behavior for capabilities without a pricing formula).

| Method | Path | Capability | Notes |
|--------|------|------------|-------|
| POST | `/v1/embeddings` | `Embedding` | |
| POST | `/v1/images/generations` | `ImageGeneration` | |
| POST | `/v1/images/edits` | `ImageEdit` | multipart/form-data (image, optional mask, prompt) — mirrors `audio_stt`'s existing file-upload handling in `proxy.rs` |
| POST | `/v1/audio/speech` | `AudioTts` | |
| POST | `/v1/audio/transcriptions` | `AudioStt` | multipart/form-data |

## 10. Admin API

All new/extended routes require `super_admin` (the catalog is now shared instance-wide infrastructure — `org_admin`/`team_admin` retain control only over their own org's users/teams/api-keys/budgets, unchanged):

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/v1/provider-profiles` | List; credentials never returned, only `has_credentials: bool` |
| POST | `/api/v1/provider-profiles` | Create: `name`, `protocol`, `base_url`, optional `api_key` (plaintext in, encrypted at rest), `allow_wildcard` |
| PATCH | `/api/v1/provider-profiles/{id}` | Update any field above, plus `enabled` |
| POST | `/api/v1/models` | Create catalog entry: `public_id`, `provider_profile_id`, `provider_model_id`, `capabilities`, `pricing` |
| PATCH | `/api/v1/models/{id}` | Update |
| DELETE | `/api/v1/models/{id}` | Delete |

No `DELETE /provider-profiles/{id}` in v1 — disable via `enabled: false` instead, to avoid orphaning `models` rows that reference it.

## 11. Startup Bootstrap & Config Deprecation

`AppConfig.providers` (static `openai`/`anthropic`/`gemini` config) is removed from the config schema. On startup, `godwit-bin` runs a one-time bootstrap: if `provider_profiles` is empty and a legacy `config.yaml` still has provider entries (transitional support during the upgrade window), it creates the equivalent `provider_profiles` rows, encrypting each key with the new master key. After this runs once, `provider_profiles` in the database is the sole source of truth going forward.

## 12. Testing Strategy

- **Unit:** AES-256-GCM round-trip (encrypt/decrypt, tamper detection), `ResolvedProfile`'s redacted `Debug` output, wildcard resolution branches (catalog hit / catalog miss + wildcard / catalog miss + no wildcard / ambiguous / not found), request/response mapping for each of the four new adapters (wiremock-based, mirroring existing `openai.rs`/`anthropic.rs`/`gemini.rs` test patterns), `CapabilityNotSupported` for unimplemented capabilities on the new adapters, OpenAI `image_edit` multipart request mapping, `ImageEdit` capability round-trip (`as_str`/`from_str`/`Display`).
- **Integration:** admin CRUD for `provider-profiles` and `models` under RBAC (super_admin succeeds, org_admin/team_admin/user forbidden), end-to-end wildcard passthrough chat completion against a mocked upstream, startup bootstrap seeding from a legacy `config.yaml`, each of the five new proxy routes against a mocked OpenAI upstream, and a `CapabilityNotSupported` 4xx response when the resolved model/adapter doesn't declare the requested capability (e.g. `/v1/images/edits` against a vllm-backed model).
- **Migration:** verify the `organization_id`-drop migration de-duplicates and preserves referential integrity against `request_logs`.
- **Regression:** existing `model_router` tests are rewritten for the new (no `organization_id`) signature rather than deleted.

## 13. Acceptance Criteria

- [ ] `provider_profiles` and `models` have no `organization_id` column; both are managed exclusively via the new `super_admin`-only admin API.
- [ ] Provider credentials are stored encrypted (AES-256-GCM) and are never returned in plaintext by any API response.
- [ ] All seven adapters (OpenAI, Anthropic, Gemini, vllm, sglang, llama.cpp, ollama) are stateless and receive credentials via `ResolvedProfile` resolved fresh per request.
- [ ] A `provider_profiles` row with `allow_wildcard = true` allows `POST /v1/chat/completions` with `model: "<profile_name>/<any-upstream-id>"` to succeed without a corresponding `models` row.
- [ ] A `provider_profiles` row with `allow_wildcard = false` rejects the same request with `NotFound` if no matching `models` row exists.
- [ ] The four new adapters pass chat, streaming, and embedding request/response mapping tests.
- [ ] `POST /v1/embeddings`, `/v1/images/generations`, `/v1/images/edits`, `/v1/audio/speech`, and `/v1/audio/transcriptions` each resolve a model and return a valid response against a mocked OpenAI adapter.
- [ ] A model backed by an adapter that doesn't declare the requested capability (e.g. `ImageEdit` on a vllm-backed model) is rejected with a clear error on the corresponding route.
- [ ] Startup bootstrap seeds `provider_profiles` from a legacy `config.yaml` exactly once, then the app functions with `AppConfig.providers` absent.
- [ ] All unit and integration tests pass with `cargo test --workspace`.
