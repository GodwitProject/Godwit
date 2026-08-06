# Godwit UI

Admin dashboard for the [Godwit](https://github.com/GodwitProject/Godwit) LLM proxy. It provides a
real-time operator console for monitoring proxy performance and managing API keys, providers, and
request logs against the Godwit REST + WebSocket API.

> **Note:** All `NEXT_PUBLIC_*` variables are inlined into the client bundle at build time (see the
> `env` block in `next.config.js`), not read at runtime.

## Features

- **Dashboard** — live LLM proxy metrics (requests, latency, tokens, error rate) with a WebSocket
  live feed and automatic 5s polling fallback.
- **Providers** — read-only listing of configured upstream providers with health and model info.
- **API Keys** — create, edit, delete, and revoke proxy keys; per-key usage and request log views.
- **Logs** — paginated request log explorer with search, model/status/date filters and a detail
  modal including full request/response bodies and fallback/PII/moderation context.

## Prerequisites

- **Node.js 20+** and npm.
- A running **Godwit backend** exposing the REST API at `/api/v1` and a WebSocket endpoint at
  `/api/v1/ws` (the Go/Rust `godwit-bin` server). See the repo root README / `docker-compose.yml`.

## Quickstart

```bash
cd apps/ui

# 1. Install dependencies
npm install

# 2. Run the dev server (default port 3001)
npm run dev
```

Open http://localhost:3001. The dev server reads `NEXT_PUBLIC_*` from your environment (or falls
back to `localhost:3000`).

### Environment variables

| Variable                | Default                                    | Purpose                          |
|-------------------------|--------------------------------------------|----------------------------------|
| `NEXT_PUBLIC_API_URL`   | `http://localhost:3000/api/v1`             | Base URL of the Godwit REST API  |
| `NEXT_PUBLIC_WS_URL`    | `ws://localhost:3000/api/v1/ws`            | Base URL of the Godwit WebSocket |

`NEXT_PUBLIC_*` values are inlined at **build** time. In a production/docker build, supply them via
the Docker build args (or the `docker compose` service env) before `npm run build`.

The API URL must include the `/api/v1` prefix (the backend nests its admin API under `/api/v1`), and
the WS URL must be `ws://`/`wss://` (not `http`), ending in `/api/v1/ws`.

## Scripts

Run within `apps/ui/`:

| Script            | Description                                    |
|-------------------|------------------------------------------------|
| `npm run dev`     | Start the Next.js dev server (port 3001)       |
| `npm run build`   | Production build (standalone output)           |
| `npm start`       | Start the production server (`next start`)     |
| `npm run lint`    | Run ESLint (`next lint`)                       |
| `npm run type-check` | Type-check with `tsc --noEmit`              |
| `npm test`        | Run unit tests with Vitest + Testing Library   |

## Folder structure

```
apps/ui/
├── next.config.js          # standalone output + env block
├── tailwind.config.ts      # tailwind theme
├── vitest.config.ts        # unit test config
├── Dockerfile              # multi-stage standalone Docker build
├── src/
│   ├── app/                # Next.js App Router pages (route segments)
│   │   ├── page.tsx        # Dashboard
│   │   ├── keys/           # API Keys
│   │   ├── providers/      # Providers
│   │   └── logs/           # Logs
│   ├── components/
│   │   ├── layout/         # Shell, Sidebar, Header, MobileNav
│   │   ├── metrics/        # MetricCard, TimeSeriesChart
│   │   ├── logs/           # LogsTable, RecentLogsTable, LogFilters, LogDetail
│   │   ├── keys/           # KeyList, KeyForm, KeyDetails
│   │   ├── providers/      # ProviderList
│   │   └── ui/             # Button, Card, Modal, Badge, Table, Input, Select, ...
│   ├── hooks/              # React Query hooks + useRealtimeMetrics (WS)
│   ├── lib/                # API client (api.ts), data sources, websocket.ts
│   ├── styles/globals.css
│   └── test/setup.ts       # Vitest setup / jest-dom matchers
└── package.json
```

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for a component-by-page walkthrough and data flow.

## API integration

The UI talks to the backend over two channels:

- **REST** — all pages fetch data via thin client modules in `src/lib/` (`api.ts`, `keys.ts`,
  `logs.ts`, `providers.ts`) using `fetch` against `NEXT_PUBLIC_API_URL`. React Query hooks in
  `src/hooks/` manage caching, refetch intervals, and invalidation.
- **WebSocket** — the Dashboard streams live metric updates over `NEXT_PUBLIC_WS_URL` via
  `src/lib/websocket.ts` (`MetricsSocket`). It subscribes to the `metrics` channel and falls back to
  5s REST polling if the socket fails (see `src/hooks/useRealtimeMetrics.ts`).

## Docker

A multi-stage standalone Dockerfile is provided for production deployments:

```bash
cd apps/ui
docker build \
  --build-arg NEXT_PUBLIC_API_URL=http://localhost:8000/api/v1 \
  --build-arg NEXT_PUBLIC_WS_URL=ws://localhost:8000/api/v1/ws \
  -t godwit-ui .

docker run -p 3002:3000 godwit-ui
```

Or launch the full stack (database + API + UI + Prometheus + Grafana) from the repo root:

```bash
docker compose up --build
```

The `ui` service is exposed on host port **3002** (3001 is reserved for Grafana). Override the
browser-facing API origin with build args / compose env if your backend listens elsewhere.
