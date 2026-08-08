# Admin UI Frontend Foundations Design

> **For agentic workers:** This spec is for phase B of the admin UI project. Phase A (backend completion) is done. Phase C (per-resource screens) will follow this foundation. Use this as the authoritative requirements doc before implementing.

**Goal:** Build a scalable frontend foundation for the admin dashboard using Next.js, shadecn, and TypeScript. This phase establishes the architecture, reusable components, auth flow, and testing patterns that all future admin screens will follow.

**Architecture:** Three-tier component system (low-level primitives → mid-level smart components → resource pages) ensures minimal duplication as new admin screens are added. Server-side data fetching with Next.js Server Components keeps authentication and API calls secure. RBAC scoping (super_admin sees all, org_admin sees their org) is baked into every page.

**Tech Stack:**
- Next.js 14+ (App Router, Server Components, Server Actions)
- shadecn/ui (React component library on Tailwind)
- Recharts (data visualization for graphs)
- TanStack Table (powerful table component)
- TypeScript (full type safety)
- Vitest + React Testing Library (unit/component tests)
- Playwright (E2E tests)

## 1. Architecture

### Three-Tier Component System

**Tier 1 — Low-level Primitives** (`/components/ui/`)
Generic, data-agnostic components built on shadecn:
- `<DataTable>` — table with sorting, filtering, pagination (uses TanStack Table)
- `<FormDialog>` — modal form container with loading/error states
- `<PageHeader>` — page title + optional action button
- `<EmptyState>` — "no data" placeholder UI
- Standard shadecn components (Button, Input, Select, Card, etc.)

**Tier 2 — Smart Mid-level Components** (`/components/admin/`)
Compose Tier 1 components with common admin patterns:
- `<ListPage>` — wraps `PageHeader` + `DataTable` + `EmptyState`; handles no-data case
- `<EditDialog>` — wraps `FormDialog` with loading state, error display, success feedback
- `<ResourceForm>` — base form template with common fields (name, role, status, dates)

**Tier 3 — Resource Pages** (`/app/admin/[resource]/page.tsx`)
Each admin screen (organizations, teams, users, etc.) composes Tier 2 components:
- Fetch data server-side
- Define columns/fields specific to the resource
- Render `<ListPage>` or `<EditDialog>` with minimal custom logic

### Data Flow

1. **Page (Server Component)** → fetches data via `fetchXxx()` helper → passes to client component
2. **Client Component** → renders UI, handles user interactions
3. **Form submission** → Server Action → backend API call → revalidate page data
4. **Error/loading states** → managed by components, not scattered across pages

### RBAC Scoping

Every page and API call respects the user's role:
- **super_admin**: unfiltered access, sees all organizations/teams/users/activity
- **org_admin**: scoped to their organization (all teams, users, activity within that org)
- **team_admin / user**: no dashboard access (redirected to `/`)

Scoping is applied:
- At the route level (middleware checks role, redirects if unauthorized)
- At the data-fetch level (API calls include `?organization_id=` parameter based on user's org)
- Consistently across all pages (no special-case scoping logic per page)

## 2. Folder Structure

```
godwit/
├── apps/
│   └── admin/                          # Next.js admin app (new)
│       ├── app/
│       │   ├── (auth)/
│       │   │   ├── login/
│       │   │   │   ├── page.tsx        # login page
│       │   │   │   └── page.module.css
│       │   │   ├── auth/
│       │   │   │   └── callback/page.tsx # OIDC callback
│       │   │   └── layout.tsx          # auth layout (no sidebar)
│       │   ├── (dashboard)/
│       │   │   ├── layout.tsx          # dashboard layout (sidebar + nav)
│       │   │   ├── page.tsx            # dashboard home
│       │   │   └── admin/
│       │   │       ├── organizations/
│       │   │       │   ├── page.tsx    # list page
│       │   │       │   └── [id]/page.tsx # detail/edit page
│       │   │       ├── teams/
│       │   │       ├── users/
│       │   │       ├── api-keys/
│       │   │       ├── models/
│       │   │       └── spend/
│       │   ├── layout.tsx              # root layout
│       │   └── page.tsx                # / → redirect to /login
│       ├── components/
│       │   ├── ui/                     # shadecn + primitives
│       │   │   ├── data-table.tsx
│       │   │   ├── form-dialog.tsx
│       │   │   ├── page-header.tsx
│       │   │   ├── empty-state.tsx
│       │   │   └── [shadecn buttons, inputs, selects, etc.]
│       │   ├── admin/                  # smart components
│       │   │   ├── list-page.tsx
│       │   │   ├── edit-dialog.tsx
│       │   │   └── resource-form.tsx
│       │   └── layout/
│       │       ├── sidebar.tsx
│       │       ├── top-bar.tsx
│       │       └── nav.tsx
│       ├── lib/
│       │   ├── api-client.ts           # fetch wrapper, auto-refresh tokens
│       │   ├── auth.ts                 # JWT, cookie handling
│       │   ├── hooks.ts                # useUser, useOrganization, etc.
│       │   └── utils.ts                # formatting, validation
│       ├── middleware.ts               # auth + RBAC checks
│       ├── env.ts                      # environment variables
│       ├── types.ts                    # TypeScript types (mirrored from backend)
│       ├── public/
│       ├── package.json
│       ├── next.config.ts
│       ├── tsconfig.json
│       └── tailwind.config.ts
├── [rest of Godwit monorepo]
└── docs/
```

**Key patterns:**
- `(auth)` and `(dashboard)` are Route Groups — auth pages have no sidebar; dashboard pages do
- Each resource folder (`organizations/`, `teams/`, etc.) contains list + detail pages
- `/lib` centralizes API, auth, custom hooks, types
- `/components/layout` contains the shared dashboard shell (sidebar, nav, top bar)

## 3. Authentication Flow

### Login Page (`/app/(auth)/login/page.tsx`)

Conditionally renders:
1. **Password form** (if enabled in backend config)
   - Email + password inputs
   - Submit button
   - "Forgot password?" link (future: password reset)

2. **SSO button** (if enabled in backend config)
   - "Sign in with [Provider]" button
   - Redirects to OIDC provider

3. **Error state** (if both disabled)
   - "Sign-in is not configured" message

### Password Authentication Flow

1. User submits email + password → `POST /api/v1/auth/login`
2. Backend returns `{ access_token, refresh_token }`
3. **Server Action** stores both in httpOnly cookies (secure, cannot be accessed by JavaScript)
4. Redirect to `/admin` (dashboard home)

### SSO (OIDC) Flow

1. User clicks "Sign in with Google" (or other provider)
2. Redirects to provider's authorization endpoint (handled by Next.js or a library)
3. Provider redirects back to `/auth/callback?code=XXX&state=YYY`
4. **Server Action** exchanges code for tokens via `POST /api/v1/auth/oidc/callback` (backend endpoint)
5. Backend auto-provisions user if first-time login, returns tokens
6. Store tokens in httpOnly cookies
7. Redirect to `/admin`

### Token Management

**httpOnly Cookies:**
- `access_token`: JWT, short-lived (15 minutes), cleared on logout
- `refresh_token`: SHA-256 hash, long-lived (7 days), rotating (single-use)
- Secure flag: only sent over HTTPS
- SameSite: strict (CSRF protection)

**Auto-refresh:**
- Middleware intercepts requests to `/admin/*`
- If `access_token` is expired but `refresh_token` exists, silently refresh via `POST /api/v1/auth/refresh`
- Store new tokens in cookies
- Retry the original request
- User never sees a login prompt (transparent)

**Logout:**
- Clear both cookies
- `POST /api/v1/auth/logout` (backend invalidates refresh token)
- Redirect to `/login`

### Protected Routes

**Middleware** (`/middleware.ts`):
- Checks every request to `/admin/*`, `/auth/*`, `/`
- Verifies `access_token` exists and is valid
- If expired: tries to refresh (if `refresh_token` exists)
- If invalid: redirects to `/login`
- If both missing: redirects to `/login`

**Route-level checks:**
- Dashboard pages check user's role
- If role is not `super_admin` or `org_admin`, redirect to `/` (not authorized for dashboard)

## 4. Data Fetching & API Integration

### Server-Side Fetching (Next.js Server Components)

Default pattern — all pages use Server Components to fetch data before rendering:

```typescript
// app/admin/organizations/page.tsx
export default async function OrganizationsPage() {
  const organizations = await fetchOrganizations()
  
  return (
    <ListPage
      data={organizations}
      columns={[...]}
      title="Organizations"
      onCreateClick={() => {}}
    />
  )
}

// lib/api-client.ts
export async function fetchOrganizations() {
  const response = await fetch('https://api.godwit.io/api/v1/organizations', {
    headers: {
      'Authorization': `Bearer ${getAccessToken()}`,
    },
  })
  
  if (response.status === 401) {
    // Token expired, refresh and retry
    await refreshToken()
    return fetchOrganizations() // recursive retry
  }
  
  if (!response.ok) {
    throw new Error(`Failed to fetch organizations: ${response.statusText}`)
  }
  
  return response.json()
}

function getAccessToken() {
  // Read from cookies (server-side)
  return cookies().get('access_token')?.value
}
```

**Benefits:**
- Data is fetched before the page renders (no loading spinners on page load)
- Tokens are never exposed to the browser (stored in httpOnly cookies, read only on server)
- Type-safe: `fetchOrganizations()` returns `Promise<Organization[]>`
- Errors are caught on the server; user sees error page, not a broken UI

### Client-Side Interactions (Server Actions)

When users submit forms (create, update, delete), use Server Actions:

```typescript
// app/admin/organizations/actions.ts
'use server'

export async function createOrganization(formData: FormData) {
  const name = formData.get('name') as string
  
  const response = await fetch('https://api.godwit.io/api/v1/organizations', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${getAccessToken()}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ name }),
  })
  
  if (!response.ok) {
    throw new Error('Failed to create organization')
  }
  
  // Revalidate the list page so it fetches fresh data
  revalidatePath('/admin/organizations')
  
  return response.json()
}

// app/admin/organizations/page.tsx
export default async function OrganizationsPage() {
  return (
    <OrganizationForm onSubmit={createOrganization} />
  )
}
```

**Benefits:**
- Form submission is type-safe (no manual JSON serialization)
- Tokens are read on the server (never passed to client)
- Revalidation keeps data fresh automatically
- No client-side state management needed

### RBAC Scoping in API Calls

Every API call includes the user's organization scope (if not super_admin):

```typescript
export async function fetchOrganizations() {
  const user = await getCurrentUser()
  
  let url = 'https://api.godwit.io/api/v1/organizations'
  
  // If org_admin, filter to their org only
  if (user.role === 'org_admin') {
    url += `?organization_id=${user.organization_id}`
  }
  // If super_admin, fetch all (no filter)
  
  return fetch(url, { headers: { 'Authorization': ... } })
}
```

Same pattern applies to teams, users, spend, etc. — every call respects the caller's org.

## 5. Dashboard Home Page

**Route:** `/app/(dashboard)/page.tsx`

**Role check:** Only `super_admin` and `org_admin` can access. Redirect to `/` if user is `team_admin` or `user`.

**Content:**

1. **Quick stats cards** (server-fetched):
   - Total organizations (super_admin sees system-wide; org_admin sees their org)
   - Total teams (scoped same way)
   - Total users (scoped same way)
   - Total API keys (scoped same way)

2. **Spend graph** (Recharts):
   - Fetch last 30 days via `GET /api/v1/spend?from=...&to=...` (scoped to user's org)
   - Line chart: date on X-axis, cost_usd on Y-axis
   - Interactive (hover for exact values)

3. **Recent activity** (table):
   - Last 5 organizations created (if super_admin) OR within their org (if org_admin)
   - Last 5 teams created (scoped)
   - Last 5 users created (scoped)
   - Columns: name, created_at, link to resource

4. **Quick navigation cards:**
   - Links to `/admin/organizations`, `/admin/teams`, `/admin/users`, `/admin/api-keys`, `/admin/models`, `/admin/spend`
   - Click card to navigate

**Purpose:**
- Landing page after login
- Shows system health at a glance (stats + spend trend)
- Entry point to each admin section

## 6. Sidebar Navigation

**Sidebar** (`/components/layout/sidebar.tsx`):

Links (always visible):
- Dashboard (home)
- Organizations
- Teams
- Users
- API Keys
- Models
- Spend

**Current section highlight:** Bold/highlighted link for the current page

**User menu** (top-right of sidebar or top bar):
- User's name + email
- "Settings" link (future: edit profile, change password)
- "Logout" button

## 7. Reusable Components Deep Dive

### `<DataTable>`

```typescript
export function DataTable<T>({
  columns: ColumnDef<T>[],
  data: T[],
  isLoading?: boolean,
  pageSize?: number,
  onRowClick?: (row: T) => void,
  selectable?: boolean, // multi-select rows
}) {
  // Uses TanStack Table for:
  // - Sorting (click column headers)
  // - Filtering (search input, date ranges)
  // - Pagination (next/prev, jump to page)
  // - Row selection (if selectable=true)
  
  return <table>{/* ... */}</table>
}
```

**Used by:** `<ListPage>`, any page needing to display a list of items

### `<FormDialog>`

```typescript
export function FormDialog({
  isOpen: boolean,
  title: string,
  description?: string,
  children: ReactNode, // form fields
  onSubmit: (formData: FormData) => Promise<void>,
  submitLabel?: string,
  isLoading?: boolean,
  error?: string,
}) {
  // Modal dialog with:
  // - Title + description
  // - Submit button (disabled while loading)
  // - Error message display
  // - Cancel button
  
  return <Dialog open={isOpen}>{/* ... */}</Dialog>
}
```

**Used by:** `<EditDialog>`, create/edit forms

### `<ListPage>`

```typescript
export function ListPage<T>({
  data: T[],
  columns: ColumnDef<T>[],
  title: string,
  description?: string,
  onCreateClick: () => void,
  isEmpty?: boolean,
  isLoading?: boolean,
  emptyStateMessage?: string,
}) {
  return (
    <>
      <PageHeader
        title={title}
        description={description}
        action={{ label: 'Create', onClick: onCreateClick }}
      />
      {isEmpty ? (
        <EmptyState message={emptyStateMessage} />
      ) : (
        <DataTable columns={columns} data={data} />
      )}
    </>
  )
}
```

**Typical usage:**

```typescript
export default async function OrganizationsPage() {
  const orgs = await fetchOrganizations()
  
  return (
    <ListPage
      data={orgs}
      columns={[
        { accessorKey: 'name', header: 'Name' },
        { accessorKey: 'created_at', header: 'Created' },
      ]}
      title="Organizations"
      isEmpty={orgs.length === 0}
      onCreateClick={() => {/* open create dialog */}}
    />
  )
}
```

### `<EditDialog>`

```typescript
export function EditDialog({
  isOpen: boolean,
  title: string,
  children: ReactNode, // form fields
  onSubmit: (formData: FormData) => Promise<void>,
  isLoading?: boolean,
  error?: string,
  onClose: () => void,
}) {
  // Wraps FormDialog with:
  // - Loading spinner overlay
  // - Error message display
  // - Success toast notification (on successful submit)
  // - Auto-close on success
  
  return <FormDialog {...props}>{children}</FormDialog>
}
```

## 8. RBAC Scoping Convention

**Every page and API call enforces:**

| Role | Organizations | Teams | Users | API Keys | Spend |
|------|---|---|---|---|---|
| `super_admin` | See all | See all | See all | See all | See all |
| `org_admin` | See own org only | See teams in own org | See users in own org | See keys in own org | See spend for own org |
| `team_admin` | No access | No access | No access | No access | No access |
| `user` | No access | No access | No access | No access | Can see own spend |

**Implementation:**
- Middleware redirects non-admins away from `/admin/*`
- Each page checks `user.role` and rejects if not authorized
- API calls include `?organization_id=user.organization_id` (for org_admin), or omit it (for super_admin)
- Backend enforces same scoping via JWT claims

## 9. Testing Strategy

### Unit & Component Tests (Vitest + React Testing Library)

Test each Tier 1 & Tier 2 component in isolation:

- `<DataTable>`: renders columns, sorting/filtering works, pagination controls appear
- `<FormDialog>`: renders form, submit button works, error message displays
- `<ListPage>`: combines header + table, empty state shows when data is empty
- `<PageHeader>`: title and action button render correctly

**File structure:** Each component has a `.test.tsx` file next to it (co-located)

### Integration Tests (E2E with Playwright)

Test critical user flows end-to-end:

1. **Auth flow (password):**
   - Visit `/login` → enter email/password → redirected to `/admin`
   - Token stored in cookies

2. **Auth flow (SSO):**
   - Visit `/login` → click SSO button → redirected to provider → callback → logged in

3. **Protected routes:**
   - Visit `/admin` without login → redirected to `/login`
   - Login → visit `/admin` → dashboard loads

4. **Dashboard loads:**
   - Login as org_admin → visit `/admin` → stats show org-specific data
   - Login as super_admin → visit `/admin` → stats show system-wide data

5. **Basic CRUD (one resource):**
   - List organizations → click create → fill form → submit → list updates

6. **RBAC enforcement:**
   - Login as org_admin → try to access org outside their org → forbidden

**File location:** `e2e/` directory in the Next.js app

### What NOT to test:
- Every variant of every component (cover happy paths; edge cases in unit tests)
- Specific backend API behavior (that's the backend's job)
- Browser quirks (Playwright handles that)

## 10. Acceptance Criteria

Foundation phase is complete when:

- ✅ Next.js app scaffolded with folder structure as described
- ✅ Login page renders (password + SSO buttons, conditionally)
- ✅ Password auth flow works end-to-end (login → tokens in cookies → redirect → logout)
- ✅ SSO flow wired to backend OIDC endpoint (no actual provider needed yet; test with mock)
- ✅ Protected routes: `/admin/*` requires login, redirects to `/login` if not authenticated
- ✅ Token auto-refresh works (expired token triggers refresh, request retried)
- ✅ Middleware enforces RBAC (super_admin vs org_admin scoping)
- ✅ Dashboard home page loads with scoped stats + spend graph + recent activity
- ✅ Sidebar nav renders with links to all resource sections
- ✅ `<DataTable>`, `<FormDialog>`, `<ListPage>`, `<EditDialog>` components built and tested
- ✅ One resource (e.g., organizations) has a fully functional list + create/edit/delete flows
- ✅ All component unit tests pass
- ✅ Critical E2E flows pass (auth, protected routes, one CRUD resource)
- ✅ TypeScript strict mode enabled; no `any` types
- ✅ README documents how to add a new resource screen (so phase C goes fast)

---

## Notes for Phase C (Per-Resource Screens)

Once the foundation is complete, each resource screen (teams, users, api-keys, models, spend) follows the same pattern:

1. Define `ColumnDef<T>[]` for the table columns
2. Fetch data in a Server Component
3. Render `<ListPage>` or `<EditDialog>`
4. Add create/update/delete Server Actions
5. Add component tests for specific validation logic
6. Done

Expected time per resource: 1-2 hours (mostly configuration, minimal custom logic).

