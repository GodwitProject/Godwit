# Godwit

A high-performance, OpenAI-compatible LLM proxy written in Rust. Godwit routes chat-completion, embedding, image, and audio requests between clients and multiple LLM providers — three hosted providers (OpenAI, Anthropic, Gemini) and four self-hosted/OpenAI-compatible backends (vllm, sglang, llama.cpp, ollama) — adding an instance-wide model/provider catalog with encrypted credentials, organization-aware API keys, role-based access control, spend tracking, and enterprise authentication (OIDC / SAML).

The name is inspired by the **Bar-tailed Godwit**, the bird that performs the longest non-stop migration known (~11,000 km from Alaska to New Zealand). Like the godwit, this proxy relays a request from one end to the other without interruption.

## Status

This repository contains the full implementation of Godwit, including the provider-catalog and self-hosted-adapters extension. All 22 planned tasks are complete:

- Modular workspace with core, database, auth, providers, cache, API, and binary crates
- PostgreSQL schema and SQLx migrations for users, organizations, teams, API keys, an instance-wide `provider_profiles`/`models` catalog, and request logs
- Argon2-hashed API keys with in-memory caching for a fast proxy hot path
- AES-256-GCM encryption of provider credentials at rest (`CREDENTIAL_ENCRYPTION_KEY`)
- JWT issue/verify and RBAC (super_admin, org_admin, team_admin, user)
- OIDC discovery / authorization-code flow and SAML ACS scaffolding
- OpenAI-compatible proxy routes: `GET /v1/models`, `POST /v1/chat/completions`, `POST /v1/embeddings`, `POST /v1/images/generations`, `POST /v1/images/edits`, `POST /v1/audio/speech`, `POST /v1/audio/transcriptions`
- Stateless provider adapters, resolved per-request from the database via `ResolvedProfile`, for all 7 providers: OpenAI, Anthropic, Gemini, vllm, sglang, llama.cpp, ollama
- Capabilities: chat, embedding, image generation, image edit, audio speech, audio transcription
- Request logging and asynchronous spend tracking
- Admin REST API for users, organizations, teams, API keys, provider profiles, models, and spend (provider-profiles/models management is super_admin only)
- Startup bootstrap that seeds `provider_profiles` from legacy `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GEMINI_API_KEY` env vars on first boot
- Docker / Docker Compose packaging
- Integration-test scaffolding and Criterion benchmarks

## Architecture

```
crates/
  godwit-core/       Shared configuration, errors, and DTOs
  godwit-db/         SQLx migrations and repository layer (instance-wide provider_profiles/models catalog)
  godwit-auth/       API key / password hashing, JWT, OIDC, SAML, credential encryption (AES-256-GCM)
  godwit-providers/  Provider trait + stateless adapters (OpenAI, Anthropic, Gemini, vllm, sglang, llama.cpp, ollama) + SSE streaming
  godwit-cache/      In-memory DashMap cache for hot-path lookups
  godwit-api/        Axum routers, middleware, model router (DB-backed), and admin/proxy routes
  godwit-bin/        `godwit` binary: config loading, DB startup, provider_profiles bootstrap, router assembly
```

Provider credentials and the model catalog live in the database (`provider_profiles`, `models`) rather than static config — they are instance-wide (no `organization_id`) and managed through the admin API. Each proxy request resolves a `ResolvedProfile` (decrypted credentials + base URL + protocol) from the catalog and hands it to the matching stateless adapter, so adapters hold no per-tenant state.

## Quick start

### Prerequisites

- Rust 1.80+
- PostgreSQL 15+
- `DATABASE_URL` environment variable pointing to a PostgreSQL database

### Run locally

```bash
cp config.example.yaml config.yaml
# Edit config.yaml for server/database/auth settings

export DATABASE_URL=postgres://user:pass@localhost:5432/godwit
export CREDENTIAL_ENCRYPTION_KEY=$(openssl rand -base64 32)

# Optional: seed the provider_profiles catalog from legacy env vars on first boot
export OPENAI_API_KEY=...
export ANTHROPIC_API_KEY=...
export GEMINI_API_KEY=...

# Run migrations automatically on startup
cargo run --bin godwit
```

After first boot, `provider_profiles` in the database is the source of truth — manage providers and models going forward through the admin `provider-profiles`/`models` API rather than env vars.

### Run with Docker Compose

```bash
export OPENAI_API_KEY=...
export ANTHROPIC_API_KEY=...
export GEMINI_API_KEY=...
export CREDENTIAL_ENCRYPTION_KEY=...
export JWT_SECRET=...
docker compose up --build
```

## Testing

Unit tests:

```bash
cargo test --workspace
```

Integration tests (require a running server):

```bash
cargo test --test proxy_integration -- --ignored
cargo test --test admin_integration -- --ignored
```

Benchmarks:

```bash
cargo bench -p godwit-providers
```

Load test:

```bash
./scripts/bench.sh
```

## Configuration

See `config.example.yaml` for server/database/auth options:

- `server`: host, port, request timeout
- `database`: PostgreSQL connection URL
- `auth`: JWT secret, token TTLs, OIDC/SAML providers

Provider credentials and the model catalog are **not** configured in `config.yaml` — they live in the `provider_profiles` and `models` tables and are managed through the admin API. Required environment variables:

- `DATABASE_URL`: PostgreSQL connection string
- `CREDENTIAL_ENCRYPTION_KEY`: base64-encoded 32-byte AES-256-GCM key used to encrypt/decrypt provider credentials at rest. **Required** — the process fails to start without it.
- `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GEMINI_API_KEY` (optional): read once at startup by the bootstrap step, which seeds `provider_profiles` for OpenAI/Anthropic/Gemini only if the table is empty. They have no effect once any provider profile exists — after first boot, manage providers exclusively via the admin `provider-profiles` API. Self-hosted backends (vllm, sglang, llama.cpp, ollama) are not bootstrapped from env vars and must be created through the admin API.

## API

### Proxy (OpenAI-compatible)

- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/embeddings`
- `POST /v1/images/generations`
- `POST /v1/images/edits`
- `POST /v1/audio/speech`
- `POST /v1/audio/transcriptions`

Use a Godwit API key in the `Authorization: Bearer <key>` header. Requests resolve a model against the `provider_profiles`/`models` catalog and dispatch to the matching adapter (OpenAI, Anthropic, Gemini, vllm, sglang, llama.cpp, or ollama), depending on which capability the requested model supports.

### Admin

All admin routes require a JWT obtained via `/api/v1/auth/login`, `/api/v1/auth/oidc/:provider/callback`, or `/api/v1/auth/refresh`. The `provider-profiles` and `models` endpoints additionally require the `super_admin` role, since they control credentials and instance-wide routing.

#### Authentication

- `POST /api/v1/auth/login` — Email/password login. Request: `{ "email": "...", "password": "..." }`. Response: `{ "access_token": "...", "refresh_token": "..." }`. Both tokens are issued; `refresh_token` is single-use, rotating on each use.
- `POST /api/v1/auth/refresh` — Issue new access+refresh token pair using an existing refresh token. Request: `{ "refresh_token": "..." }`. Response: `{ "access_token": "...", "refresh_token": "..." }`.
- `POST /api/v1/auth/logout` — Invalidate a refresh token. Request: `{ "refresh_token": "..." }`. Response: `{ "logged_out": true }`.
- `GET /api/v1/auth/oidc/:provider` — Redirect to OIDC authorization endpoint.
- `GET /api/v1/auth/oidc/:provider/callback` — OIDC authorization-code callback. Returns: `{ "access_token": "...", "refresh_token": "..." }`. Creates user on first login if not present.
- `POST /api/v1/auth/saml/:provider/acs` — SAML ACS endpoint (scaffolded, requires IdP metadata and XML signature validation).

#### Organizations

Requires `super_admin` role. All operations return organization objects.

- `GET /api/v1/organizations` — List all organizations.
- `POST /api/v1/organizations` — Create organization. Request: `{ "name": "...", "rate_limit_requests_per_minute": <optional> }`.
- `PATCH /api/v1/organizations/:id` — Update organization. Request: `{ "name": "...", "rate_limit_requests_per_minute": <optional> }`.

#### Teams

Requires `org_admin` or `super_admin` role. Team CRUD is scoped to the user's organization unless the user is `super_admin` (who may pass `?organization_id=` to list/create for any organization). Team members (users within a team) can be managed by `team_admin` role holders for that specific team.

- `GET /api/v1/teams?organization_id=<optional>` — List teams. If `super_admin`, may query any organization via `organization_id`; otherwise, limited to own organization.
- `POST /api/v1/teams` — Create team. Request: `{ "name": "...", "organization_id": <optional; required for super_admin, ignored otherwise> }`.
- `PATCH /api/v1/teams/:id` — Update team. Request: `{ "name": "..." }`.
- `POST /api/v1/teams/:id/members` — Add user to team. Request: `{ "user_id": "...", "role": "team_admin" | "member" }`.
- `DELETE /api/v1/teams/:id/members/:user_id` — Remove user from team.

#### Users

Requires `org_admin` or `super_admin` role. User CRUD is scoped to the user's organization unless the user is `super_admin` (who may pass `?organization_id=` to list/create for any organization). A user cannot delete their own account or change their own role.

- `GET /api/v1/users?organization_id=<optional>` — List users. If `super_admin`, may query any organization via `organization_id`; otherwise, limited to own organization.
- `POST /api/v1/users` — Create user. Request: `{ "email": "...", "name": <optional>, "role": "super_admin" | "org_admin" | "team_admin" | "user" }`. Requires `super_admin` role to create `super_admin` users; `org_admin` can create lower roles within their organization.
- `GET /api/v1/users/:id` — Fetch user by ID. Returns user object.
- `PATCH /api/v1/users/:id` — Update user. Request: `{ "name": <optional>, "role": <optional>, "organization_id": <optional> }`. Only `super_admin` may move a user to a different organization.
- `DELETE /api/v1/users/:id` — Delete user. Cascades to team memberships, API keys, and request logs. Cannot delete self.

#### Spend & Usage

- `GET /api/v1/spend?from=<ISO8601>&to=<ISO8601>&organization_id=<optional>&team_id=<optional>&user_id=<optional>` — Aggregate spend grouped by organization/team/user. All query parameters are optional; filters are applied conjunctively. Response: `{ "data": [ { "organization_id": "...", "team_id": "...", "user_id": "...", "total_cost_usd": "...", "request_count": <int>, "tokens_in": <int>, "tokens_out": <int> }, ... ] }`. RBAC scoping: `super_admin` gets unfiltered results; `org_admin` is always scoped to their own organization but may filter by team/user; `team_admin`/`user` are scoped to their own usage only, with any org/team/user query param ignored.

#### API Keys

- `GET /api/v1/api-keys` — List API keys for the authenticated user.
- `POST /api/v1/api-keys` — Create a new API key. Returns the plaintext key (only returned on creation; hashed form is stored).

#### Provider Profiles & Models

Requires `super_admin` role. Credentials are never returned in responses — `provider-profiles` responses expose only a `has_credentials` boolean.

- `GET /api/v1/provider-profiles` — List all provider profiles.
- `POST /api/v1/provider-profiles` — Create provider profile. Request: `{ "name": "...", "provider": "openai" | "anthropic" | "gemini" | "vllm" | "sglang" | "llama.cpp" | "ollama", "api_key": "..." }`.
- `PATCH /api/v1/provider-profiles/:id` — Update provider profile credentials. Request: `{ "api_key": "..." }`.
- `GET /api/v1/models` — List all models.
- `POST /api/v1/models` — Create model. Request: `{ "public_id": "...", "provider_profile_id": "...", "provider_model_id": "...", "capabilities": [ "chat" | "embedding" | "image_generation" | "image_edit" | "audio_speech" | "audio_transcription" ], "pricing": { "input_per_1k": "...", "output_per_1k": "..." }, "config": {} }`.
- `PATCH /api/v1/models/:id` — Update model. Request: `{ "capabilities": <optional>, "pricing": <optional>, "config": <optional> }`.
- `DELETE /api/v1/models/:id` — Delete model.

## License

MIT
