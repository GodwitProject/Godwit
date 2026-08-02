# Godwit

A high-performance, OpenAI-compatible LLM proxy written in Rust. Godwit routes chat-completion requests between clients and multiple LLM providers (OpenAI, Anthropic, and more to come), adding organization-aware API keys, role-based access control, spend tracking, and enterprise authentication (OIDC / SAML).

The name is inspired by the **Bar-tailed Godwit**, the bird that performs the longest non-stop migration known (~11,000 km from Alaska to New Zealand). Like the godwit, this proxy relays a request from one end to the other without interruption.

## Status

This repository contains the **MVP implementation** of Godwit. All 20 planned MVP tasks are complete:

- Modular workspace with core, database, auth, providers, cache, API, and binary crates
- PostgreSQL schema and SQLx migrations for users, organizations, teams, API keys, models, and request logs
- Argon2-hashed API keys with in-memory caching for a fast proxy hot path
- JWT issue/verify and RBAC (super_admin, org_admin, team_admin, user)
- OIDC discovery / authorization-code flow and SAML ACS scaffolding
- OpenAI-compatible `/v1/models` and `/v1/chat/completions` proxy routes
- Provider clients for OpenAI and Anthropic, with SSE streaming support
- Request logging and asynchronous spend tracking
- Admin REST API for users, organizations, teams, API keys, models, and spend
- Docker / Docker Compose packaging
- Integration-test scaffolding and Criterion benchmarks

## Architecture

```
crates/
  godwit-core/       Shared configuration, errors, and DTOs
  godwit-db/         SQLx migrations and repository layer
  godwit-auth/       API key / password hashing, JWT, OIDC, SAML
  godwit-providers/  Provider trait + OpenAI / Anthropic clients + SSE streaming
  godwit-cache/      In-memory DashMap cache for hot-path lookups
  godwit-api/        Axum routers, middleware, and admin/proxy routes
  godwit-bin/        `godwit` binary: config loading, DB startup, router assembly
```

## Quick start

### Prerequisites

- Rust 1.80+
- PostgreSQL 15+
- `DATABASE_URL` environment variable pointing to a PostgreSQL database

### Run locally

```bash
cp config.example.yaml config.yaml
# Edit config.yaml with your provider API keys

# Run migrations automatically on startup
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo run --bin godwit
```

### Run with Docker Compose

```bash
export OPENAI_API_KEY=...
export ANTHROPIC_API_KEY=...
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

See `config.example.yaml` for all available options:

- `server`: host, port, request timeout
- `database`: PostgreSQL connection URL
- `auth`: JWT secret, token TTLs, OIDC/SAML providers
- `providers`: OpenAI and Anthropic API keys and base URLs

## API

### Proxy (OpenAI-compatible)

- `GET /v1/models`
- `POST /v1/chat/completions`

Use a Godwit API key in the `Authorization: Bearer <key>` header.

### Admin

- `POST /api/v1/auth/login`
- `GET /api/v1/auth/oidc/:provider`
- `GET /api/v1/auth/oidc/:provider/callback`
- `POST /api/v1/auth/saml/:provider/acs`
- `GET /api/v1/users`
- `GET /api/v1/organizations`
- `GET /api/v1/teams`
- `GET|POST /api/v1/api-keys`
- `GET /api/v1/models`
- `GET /api/v1/spend`

Admin routes require a JWT obtained via `/api/v1/auth/login` or OIDC callback.

## License

MIT
