# Deployment

Production-style deployment of the Godwit LLM proxy is provided via Docker Compose. It runs the Godwit backend together with PostgreSQL 15, Prometheus, and Grafana, all wired on a shared bridge network with persistent storage and health-gated startup ordering.

## Services

| Service | Image / Build | Host port | Purpose |
|---------|---------------|-----------|---------|
| `db` | `postgres:15-alpine` | 5432 | PostgreSQL 15 (SQLx migrations run on backend startup) |
| `api` | local `Dockerfile` | 8000 → 3000 | Godwit backend (axum, OpenAI-compatible proxy) |
| `prometheus` | `prom/prometheus:latest` | 9090 | Scrapes `api:3000/metrics` |
| `grafana` | `grafana/grafana:latest` | 3001 → 3000 | Dashboards for Godwit metrics |
| `ui` | `apps/ui/Dockerfile` (multi-stage) | 3002 → 3000 | Next.js UI (optional) |
| `admin` | `apps/admin/Dockerfile` | 3000 → 3000 | Legacy admin console (profile `admin`) |

## Prerequisites

- Docker Engine with the Compose plugin (v2+).
- Rust toolchain is only needed if you build the backend binary manually; the container build handles it via `cargo-chef`.

## Configuration

All configuration lives in the root `.env` file (see `.env` in the repo for the variable list). Copy and edit as needed:

```bash
cp .env .env.prod
# edit .env.prod: set real API keys, JWT_SECRET, ADMIN_* credentials, Grafana password
```

> **Never commit real secrets.** The repo's `.env` contains only dev/test placeholders.

Key variables:

- `DATABASE_URL` is **populated by the compose file** from `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB`; you usually don't set it directly.
- `OPENAI_API_KEY`, `ANTHROPIC_API_KEY` — provider credentials.
- `JWT_SECRET`, `CREDENTIAL_ENCRYPTION_KEY` — signing/encryption secrets. Change in production.
- `ADMIN_EMAIL`, `ADMIN_PASSWORD` — first super-admin account, created on startup (see `crates/godwit-bin/src/bootstrap.rs`).
- `GRAFANA_ADMIN_USER` / `GRAFANA_ADMIN_PASSWORD` — default `admin` / `admin`; **change in production**.

## Backend config

The backend reads `config.yaml` (a copy of `config.example.yaml`, mounted into the container at `/app/config.yaml`). The compose file points `DATABASE_URL` at the `db` service, overriding the `database.url` in the YAML when the environment variable is honored. Metrics are exposed by the binary at `GET /metrics` on the server port.

## Startup

```bash
# Validate the compose file (no build)
docker compose config

# Build images
docker compose build

# Start everything
docker compose up -d

# View logs
docker compose logs -f api

# Bring up just the legacy admin console
docker compose --profile admin up -d admin
```

Startup ordering is enforced via health checks:

- `db` must be healthy (`pg_isready`) before `api` starts.
- `api` must be healthy (`GET /health`) before `prometheus` and `ui` start.
- `prometheus` must be healthy before `grafana` starts.

Migrations: the `godwit` binary runs SQLx migrations against the database automatically on startup — no separate step required.

## Monitoring

- Prometheus UI: http://localhost:9090
- Grafana: http://localhost:3001 (login with `GRAFANA_ADMIN_USER` / `GRAFANA_ADMIN_PASSWORD`; default `admin`/`admin`)
  - The Prometheus datasource and the **Godwit** dashboard are auto-provisioned from `docker/grafana/provisioning/`.
  - Dashboard panels: request rate, latency p95 histogram, token usage, cost, error rate, active requests.

## Volumes

Persistent named volumes are used so data survives container restarts:

- `pgdata` — PostgreSQL data (`/var/lib/postgresql/data`)
- `prometheus_data` — time-series data (`/prometheus`)
- `grafana_data` — Grafana data (`/var/lib/grafana`)

Back these up regularly. To reset: `docker compose down -v`.

## Notes / caveats

- The `ui` and `admin` services are optional auxiliary frontends; the core proxy deployment is `db` + `api`. If you do not need the Next.js UI, comment it out (or use a profile).
- The legacy `admin` service runs a raw `npm run dev` Next.js dev server, which is not production-grade; it is gated behind the `admin` profile for that reason.
- If `next build` fails for the `ui` image (e.g. node runtime SIGBUS on certain hosts/kernels), see the TODO comment in `apps/ui/Dockerfile` — the service stays wired up in the compose network so it can be enabled once the build is verified.
