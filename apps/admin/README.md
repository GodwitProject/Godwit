# Godwit Admin Dashboard

## Getting Started

```bash
cd apps/admin
npm install
npm run dev
```

Navigate to http://localhost:3000 and log in.

## Architecture

Three-tier component system:

1. **Tier 1 — Primitives** (`components/ui/`)
   - `<DataTable>` — generic table with sorting, filtering, pagination
   - `<FormDialog>` — modal form container
   - `<PageHeader>` — page title + action button
   - `<EmptyState>` — "no data" placeholder

2. **Tier 2 — Smart Components** (`components/admin/`)
   - `<ListPage>` — list layout (header + table + empty state)
   - `<EditDialog>` — edit modal with form handling
   - `<ResourceForm>` — base form for common fields

3. **Tier 3 — Resource Pages** (`app/(dashboard)/admin/[resource]/`)
   - Fetch data server-side
   - Define columns/fields
   - Render smart components

## Data Flow

1. **Page (Server Component)** → fetches data via `apiCall()`
2. **Components render** → handle interactions
3. **Form submit** → Server Action → API call → revalidate data

## Adding a New Resource

Example: adding a "Billing Plans" resource.

### Step 1: Create folder structure

```bash
mkdir -p app/(dashboard)/admin/billing-plans/[id]
touch app/(dashboard)/admin/billing-plans/page.tsx
touch app/(dashboard)/admin/billing-plans/[id]/page.tsx
touch app/(dashboard)/admin/billing-plans/actions.ts
```

### Step 2: Define your types

File: `app/(dashboard)/admin/billing-plans/types.ts`

```typescript
export interface BillingPlan {
  id: string
  name: string
  price_usd: number
  created_at: string
}
```

### Step 3: Create Server Actions

File: `app/(dashboard)/admin/billing-plans/actions.ts`

```typescript
'use server'

import { apiCall } from '@/lib/api-client'
import { BillingPlan } from './types'

export async function createBillingPlan(
  name: string,
  price_usd: number
): Promise<{ success: boolean; plan?: BillingPlan; error?: string }> {
  try {
    const response = await apiCall('/api/v1/billing-plans', {
      method: 'POST',
      body: JSON.stringify({ name, price_usd }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create billing plan' }
    }

    const data = await response.json()
    return { success: true, plan: data.data }
  } catch (err) {
    return { success: false, error: 'An error occurred' }
  }
}

// Similarly: updateBillingPlan, deleteBillingPlan
```

### Step 4: Create list page

File: `app/(dashboard)/admin/billing-plans/page.tsx`

```typescript
'use client'

import { useState, useEffect } from 'react'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { apiCall } from '@/lib/api-client'
import { BillingPlan } from './types'
import { createBillingPlan } from './actions'

const columns: ColumnDef<BillingPlan>[] = [
  { accessorKey: 'name', header: 'Name' },
  { accessorKey: 'price_usd', header: 'Price (USD)' },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function BillingPlansPage() {
  const [plans, setPlans] = useState<BillingPlan[]>([])
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchPlans = async () => {
      try {
        const response = await apiCall('/api/v1/billing-plans')
        if (response.ok) {
          const data = await response.json()
          setPlans(data.data || [])
        }
      } catch (err) {
        console.error('Failed to fetch billing plans:', err)
      }
    }

    fetchPlans()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const price_usd = parseFloat(formData.get('price_usd') as string)

    const result = await createBillingPlan(name, price_usd)
    if (result.success && result.plan) {
      setPlans([...plans, result.plan])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create billing plan')
    }
  }

  return (
    <>
      <ListPage
        data={plans}
        columns={columns}
        title="Billing Plans"
        isEmpty={plans.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create Billing Plan"
        onSubmit={handleCreateSubmit}
        onClose={() => setIsCreateDialogOpen(false)}
      >
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            id="name"
            name="name"
            type="text"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>

        <div>
          <label htmlFor="price_usd" className="block text-sm font-medium text-gray-700">
            Price (USD)
          </label>
          <input
            id="price_usd"
            name="price_usd"
            type="number"
            step="0.01"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
```

### Step 5: Create detail page

Similar pattern to `app/(dashboard)/admin/organizations/[id]/page.tsx` — fetch by ID, render PageHeader with edit/delete actions, FormDialog for edit.

### Step 6: Add to sidebar

Update `components/layout/sidebar.tsx` to include the new resource in the `navigation` array.

### Step 7: Test

1. Unit tests: test the Server Actions in isolation
2. E2E tests: test the full CRUD flow in a browser
3. Manual test: navigate to the page, create/edit/delete an item

## Testing

### Unit Tests

```bash
npm test
```

### E2E Tests

```bash
npm run test:e2e
```

## API Integration

All API calls go through `lib/api-client.ts`, which:
- Reads the access token from httpOnly cookies (server-side only)
- Automatically refreshes tokens on 401
- Adds the `Authorization: Bearer <token>` header

To call an API:

```typescript
import { apiCall } from '@/lib/api-client'

const response = await apiCall('/api/v1/resource')
const data = await response.json()
```

## RBAC Scoping

Every page and API call respects the user's role:
- **super_admin**: unfiltered access
- **org_admin**: scoped to own organization
- **team_admin / user**: no dashboard access

Scoping is applied:
1. At the page level (middleware + `app/(dashboard)/layout.tsx`)
2. At the API level (query params like `?organization_id=...`)

## Environment Variables

- `NEXT_PUBLIC_API_URL`: Backend API base URL (default: `https://api.godwit.io`)
- `NEXT_PUBLIC_APP_URL`: Frontend app URL (for OIDC redirects, default: `http://localhost:3000`)

See `.env.local` for development values.
