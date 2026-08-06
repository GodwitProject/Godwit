# Godwit UI — Architecture

This doc walks through the physical layout of the Next.js app, how pages map to routed
segments, the hook/module layer, and the data flow (WebSocket live + REST fallback).

## High-level layout

The app uses two routing/app shells:

- **Next.js App Router** (`src/app/`) defines the page routes: `/` (Dashboard), `/providers`,
  `/keys`, `/logs`.
- A **shared layout shell** (`src/components/layout/`) wraps every page: a `Shell` composes a fixed
  `Sidebar` (desktop, 256px), a top `Header`, a `MobileNav` (bottom bar on small screens) and the
  page content in the `main` region. It is wired once in `src/app/layout.tsx`.

All pages are `'use client'` because they rely on hooks and interactivity.

## Page → component map

| Route            | Page file                    | Domain components |
|------------------|------------------------------|-------------------|
| `/`              | `src/app/page.tsx`           | `MetricCard`, `TimeSeriesChart`, `RecentLogsTable` |
| `/providers`     | `src/app/providers/page.tsx` | `ProviderList` |
| `/keys`          | `src/app/keys/page.tsx`      | `KeyList`, `KeyForm`, `KeyDetails` |
| `/logs`          | `src/app/logs/page.tsx`      | `LogFilters`, `LogsTable`, `LogDetail` |

Each page keeps its own column/row state locally (React `useState`) and renders a small set of
domain components. Domain components never talk to the network directly — they receive data via
props from hooks and emit callbacks (e.g. `onDetail`, `onSubmit`).

## Component layers

### `src/components/ui/` — base primitives (presentational only)

Theme-aware, stateless building blocks used everywhere: `Button`, `Card`, `Input`, `Select`,
`Badge`, `Table` (+ `TableHead/Body/Row/HeadCell/Cell`), `Modal`, `Toggle`, `Checkbox`. They accept
Tailwind classes via `className` and expose a consistent, small prop surface. They contain no data
logic.

### `src/components/` — domain components

Compose base primitives into UI for a specific feature:

- `layout/` — `Shell`, `Sidebar`, `Header`, `MobileNav`.
- `metrics/` — `MetricCard`, `TimeSeriesChart`.
- `logs/` — `RecentLogsTable` (dashboard), `LogsTable`, `LogFilters`, `LogDetail`.
- `keys/` — `KeyList`, `KeyForm` (create key modal with show-once full key), `KeyDetails` (usage +
  recent activity).
- `providers/` — `ProviderList` (expandable rows: base URL, models, fallback chain).

### `src/hooks/` — data access

React Query + WebSocket hooks are the only place components read network state:

- `useMetrics.ts`, `useProviders.ts`, `useKeys.ts`, `useLogs.ts` — standard React Query queries and
  mutations (create/update/delete/revoke) with `invalidateQueries` after writes.
- `useRealtimeMetrics.ts` — Dashboard-specific: tries WebSocket first, falls back to polling.

### `src/lib/` — api client, data sources, websocket

- `api.ts`, `providers.ts`, `keys.ts`, `logs.ts` — thin `fetch`-based clients returning
  strongly-typed DTOs (mirroring `src/hooks/` consumers).
- `websocket.ts` — `MetricsSocket` manager: connect, subscribe to the `metrics` channel,
  auto-reconnect with backoff, and failure signalling.
- `utils.ts`, `constants.ts` — shared helpers / config.

`src/lib/types.ts` (or per-module types) holds the TypeScript DTO shapes used across hooks and
components.

## Data flow

Two channels (see `README.md` for env details):

1. **Initial load (REST)** — on page mount, hooks fire queries against `NEXT_PUBLIC_API_URL`
   (`/api/v1/...`). React Query caches results and refetches on a per-page interval
   (e.g. health polls every 10s).

2. **Live updates (WebSocket)** — the Dashboard opens `MetricsSocket` against
   `NEXT_PUBLIC_WS_URL`. It sends `{ type: "subscribe", channel: "metrics" }` and applies incoming
   `{ type: "metrics:update", data }` messages to state. If the socket errors/closes repeatedly, the
   hook sets `status: "polling"` and the existing 5s REST polling takes over, so the dashboard
   never goes blank.

## Authentication

Auth is out of scope for the Phase 1 MVP. Future work should attach an Authorization header (master
key or per-key token) in the `src/lib/*` clients and the `MetricsSocket` connect handshake.

## Testing

- **Vitest + React Testing Library** (`src/components/**/*.test.tsx`) — unit tests for presentational
  and domain components; WebSocket logic is tested with a mocked global `WebSocket`.
- Run everything with `npm test`; type-check with `npm run type-check`.
