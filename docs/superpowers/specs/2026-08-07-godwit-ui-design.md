# Godwit UI — Design Specification

**Date:** 2026-08-07  
**Version:** 1.0.0  
**Status:** Draft → Review → Approved  
**Author:** Godwit Team

---

## 1. Overview

### 1.1 Purpose

Godwit UI est un dashboard admin moderne pour la gestion et l'observabilité d'un proxy LLM en production. Il fournit une interface unifiée pour :

- Surveiller les performances (requests, latency, tokens, coûts)
- Configurer les providers et modèles
- Gérer les accès (API keys, permissions, rate limits)
- Consulter les logs et déboguer les requêtes
- Configurer les guardrails (PII masking, moderation, budget alerts)

### 1.2 Goals

- **Productivité** : Un admin peut configurer un provider en < 2 minutes
- **Visibilité** : Temps réel sur les metrics critiques (latency, error rate, spend)
- **Fiabilité** : Détection rapide des incidents (providers down, rate limits, budgets dépassés)
- **Sécurité** : Gestion fine des accès (API keys, scopes, audits)

### 1.3 Non-Goals

- Interface de chat pour end-users (hors scope — c'est un dashboard admin)
- Éditeur de prompts ou playground (peut être ajouté plus tard)
- Multi-tenant white-label (une seule instance, multi-orgs en interne)

---

## 2. Architecture

### 2.1 Stack Technique

```
Frontend:
  - Framework: Next.js 14 (App Router, Server Components)
  - Language: TypeScript 5.3+
  - Styling: Tailwind CSS 3.4+ + custom tokens (Godwit Design System)
  - State: React Query (server state) + Zustand (client state)
  - Real-time: WebSocket (metrics en direct) + polling fallback (10s)
  - Charts: Recharts ou Visx (léger, customizable)
  - Tables: TanStack Table (filtrage, tri, pagination)
  - Forms: React Hook Form + Zod validation

Backend (Godwit Rust):
  - WebSocket endpoint: `ws://localhost:3000/api/v1/ws/metrics`
  - REST API: `http://localhost:3000/api/v1/*`
  - Authentication: JWT (via API keys ou OIDC)

Deployment:
  - Option A: Docker (image multi-stage, Nginx reverse proxy)
  - Option B: Vercel/Netlify (frontend) + Godwit backend séparé
  - Option C: Mono-repo (apps/ui/ dans Godwit)
```

### 2.2 Structure du Projet

```
apps/ui/
├── src/
│   ├── app/
│   │   ├── layout.tsx              # Root layout (sidebar + header)
│   │   ├── page.tsx                # Dashboard Overview
│   │   ├── providers/
│   │   │   └── page.tsx            # Providers management
│   │   ├── keys/
│   │   │   └── page.tsx            # API Keys management
│   │   ├── logs/
│   │   │   └── page.tsx            # Request logs
│   │   ├── usage/
│   │   │   └── page.tsx            # Usage analytics
│   │   ├── guardrails/
│   │   │   └── page.tsx            # Guardrails config
│   │   ├── models/
│   │   │   └── page.tsx            # Model catalog
│   │   └── settings/
│   │       └── page.tsx            # Global settings
│   ├── components/
│   │   ├── ui/                     # Base components
│   │   │   ├── Button.tsx
│   │   │   ├── Card.tsx
│   │   │   ├── Table.tsx
│   │   │   ├── Input.tsx
│   │   │   ├── Select.tsx
│   │   │   ├── Badge.tsx
│   │   │   ├── Modal.tsx
│   │   │   └── ...
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── Header.tsx
│   │   │   ├── Shell.tsx
│   │   │   └── MobileNav.tsx
│   │   ├── metrics/
│   │   │   ├── MetricCard.tsx
│   │   │   ├── TimeSeriesChart.tsx
│   │   │   ├── BreakdownChart.tsx
│   │   │   └── StatusIndicator.tsx
│   │   ├── providers/
│   │   │   ├── ProviderList.tsx
│   │   │   ├── ProviderForm.tsx
│   │   │   ├── FallbackChainBuilder.tsx
│   │   │   └── HealthStatus.tsx
│   │   ├── keys/
│   │   │   ├── KeyList.tsx
│   │   │   ├── KeyForm.tsx
│   │   │   ├── KeyDetails.tsx
│   │   │   └── UsageTable.tsx
│   │   ├── logs/
│   │   │   ├── LogsTable.tsx
│   │   │   ├── LogFilters.tsx
│   │   │   ├── LogDetail.tsx
│   │   │   └── SearchBar.tsx
│   │   ├── usage/
│   │   │   ├── SpendChart.tsx
│   │   │   ├── TokenBreakdown.tsx
│   │   │   ├── CostByOrg.tsx
│   │   │   └── BudgetProgress.tsx
│   │   ├── guardrails/
│   │   │   ├── PiiConfig.tsx
│   │   │   ├── ModerationConfig.tsx
│   │   │   ├── BudgetAlerts.tsx
│   │   │   └── FallbackStats.tsx
│   │   └── models/
│   │       ├── ModelCatalog.tsx
│   │       ├── ModelCard.tsx
│   │       └── ModelForm.tsx
│   ├── lib/
│   │   ├── api.ts                  # API client (REST + WebSocket)
│   │   ├── websocket.ts            # WebSocket connection manager
│   │   ├── utils.ts                # Helpers (formatting, dates, etc.)
│   │   ├── types.ts                # TypeScript types
│   │   └── constants.ts            # Config constants
│   ├── hooks/
│   │   ├── useMetrics.ts           # Real-time metrics hook
│   │   ├── useProviders.ts         # Providers data hook
│   │   ├── useKeys.ts              # API keys data hook
│   │   └── ...
│   └── styles/
│       └── globals.css             # Tailwind + custom tokens
├── public/
│   └── favicon.ico
├── tailwind.config.ts
├── tsconfig.json
├── package.json
└── README.md
```

### 2.3 Data Flow

```
┌─────────────┐      WebSocket       ┌─────────────┐
│   Godwit    │ ───────────────────► │   Godwit    │
│   Backend   │    (metrics stream)  │     UI      │
│  (Rust)     │ ◄─────────────────── │  (Next.js)  │
└─────────────┘    (subscriptions)   └─────────────┘
       ▲                                    │
       │                                    │
       │ REST API                           │
       │ (CRUD operations)                  │
       └────────────────────────────────────┘
```

**Flux typique :**

1. **Initial load** : Next.js fetch data via REST (`/api/v1/providers`, `/api/v1/keys`, etc.)
2. **Real-time updates** : WebSocket connection établie, subscribe à `/metrics`
3. **User actions** : Forms submit → REST API → refresh local state (React Query)
4. **Navigation** : Next.js App Router → server components fetch data per page

---

## 3. Design System

### 3.1 Couleurs

Basé sur le DESIGN.md fourni (Google Stitch) :

```typescript
// tailwind.config.ts
colors: {
  // Surface
  surface: '#f8f9fb',
  'surface-dim': '#d9dadc',
  'surface-bright': '#f8f9fb',
  'surface-container-lowest': '#ffffff',
  'surface-container-low': '#f3f4f6',
  'surface-container': '#edeef0',
  'surface-container-high': '#e7e8ea',
  'surface-container-highest': '#e1e2e4',
  
  // On Surface
  'on-surface': '#191c1e',
  'on-surface-variant': '#434655',
  
  // Primary (Godwit Cobalt Blue)
  primary: '#004ac6',
  'on-primary': '#ffffff',
  'primary-container': '#2563eb',
  'on-primary-container': '#eeefff',
  'primary-fixed': '#dbe1ff',
  'primary-fixed-dim': '#b4c5ff',
  
  // Secondary
  secondary: '#515f74',
  'on-secondary': '#ffffff',
  'secondary-container': '#d5e3fc',
  'on-secondary-container': '#57657a',
  
  // Tertiary
  tertiary: '#005a82',
  'on-tertiary': '#ffffff',
  'tertiary-container': '#0074a6',
  'on-tertiary-container': '#e4f2ff',
  
  // Error
  error: '#ba1a1a',
  'on-error': '#ffffff',
  'error-container': '#ffdad6',
  'on-error-container': '#93000a',
  
  // Functional Status Colors
  success: '#10b981',    // Emerald
  warning: '#f59e0b',    // Amber
  info: '#3b82f6',       // Blue
  
  // Borders & Outlines
  outline: '#737686',
  'outline-variant': '#c3c6d7',
}
```

### 3.2 Typographie

```typescript
// Fonts
fontFamily: {
  sans: ['Inter', 'system-ui', 'sans-serif'],
  mono: ['JetBrains Mono', 'monospace'],
}

// Font Sizes
fontSize: {
  'display-lg': ['30px', { lineHeight: '36px', fontWeight: '700', letterSpacing: '-0.02em' }],
  'headline-md': ['24px', { lineHeight: '32px', fontWeight: '700', letterSpacing: '-0.01em' }],
  'title-md': ['20px', { lineHeight: '28px', fontWeight: '700' }],
  'section-sm': ['18px', { lineHeight: '28px', fontWeight: '600' }],
  'body-base': ['16px', { lineHeight: '24px', fontWeight: '400' }],
  'label-sm': ['14px', { lineHeight: '20px', fontWeight: '500' }],
  'caption-xs': ['12px', { lineHeight: '16px', fontWeight: '400' }],
  'code-sm': ['13px', { lineHeight: '20px', fontWeight: '400' }],
}
```

### 3.3 Spacing

```typescript
spacing: {
  'base-unit': '4px',
  'gutter': '16px',
  'margin-mobile': '16px',
  'margin-desktop': '32px',
  'sidebar-width': '256px',
  'container-padding': '24px',
}
```

### 3.4 Elevation & Borders

```css
/* Ambient Shadow (low elevation) */
.ambient-shadow {
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px -1px rgba(0, 0, 0, 0.1);
}

/* Hairline Border (subtle divider) */
.hairline-border {
  border: 1px solid #e5e7eb;
}

/* Rounded Corners */
rounded-sm: 0.125rem (2px)
rounded-DEFAULT: 0.25rem (4px)
rounded-md: 0.375rem (6px)
rounded-lg: 0.5rem (8px)
rounded-xl: 0.75rem (12px)
rounded-full: 9999px
```

### 3.5 Composants de Base

**Button :**
- Primary: `bg-primary text-on-primary hover:bg-primary/90`
- Secondary: `bg-surface-container-lowest hairline-border hover:bg-surface-container-low`
- Ghost: `bg-transparent hover:bg-surface-container-high`
- Sizes: sm (label-sm), md (body-base), lg (title-md)

**Input :**
- Background: `bg-surface-container-lowest`
- Border: `1px solid outline-variant`
- Focus: `2px ring primary`
- Label: `label-sm font-medium, 4px above input`

**Card :**
- Background: `bg-surface-container-lowest`
- Padding: `24px`
- Border: `hairline-border`
- Shadow: `ambient-shadow`
- Radius: `rounded-xl`

**Table :**
- Header: `bg-surface-container-low caption-xs uppercase`
- Row: `border-b hairline-border hover:bg-surface-container-low`
- Cell: `py-3 px-6 body-base`
- Monospace columns: `code-sm` pour IDs, tokens, latencies

**Badge/Status Chip :**
- Background: `10% opacity` de la couleur de status
- Text: couleur saturée
- Radius: `rounded-full` (pill)
- Ex: `bg-success/10 text-success` pour "Healthy"

---

## 4. Pages Détaillées

### 4.1 Dashboard Overview (`/`)

**URL:** `/`  
**Layout:** Full width (max-w-7xl), sidebar + header  
**Real-time:** ✅ WebSocket (metrics updates every 1s)

#### Sections

**A. Header**
- Title: "Dashboard"
- Subtitle: "Real-time LLM proxy performance metrics"
- Actions:
  - Date range picker: "Last 24 Hours" (dropdown: 1h, 6h, 24h, 7d, 30d, custom)
  - Export button: CSV/PDF download

**B. Metrics Grid (4 cards)**

| Metric | Format | Trend | Source |
|--------|--------|-------|--------|
| Total Requests | `1.24M` | `+12.5%` vs yesterday | `GET /api/v1/metrics/summary` |
| Avg Latency | `342ms` | `+42ms` (warning if >500ms) | `GET /api/v1/metrics/latency` |
| Token Usage | `845M` (input+output) | `+5.2%` | `GET /api/v1/metrics/tokens` |
| Error Rate | `0.04%` | "Stable" (green if <1%) | `GET /api/v1/metrics/errors` |

**C. Time Series Chart**
- Title: "Request Volume"
- X-axis: Time (00:00 → 24:00)
- Y-axis: Requests count
- Data: 1 point per minute (aggregated)
- Interactive: Hover tooltip (exact count), zoom (brush selection)
- Real-time: Update every 1s via WebSocket

**D. Recent Events Table**
- Title: "Recent Proxy Events"
- Link: "View All Logs" → `/logs`
- Columns:
  - Timestamp (`10:42:15 AM`)
  - Request ID (`req_8f73b2a1` — monospace)
  - Model (`gpt-4-turbo`)
  - Status (Badge: `200 OK` green, `429 Rate Limit` red)
  - Latency (`245ms` — monospace)
- Rows: Last 10 events
- Sort: Timestamp DESC

**API Endpoints Required:**
```
GET /api/v1/metrics/summary         # Total requests, error rate
GET /api/v1/metrics/latency         # p50, p95, p99
GET /api/v1/metrics/tokens          # Input/output/cache tokens
GET /api/v1/metrics/timeseries      # Request volume over time
GET /api/v1/logs/recent?limit=10    # Recent events
WS /api/v1/ws/metrics               # Real-time updates
```

---

### 4.2 Providers (`/providers`)

**URL:** `/providers`  
**Layout:** Full width  
**Real-time:** ✅ Health status polling (10s)

#### Sections

**A. Header**
- Title: "Providers"
- Subtitle: "Configure LLM providers and fallback chains"
- Actions:
  - "Add Provider" button → Modal form
  - "Sync Pricing" button → Trigger GitHub sync (background job)

**B. Provider List (Table)**

| Column | Format | Notes |
|--------|--------|-------|
| Provider | Logo + Name (OpenAI, Anthropic, etc.) | Click → expand details |
| Status | Badge (✅ Healthy, ⚠️ Degraded, ❌ Down) | Polled every 10s |
| Models | Count (`23 models`) | Click → filter model catalog |
| Avg Latency | `342ms` (p95) | From last 5 minutes |
| Error Rate | `0.04%` | From last 24h |
| Actions | Edit, Delete, Test Connection | Dropdown menu |

**C. Provider Detail (Expandable Row)**
When clicking a provider:

- **Config Section:**
  - Base URL
  - API Key (masked: `sk-****-xyz`)
  - Timeout (ms)
  - Retry config (max retries, backoff)
  
- **Models Section:**
  - List of enabled models
  - Toggle per model (enable/disable)
  - Pricing (input/output/cache per 1K tokens)
  
- **Fallback Chain Section:**
  - Visual builder (drag-and-drop)
  - Primary → Fallback 1 → Fallback 2 → ...
  - Conditions: "On error", "On timeout", "On rate limit"
  - Stats: "Fallback triggered 12 times in last 24h"

**D. Add Provider Modal**
- Form fields:
  - Provider type (dropdown: OpenAI, Anthropic, Gemini, Azure, etc.)
  - Name (custom label)
  - Base URL
  - API Key (password input)
  - Timeout (ms, default: 30000)
  - Test Connection button (before save)
- Validation: Required fields, URL format, API key format
- On submit: `POST /api/v1/providers`

**API Endpoints Required:**
```
GET /api/v1/providers                 # List all providers
POST /api/v1/providers                # Create provider
GET /api/v1/providers/:id             # Get provider details
PUT /api/v1/providers/:id             # Update provider
DELETE /api/v1/providers/:id          # Delete provider
POST /api/v1/providers/:id/test       # Test connection
GET /api/v1/providers/:id/models      # List models for provider
GET /api/v1/providers/:id/fallbacks   # Get fallback chain
PUT /api/v1/providers/:id/fallbacks   # Update fallback chain
GET /api/v1/providers/health          # Health check all providers
```

---

### 4.3 API Keys (`/keys`)

**URL:** `/keys`  
**Layout:** Full width  
**Real-time:** ❌ (polling 30s for usage stats)

#### Sections

**A. Header**
- Title: "API Keys"
- Subtitle: "Manage access credentials and permissions"
- Actions:
  - "Create Key" button → Modal form

**B. Key List (Table)**

| Column | Format | Notes |
|--------|--------|-------|
| Name | `Production App Key` | Click → detail view |
| Prefix | `sk_live_****` | Monospace, first 8 + last 4 chars |
| Owner | User/Team name | Avatar + name |
| Scopes | Badges (`chat`, `embeddings`, `admin`) | Multi-value |
| Spend (30d) | `$1,234.56` | From spend tracking |
| Requests (24h) | `45.2K` | Count |
| Last Used | `2 minutes ago` | Relative time |
| Status | Toggle (Active/Revoked) | Click to toggle |
| Actions | Edit, Revoke, Delete | Dropdown |

**C. Create Key Modal**
- Form fields:
  - Name (required)
  - Owner (dropdown: users/teams)
  - Scopes (checkboxes: `chat`, `embeddings`, `images`, `admin`, `billing`)
  - Allowed Models (multi-select, default: all)
  - Budget (optional: `$ amount` per day/week/month)
  - Rate Limit (optional: `RPM` / `TPM`)
  - Expiry (optional: date picker)
  - Permissions:
    - ✅ Can view logs
    - ✅ Can view spend
    - ❌ Can create other keys (admin only)
- On submit: `POST /api/v1/keys` → returns full key (show once, then masked)
- **Important:** Display key only once with warning: "Copy this key now. You won't see it again."

**D. Key Detail View**
When clicking a key name:

- **Header:**
  - Name, prefix, owner, created date
  - Copy button (for full key, if just created)
  - Revoke/Activate toggle
  
- **Usage Stats:**
  - Spend over time (chart, 30 days)
  - Requests per day (bar chart)
  - Top models used (pie chart)
  - Token breakdown (input/output/cache)
  
- **Recent Activity:**
  - Table: last 50 requests
  - Filters: date range, model, status
  
- **Settings:**
  - Edit name
  - Update scopes
  - Change budget/rate limits
  - Set expiry

**E. Bulk Actions**
- Checkboxes on table rows
- Actions: Revoke selected, Delete selected, Export selected

**API Endpoints Required:**
```
GET /api/v1/keys                      # List all keys (with stats)
POST /api/v1/keys                     # Create new key
GET /api/v1/keys/:id                  # Get key details
PUT /api/v1/keys/:id                  # Update key
DELETE /api/v1/keys/:id               # Delete key
POST /api/v1/keys/:id/revoke          # Revoke key
GET /api/v1/keys/:id/usage            # Usage stats (spend, requests, tokens)
GET /api/v1/keys/:id/logs             # Request logs for this key
```

---

### 4.4 Logs (`/logs`)

**URL:** `/logs`  
**Layout:** Full width  
**Real-time:** ❌ (search/filters trigger new queries)

#### Sections

**A. Header**
- Title: "Request Logs"
- Subtitle: "Search and analyze proxy requests"
- Actions:
  - Export button (CSV/JSON)
  - "Live Tail" toggle (auto-refresh every 5s)

**B. Filters Bar**
- Search input: "Search by request ID, model, or content..."
- Filters (collapsible):
  - Date range (picker: last 1h, 24h, 7d, 30d, custom)
  - Model (multi-select)
  - Provider (multi-select)
  - Status (dropdown: All, Success, Error, Rate Limit)
  - API Key (multi-select)
  - Organization (multi-select)
  - Team (multi-select)
  - Tags (multi-select, free text)
  - Latency (range slider: 0ms → 10000ms+)
  - Cost (range: $0 → $100+)
- "Apply Filters" button
- "Clear All" link

**C. Logs Table**

| Column | Format | Notes |
|--------|--------|-------|
| Timestamp | `2026-08-07 10:42:15` | Sortable |
| Request ID | `req_8f73b2a1` (monospace) | Click → detail |
| Model | `gpt-4-turbo` | |
| Provider | `openai` | |
| Status | Badge (200, 429, 500, etc.) | Color-coded |
| Tokens | `1.2K / 3.4K` (input / output) | |
| Cost | `$0.0456` | |
| Latency | `342ms` (monospace) | Sortable |
| Actions | View detail (icon) | |

- Pagination: 50 rows per page (configurable: 20, 50, 100)
- Sort: Click column headers (timestamp DESC by default)

**D. Log Detail Modal**
When clicking a request ID:

- **Overview:**
  - Request ID, timestamp, model, provider
  - Status code, latency, total cost
  - API key (masked), org, team, user
  
- **Request:**
  - Full request body (JSON, syntax highlighted)
  - Headers (sensitive headers masked)
  - Messages (for chat completions)
  
- **Response:**
  - Full response body (JSON)
  - Usage (prompt tokens, completion tokens, total)
  - Finish reason
  
- **Timeline:**
  - Visual timeline:
    - Received: `10:42:15.000`
    - Forwarded to provider: `10:42:15.050`
    - First token: `10:42:15.450`
    - Completed: `10:42:15.850`
  - Total: `850ms`
  
- **Guardrails:**
  - PII detected: `Yes (email, phone)` → show masked content
  - Moderation: `Passed` or `Blocked (category: violence)`
  - Fallback: `No` or `Yes (from openai/gpt-4 → anthropic/claude-3)`

- **Actions:**
  - Copy request ID
  - Download JSON
  - Retry request (for admins)

**API Endpoints Required:**
```
GET /api/v1/logs                      # List logs (with filters, pagination)
GET /api/v1/logs/:id                  # Get single log detail
GET /api/v1/logs/export               # Export logs (CSV/JSON)
GET /api/v1/logs/filters              # Get available filter values (models, providers, etc.)
WS /api/v1/logs/live                  # Live tail (optional, for "Live Tail" mode)
```

---

### 4.5 Usage (`/usage`)

**URL:** `/usage`  
**Layout:** Full width  
**Real-time:** ❌ (data aggregated, refresh on navigation)

#### Sections

**A. Header**
- Title: "Usage Analytics"
- Subtitle: "Track spend, tokens, and requests across your organization"
- Actions:
  - Date range picker (30d, 90d, YTD, custom)
  - Export button (CSV/PDF)

**B. Summary Cards (4)**

| Metric | Format | Breakdown |
|--------|--------|-----------|
| Total Spend | `$12,345.67` | vs last period: `+12.5%` |
| Total Requests | `1.24M` | vs last period: `+8.3%` |
| Total Tokens | `845M` | Input: `500M`, Output: `345M` |
| Avg Cost per 1K tokens | `$0.0146` | vs last period: `-2.1%` |

**C. Spend Over Time (Chart)**
- Type: Area chart (stacked by org or team)
- X-axis: Date (daily granularity)
- Y-axis: Spend in USD
- Legend: Click to toggle orgs/teams
- Tooltip: Exact spend, breakdown by provider

**D. Breakdown Charts (3 columns)**

1. **Spend by Organization** (Pie chart)
   - Segments: Orgs (or "Unassigned" if no org)
   - Tooltip: `$ amount`, `% of total`
   - Click segment → filter table below

2. **Spend by Provider** (Bar chart)
   - X-axis: Providers (OpenAI, Anthropic, Gemini, etc.)
   - Y-axis: `$ amount`
   - Color: Provider brand color
   - Tooltip: `$ amount`, `% of total`

3. **Top 10 Models** (Horizontal bar chart)
   - Y-axis: Model names
   - X-axis: `$ amount` (or tokens)
   - Sort: DESC by spend
   - Tooltip: Model, spend, tokens

**E. Token Usage Breakdown**
- Type: Stacked area chart
- X-axis: Date
- Y-axis: Tokens (millions)
- Stacks: Input tokens, Output tokens, Cache tokens
- Tooltip: Exact counts per type

**F. Detailed Spend Table**

| Column | Format | Notes |
|--------|--------|-------|
| Organization | Name (or "Unassigned") | Click → filter |
| Team | Name (or "—") | |
| API Key | Prefix + name | Click → key detail |
| Spend | `$1,234.56` | Sortable |
| Requests | `45.2K` | Sortable |
| Tokens | `845M` (input/output) | Sortable |
| Avg Cost/1K | `$0.0146` | Calculated |
| Budget | `$5,000` (if set) | Progress bar: `24.7%` |
| Actions | View detail | |

- Sort: By spend (DESC default)
- Pagination: 20 rows per page

**G. Budget Progress Section**
- For each org/team/key with a budget:
  - Progress bar: `███░░░░░░░ 24.7%`
  - Labels: `$1,234.56 / $5,000.00`
  - Status: Green (<80%), Yellow (80-99%), Red (≥100%)
  - Alert: "Budget exceeded" if >100%

**API Endpoints Required:**
```
GET /api/v1/usage/summary               # Total spend, requests, tokens
GET /api/v1/usage/timeseries            # Spend over time (daily)
GET /api/v1/usage/by-org                # Breakdown by organization
GET /api/v1/usage/by-provider           # Breakdown by provider
GET /api/v1/usage/by-model              # Breakdown by model
GET /api/v1/usage/by-key                # Detailed table data
GET /api/v1/usage/budgets               # Budget progress for all entities
GET /api/v1/usage/tokens                # Token usage breakdown (input/output/cache)
```

---

### 4.6 Guardrails (`/guardrails`)

**URL:** `/guardrails`  
**Layout:** Full width  
**Real-time:** ❌ (config changes are immediate, no real-time needed)

#### Sections

**A. Header**
- Title: "Guardrails"
- Subtitle: "Configure PII masking, moderation, and budget alerts"
- Actions:
  - "Save Changes" button (sticky, appears on any change)
  - "Reset to Defaults" link

**B. PII Masking Card**

**Toggle:** `Enable PII Masking` (on/off switch)

**Patterns Table:**

| Pattern | Regex | Replacement | Enabled | Actions |
|---------|-------|-------------|---------|---------|
| Email | `[a-zA-Z0-9._%+-]+@...` | `[EMAIL]` | ✅ | Edit, Delete |
| Phone | `\+?[\d\s-()]{10,}` | `[PHONE]` | ✅ | Edit, Delete |
| Credit Card | `\b\d{4}[-\s]?\d{4}...` | `[CARD]` | ✅ | Edit, Delete |
| SSN | `\b\d{3}-\d{2}-\d{4}\b` | `[SSN]` | ✅ | Edit, Delete |

- "Add Custom Pattern" button → Modal (name, regex, replacement)
- Test input: "Test your regex" → shows matches in real-time

**Settings:**
- Mask request content: ✅ (checkbox)
- Mask response content: ✅ (checkbox)
- Log masked content: ❌ (checkbox, warning: "Privacy implication")

---

**C. Moderation Card**

**Pre-Call Moderation:**
- Toggle: `Enable pre-call moderation` (on/off)
- Provider dropdown: "Select moderation provider" (OpenAI, Perspective API, etc.)
- Model dropdown: "Select model" (text-moderation-latest, etc.)
- Block on failure: ✅ (checkbox)
- Categories to check (checkboxes):
  - ✅ Hate speech
  - ✅ Harassment
  - ✅ Self-harm
  - ✅ Sexual content
  - ✅ Violence
  - ❌ Profanity (optional)

**Post-Call Moderation:**
- Toggle: `Enable post-call moderation` (on/off)
- Same provider/model config
- Block on failure: ✅ (checkbox)
- Log blocked responses: ✅ (checkbox)

**Stats (last 24h):**
- Pre-call blocked: `23 requests`
- Post-call blocked: `5 responses`
- Most common category: `Hate speech (45%)`

---

**D. Budget Alerts Card**

**Global Settings:**
- Default threshold: `80%` (input field, slider)
- Check interval: `5 minutes` (dropdown: 1, 5, 15, 30, 60)
- Max retries: `5` (input field)

**Alert Config Table:**

| Entity | Type | Threshold | Webhook URL | Status | Actions |
|--------|------|-----------|-------------|--------|---------|
| Acme Corp | Org | 80% | `https://hooks.slack.com/...` | ✅ Active | Edit, Delete |
| Team Alpha | Team | 90% | `https://api.example.com/alerts` | ✅ Active | Edit, Delete |
| Production Key | API Key | 100% | `https://hooks.slack.com/...` | ❌ Disabled | Edit, Delete |

- "Add Alert" button → Modal:
  - Entity type (Org/Team/API Key)
  - Entity selector (dropdown)
  - Threshold (%, slider + input)
  - Webhook URL (input, validation)
  - Enable/disable toggle

**Recent Alerts (Table):**
- Timestamp, Entity, Threshold, Current Spend, Status (Sent/Failed)
- "View All Alerts" link

---

**E. Fallback Stats Card**

**Fallback Chains Overview:**
- Total fallbacks triggered (24h): `47`
- Most common fallback: `openai/gpt-4 → anthropic/claude-3-opus (23 times)`
- Avg latency added by fallback: `+234ms`

**Fallback Chains Table:**

| Primary | Fallback 1 | Fallback 2 | Triggered (24h) | Success Rate | Actions |
|---------|------------|------------|-----------------|--------------|---------|
| gpt-4 | claude-3-opus | gemini-1.5-pro | 23 | 95.7% | View, Edit |
| gpt-3.5-turbo | claude-3-haiku | — | 12 | 100% | View, Edit |

- Click "View" → Modal with timeline of fallback events
- Click "Edit" → Redirect to `/providers` fallback chain builder

**API Endpoints Required:**
```
GET /api/v1/guardrails/config          # Get full guardrails config
PUT /api/v1/guardrails/config          # Update guardrails config
GET /api/v1/guardrails/pii/patterns    # List PII patterns
POST /api/v1/guardrails/pii/patterns   # Add PII pattern
PUT /api/v1/guardrails/pii/patterns/:id # Update pattern
DELETE /api/v1/guardrails/pii/patterns/:id # Delete pattern
GET /api/v1/guardrails/moderation/stats # Moderation stats (blocked counts)
GET /api/v1/guardrails/alerts          # List alert configs
POST /api/v1/guardrails/alerts         # Create alert config
PUT /api/v1/guardrails/alerts/:id      # Update alert
DELETE /api/v1/guardrails/alerts/:id   # Delete alert
GET /api/v1/guardrails/alerts/history  # Recent alerts sent
GET /api/v1/guardrails/fallbacks/stats # Fallback statistics
```

---

### 4.7 Models (`/models`)

**URL:** `/models`  
**Layout:** Full width  
**Real-time:** ❌ (pricing sync is manual/background)

#### Sections

**A. Header**
- Title: "Model Catalog"
- Subtitle: "Browse and configure available models"
- Actions:
  - "Sync Pricing" button (triggers GitHub sync)
  - "Add Custom Model" button → Modal
  - Search input: "Search models..."

**B. Filters**
- Provider (multi-select: OpenAI, Anthropic, Gemini, etc.)
- Capabilities (checkboxes):
  - ✅ Tool calling
  - ✅ Vision
  - ✅ Streaming
  - ✅ Prompt caching
- Context window (range slider: 4K → 2M tokens)
- Pricing (range: $0 → $100 per 1M tokens)

**C. Model Grid (Cards)**

Each card displays:

- **Header:**
  - Model name (e.g., `gpt-4-turbo`)
  - Provider logo + name
  - Status badge (Available, Deprecated, Coming Soon)
  
- **Capabilities (Badges):**
  - `Tool Calling` `Vision` `Streaming` `Cache`
  
- **Specs:**
  - Context: `128K tokens`
  - Max output: `4K tokens`
  - Knowledge cutoff: `Dec 2023`
  
- **Pricing:**
  - Input: `$10.00 / 1M tokens`
  - Output: `$30.00 / 1M tokens`
  - Cache read: `$1.00 / 1M tokens`
  - Cache write: `$1.25 / 1M tokens`
  
- **Actions:**
  - Toggle: Enable/Disable model
  - "Configure" button → Modal (pricing overrides, aliases)

**D. Model Detail Modal**
When clicking "Configure":

- **Overview:**
  - Name, provider, description
  - Context window, max output
  - Supported capabilities
  
- **Pricing:**
  - Default pricing (from provider)
  - Override inputs (if custom pricing needed)
  - Calculated example: "1K input + 500 output = `$0.025`"
  
- **Aliases:**
  - List of aliases (e.g., `gpt-4-turbo` → `gpt-4-0125-preview`)
  - "Add Alias" button
  
- **Access Control:**
  - Which API keys can access this model
  - "Restrict to specific keys" toggle
  - Key selector (multi-select)
  
- **Usage Stats (24h):**
  - Requests: `1.2K`
  - Tokens: `45M`
  - Spend: `$567.89`

**E. Add Custom Model Modal**
- Form fields:
  - Model ID (required, e.g., `my-custom-model`)
  - Provider (dropdown)
  - Context window (input, tokens)
  - Max output (input, tokens)
  - Pricing (input/output/cache per 1M tokens)
  - Capabilities (checkboxes)
  - Description (textarea)

**API Endpoints Required:**
```
GET /api/v1/models                    # List all models (with filters)
GET /api/v1/models/:id                # Get model details
PUT /api/v1/models/:id                # Update model (enable/disable, pricing)
POST /api/v1/models                   # Add custom model
GET /api/v1/models/:id/usage          # Usage stats for model
GET /api/v1/models/capabilities       # List all capabilities
POST /api/v1/models/sync-pricing      # Trigger pricing sync from GitHub
```

---

### 4.8 Settings (`/settings`)

**URL:** `/settings`  
**Layout:** Full width  
**Real-time:** ❌ (config changes are immediate)

#### Sections

**A. Header**
- Title: "Settings"
- Subtitle: "Configure global proxy settings"
- Actions:
  - "Save Changes" button (sticky)
  - "Reset to Defaults" link

**B. General Settings Card**

- **Instance Name:** Input (e.g., "Production LLM Proxy")
- **Timezone:** Dropdown (e.g., "UTC", "America/New_York")
- **Date Format:** Dropdown (e.g., "YYYY-MM-DD", "MM/DD/YYYY")
- **Currency:** Dropdown (e.g., "USD", "EUR")
- **Default Language:** Dropdown (e.g., "en-US", "fr-FR")

---

**C. Authentication Card**

- **Master Key:** Display (masked) + "Rotate" button
- **OIDC/SAML:**
  - Toggle: `Enable SSO`
  - Provider (dropdown: Okta, Azure AD, Google, Generic OIDC)
  - Issuer URL
  - Client ID
  - Client Secret (masked)
  - Redirect URI (auto-generated)
  - "Test Connection" button
- **API Key Auth:**
  - Toggle: `Require API keys for all requests`
  - Default scopes for new keys

---

**D. Rate Limits Card**

- **Global Rate Limits:**
  - Requests per minute (input)
  - Tokens per minute (input)
  - Per API key (toggle)
  - Per organization (toggle)
  - Per team (toggle)
  
- **Burst Allowance:**
  - Enable burst: ✅ (checkbox)
  - Burst multiplier: `2x` (input)
  
- **Rate Limit Response:**
  - Status code: `429` (dropdown: 429, 503)
  - Retry-After header: ✅ (checkbox)
  - Custom error message: (textarea)

---

**E. Webhooks Card**

**Webhook Endpoints Table:**

| URL | Events | Status | Last Triggered | Actions |
|-----|--------|--------|----------------|---------|
| `https://api.example.com/hooks` | budget.alert, model.error | ✅ Active | 2m ago | Edit, Delete, Test |
| `https://hooks.slack.com/...` | guardrails.blocked | ❌ Disabled | 1d ago | Edit, Delete, Test |

- "Add Webhook" button → Modal:
  - URL (input, validation)
  - Events (checkboxes: budget.alert, model.error, guardrails.blocked, provider.down)
  - Secret (for HMAC signature)
  - Enable/disable toggle
  - "Send Test Event" button

**Delivery Logs:**
- Recent webhook deliveries (success/failure, response code, latency)
- "View All Logs" link

---

**F. Caching Card**

- **Enable Caching:** Toggle
- **Cache Backend:** Dropdown (Redis, In-Memory, Database)
- **Redis Config:**
  - Host, Port, Password (masked)
  - DB index
  - Connection pool size
- **Cache TTL:** Input (seconds, default: 3600)
- **Cache Keys:**
  - Cache by: (checkboxes) Model, Messages, Temperature, Max Tokens
  - Ignore: (checkboxes) User, Metadata
- **Cache Stats:**
  - Hit rate: `67%`
  - Entries: `12,345`
  - Memory usage: `234 MB`

---

**G. Logging & Auditing Card**

- **Request Logging:**
  - Enable: ✅
  - Log level: Dropdown (DEBUG, INFO, WARN, ERROR)
  - Log request body: ✅ (warning: privacy)
  - Log response body: ✅ (warning: privacy)
  - Retention: Dropdown (7d, 30d, 90d, 1y, indefinite)
  
- **Audit Log:**
  - Enable: ✅
  - Log config changes: ✅
  - Log API key creation/deletion: ✅
  - Log admin actions: ✅
  - Export audit log: Button (CSV/JSON)

---

**H. Advanced Settings Card**

- **Debug Mode:** Toggle (verbose logging, stack traces)
- **CORS:**
  - Allowed origins: Input (comma-separated)
  - Allow credentials: ✅
- **Custom Headers:**
  - Add header: Key + Value inputs
  - List of custom headers (with delete)
- **Feature Flags:**
  - Enable new routing engine: ❌ (beta)
  - Enable semantic caching: ❌ (beta)
  - Enable A/B testing: ❌ (alpha)

**API Endpoints Required:**
```
GET /api/v1/settings                  # Get all settings
PUT /api/v1/settings                  # Update settings
GET /api/v1/settings/auth             # Auth config
PUT /api/v1/settings/auth             # Update auth config
GET /api/v1/settings/rate-limits      # Rate limit config
PUT /api/v1/settings/rate-limits      # Update rate limits
GET /api/v1/settings/webhooks         # List webhooks
POST /api/v1/settings/webhooks        # Create webhook
PUT /api/v1/settings/webhooks/:id     # Update webhook
DELETE /api/v1/settings/webhooks/:id  # Delete webhook
POST /api/v1/settings/webhooks/:id/test # Send test event
GET /api/v1/settings/webhooks/logs    # Webhook delivery logs
GET /api/v1/settings/cache            # Cache config
PUT /api/v1/settings/cache            # Update cache config
GET /api/v1/settings/logging          # Logging config
PUT /api/v1/settings/logging          # Update logging config
```

---

## 5. API Contract

### 5.1 Base URL

```
Production: https://your-godwit-instance.com/api/v1
Local:      http://localhost:3000/api/v1
```

### 5.2 Authentication

All endpoints require authentication via:

```
Authorization: Bearer <api_key>
```

Or for admin endpoints:

```
Authorization: Bearer <master_key>
```

### 5.3 Response Format

**Success:**
```json
{
  "data": { ... },
  "meta": {
    "page": 1,
    "per_page": 50,
    "total": 1234
  }
}
```

**Error:**
```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Field 'name' is required",
    "details": [ ... ]
  }
}
```

### 5.4 WebSocket Protocol

**Connection:**
```
ws://localhost:3000/api/v1/ws/metrics?token=<jwt_token>
```

**Client → Server Messages:**
```json
{
  "type": "subscribe",
  "channel": "metrics",
  "filters": {
    "org_id": "uuid",
    "models": ["gpt-4", "claude-3"]
  }
}
```

**Server → Client Messages:**
```json
{
  "type": "metrics:update",
  "data": {
    "requests_total": 1234567,
    "latency_p95_ms": 342,
    "tokens_total": 845000000,
    "error_rate": 0.0004,
    "timestamp": "2026-08-07T10:42:15Z"
  }
}
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

- **Components:** Render tests (React Testing Library)
  - Button clicks, form submissions
  - Props validation
  - Conditional rendering
  
- **Hooks:** Logic tests
  - Data fetching (mock API)
  - State updates
  - Error handling
  
- **Utils:** Pure function tests
  - Date formatting
  - Number formatting (currency, tokens)
  - Validation functions

### 6.2 Integration Tests

- **API Integration:**
  - Mock server (MSW - Mock Service Worker)
  - Test data fetching, error states, loading states
  
- **WebSocket Integration:**
  - Mock WebSocket server
  - Test connection, subscription, message handling
  
- **Form Submissions:**
  - End-to-end form flow (fill → submit → success/error)

### 6.3 E2E Tests

- **Playwright:**
  - Critical user journeys:
    - Login → Dashboard → View metrics
    - Create API key → Use key → View usage
    - Configure provider → Test connection → Send request
    - Set up budget alert → Trigger alert → Receive webhook
  - Cross-browser (Chrome, Firefox, Safari)
  - Mobile responsive tests

### 6.4 Performance Tests

- **Lighthouse:**
  - Target scores: Performance ≥90, Accessibility ≥90, Best Practices ≥90, SEO ≥90
  
- **Bundle Size:**
  - Target: <500KB initial load, <2MB total
  
- **Time to Interactive:**
  - Target: <3s on 3G, <1s on broadband

---

## 7. Deployment

### 7.1 Build Process

```bash
# Install dependencies
npm install

# Type check
npm run type-check

# Lint
npm run lint

# Test
npm run test

# Build
npm run build

# Output: .next/ directory (static files + server bundle)
```

### 7.2 Docker Image

```dockerfile
# Multi-stage build
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/public ./public
COPY --from=builder /app/package*.json ./
RUN npm ci --only=production

EXPOSE 3000
CMD ["npm", "start"]
```

### 7.3 Environment Variables

```bash
# Required
NEXT_PUBLIC_API_URL=http://localhost:3000/api/v1
NEXT_PUBLIC_WS_URL=ws://localhost:3000/api/v1/ws

# Optional
NEXT_PUBLIC_SENTRY_DSN=https://...
NEXT_PUBLIC_ANALYTICS_ID=UA-XXXXX-Y
```

### 7.4 Hosting Options

**Option A: Vercel (Recommended for simplicity)**
- Zero-config deployment
- Automatic preview deployments for PRs
- Edge functions for low latency
- Cost: Free tier → $20/mo (Pro)

**Option B: Docker + Kubernetes**
- Full control over infrastructure
- Can run alongside Godwit backend
- Requires more DevOps overhead

**Option C: Mono-repo (apps/ui/ in Godwit)**
- Single deployment unit
- Shared CI/CD pipeline
- Easier to keep frontend/backend in sync

---

## 8. Rollout Plan

### Phase 1: MVP (Week 1-2)
- ✅ Dashboard Overview (read-only metrics)
- ✅ Providers list + basic config
- ✅ API Keys CRUD
- ✅ Logs viewer (no advanced filters)

### Phase 2: Analytics (Week 3-4)
- ✅ Usage analytics (spend, tokens, breakdowns)
- ✅ Advanced log filters + search
- ✅ Model catalog

### Phase 3: Guardrails (Week 5-6)
- ✅ PII masking config
- ✅ Moderation config
- ✅ Budget alerts + webhooks

### Phase 4: Polish (Week 7-8)
- ✅ Settings page (auth, rate limits, caching)
- ✅ Real-time WebSocket updates
- ✅ Mobile responsive improvements
- ✅ Performance optimization

### Phase 5: Production (Week 9+)
- ✅ E2E tests
- ✅ Security audit
- ✅ Documentation
- ✅ User training

---

## 9. Success Metrics

- **Adoption:** % of admins using UI vs CLI/config files (target: 80% in 3 months)
- **Efficiency:** Time to configure a new provider (target: <2 minutes)
- **Reliability:** Time to detect provider outage (target: <1 minute via dashboard)
- **Satisfaction:** User survey score (target: ≥4.5/5)

---

## 10. Open Questions

1. **Multi-tenant support:** Should the UI support multiple organizations with separate logins? (Currently: single instance, multi-org in DB)
2. **White-labeling:** Should companies be able to customize branding (logo, colors)?
3. **Playground:** Add a "Try it out" section for testing models with a chat UI?
4. **Mobile app:** Native mobile app for alerts/monitoring, or responsive web only?

---

## Appendix A: Component Inventory

**Base Components (30+):**
- Button (4 variants)
- Input (text, password, number, textarea)
- Select (single, multi)
- Checkbox, Radio, Toggle
- Badge, Status Chip
- Card, Modal, Drawer
- Table (sortable, filterable, paginated)
- Tabs, Accordion
- Tooltip, Popover
- Toast/Notification
- Loading Spinner, Skeleton
- Empty State, Error State
- Pagination, Breadcrumbs

**Domain Components (50+):**
- MetricCard, TimeSeriesChart, BreakdownChart
- ProviderList, ProviderForm, FallbackChainBuilder
- KeyList, KeyForm, KeyDetails, UsageTable
- LogsTable, LogFilters, LogDetail
- SpendChart, TokenBreakdown, BudgetProgress
- PiiConfig, ModerationConfig, BudgetAlerts
- ModelCatalog, ModelCard, ModelForm
- Sidebar, Header, Shell, MobileNav

**Total estimated components: 80+**

---

## Appendix B: API Endpoint Summary

**Total endpoints needed: 50+**

| Category | Endpoints | Count |
|----------|-----------|-------|
| Metrics | `/metrics/*` | 5 |
| Providers | `/providers/*` | 8 |
| API Keys | `/keys/*` | 7 |
| Logs | `/logs/*` | 5 |
| Usage | `/usage/*` | 8 |
| Guardrails | `/guardrails/*` | 10 |
| Models | `/models/*` | 6 |
| Settings | `/settings/*` | 10 |
| WebSocket | `/ws/*` | 2 |

---

**END OF DESIGN SPEC**
