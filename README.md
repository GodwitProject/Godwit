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

- `POST /api/v1/auth/login`
- `GET /api/v1/auth/oidc/:provider`
- `GET /api/v1/auth/oidc/:provider/callback`
- `POST /api/v1/auth/saml/:provider/acs`
- `GET /api/v1/users`
- `GET /api/v1/organizations`
- `GET /api/v1/teams`
- `GET|POST /api/v1/api-keys`
- `GET|POST /api/v1/provider-profiles`
- `PATCH /api/v1/provider-profiles/:id`
- `GET|POST /api/v1/models`
- `PATCH|DELETE /api/v1/models/:id`
- `GET /api/v1/spend`

Admin routes require a JWT obtained via `/api/v1/auth/login` or OIDC callback. The `provider-profiles` and `models` endpoints additionally require the `super_admin` role, since they control credentials and instance-wide routing. Credentials are never returned in responses — `provider-profiles` responses expose only a `has_credentials` boolean.

## License

MIT
