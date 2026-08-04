'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { createApiKey, listApiKeys } from './actions'

interface ApiKey {
  id: string
  user_id: string
  team_id: string | null
  organization_id: string
  name: string
  key_prefix: string
  scopes: string[]
  budget_limit_usd: string | null
  budget_spent_usd: string
  rate_limit_requests_per_minute: number | null
  expires_at: string | null
  disabled: boolean
  created_at: string
}

const columns: ColumnDef<ApiKey>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
  },
  {
    accessorKey: 'key_prefix',
    header: 'Key Prefix',
  },
  {
    id: 'scopes',
    header: 'Scopes',
    cell: (info) => (info.row.original.scopes || []).join(', '),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function ApiKeysPage() {
  const router = useRouter()
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  const fetchApiKeys = async () => {
    try {
      setApiKeys(await listApiKeys())
    } catch (err) {
      console.error('Failed to fetch API keys:', err)
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    fetchApiKeys()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const scopes = formData.get('scopes') as string
    const result = await createApiKey(name, scopes)

    if (result.success && result.apiKey) {
      setIsCreateDialogOpen(false)
      alert(`API key created. Copy it now — it will not be shown again:\n\n${result.apiKey}`)
      await fetchApiKeys()
    } else {
      throw new Error(result.error || 'Failed to create API key')
    }
  }

  return (
    <>
      <ListPage
        data={apiKeys}
        columns={columns}
        title="API Keys"
        isEmpty={apiKeys.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
        onRowClick={(key) => router.push(`/admin/api-keys/${key.id}`)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create API Key"
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
          <label htmlFor="scopes" className="block text-sm font-medium text-gray-700">
            Scopes
          </label>
          <input
            id="scopes"
            name="scopes"
            type="text"
            placeholder="proxy:write,proxy:read"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
