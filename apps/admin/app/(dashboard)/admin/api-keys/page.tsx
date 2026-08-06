'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { ApiKey, Model } from '@/lib/types'
import { createApiKey, listApiKeys, listModels } from './actions'

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
    id: 'allowed_models',
    header: 'Allowed Models',
    cell: (info) => {
      const models = info.row.original.allowed_models || []
      return models.length === 0 ? 'All models' : models.join(', ')
    },
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
  const [models, setModels] = useState<Model[]>([])
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
    const fetchModels = async () => {
      try {
        setModels(await listModels())
      } catch (err) {
        console.error('Failed to fetch models:', err)
      }
    }
    fetchModels()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const scopes = formData.get('scopes') as string
    const allowedModels = formData.getAll('allowed_models') as string[]
    const result = await createApiKey(name, scopes, allowedModels)

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
        <div>
          <label htmlFor="allowed_models" className="block text-sm font-medium text-gray-700">
            Allowed Models
          </label>
          <select
            id="allowed_models"
            name="allowed_models"
            multiple
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
            size={Math.min(6, Math.max(3, models.length))}
          >
            {models.map((model) => (
              <option key={model.id} value={model.public_id}>
                {model.public_id}
              </option>
            ))}
          </select>
          <p className="mt-1 text-xs text-gray-500">Hold Ctrl/Cmd to select multiple models. Leave empty to allow all models.</p>
        </div>
      </FormDialog>
    </>
  )
}
