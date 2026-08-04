'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { createModel, listModels } from './actions'

interface Model {
  id: string
  public_id: string
  provider: string
  provider_profile_id: string
  provider_model_id: string
  capabilities: string[]
  pricing: Record<string, unknown>
  config: Record<string, unknown>
  created_at: string
}

const columns: ColumnDef<Model>[] = [
  {
    accessorKey: 'public_id',
    header: 'Public ID',
  },
  {
    accessorKey: 'provider',
    header: 'Provider',
  },
  {
    accessorKey: 'provider_model_id',
    header: 'Provider Model ID',
  },
  {
    id: 'capabilities',
    header: 'Capabilities',
    cell: (info) => (info.row.original.capabilities || []).join(', '),
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function ModelsPage() {
  const router = useRouter()
  const [models, setModels] = useState<Model[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchModels = async () => {
      try {
        setModels(await listModels())
      } catch (err) {
        console.error('Failed to fetch models:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchModels()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const publicId = formData.get('public_id') as string
    const provider = formData.get('provider') as string
    const providerProfileId = formData.get('provider_profile_id') as string
    const providerModelId = formData.get('provider_model_id') as string
    const capabilities = formData.get('capabilities') as string

    const result = await createModel(
      publicId,
      provider,
      providerProfileId,
      providerModelId,
      capabilities
    )

    if (result.success && result.model) {
      setModels([...models, result.model])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create model')
    }
  }

  return (
    <>
      <ListPage
        data={models}
        columns={columns}
        title="Models"
        isEmpty={models.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
        onRowClick={(model) => router.push(`/admin/models/${model.id}`)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create Model"
        onSubmit={handleCreateSubmit}
        onClose={() => setIsCreateDialogOpen(false)}
      >
        <div>
          <label htmlFor="public_id" className="block text-sm font-medium text-gray-700">
            Public ID
          </label>
          <input
            id="public_id"
            name="public_id"
            type="text"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
        <div>
          <label htmlFor="provider" className="block text-sm font-medium text-gray-700">
            Provider
          </label>
          <input
            id="provider"
            name="provider"
            type="text"
            placeholder="openai"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
        <div>
          <label htmlFor="provider_profile_id" className="block text-sm font-medium text-gray-700">
            Provider Profile ID
          </label>
          <input
            id="provider_profile_id"
            name="provider_profile_id"
            type="text"
            placeholder="uuid"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
        <div>
          <label htmlFor="provider_model_id" className="block text-sm font-medium text-gray-700">
            Provider Model ID
          </label>
          <input
            id="provider_model_id"
            name="provider_model_id"
            type="text"
            placeholder="gpt-4o"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
        <div>
          <label htmlFor="capabilities" className="block text-sm font-medium text-gray-700">
            Capabilities
          </label>
          <input
            id="capabilities"
            name="capabilities"
            type="text"
            placeholder="chat,embedding"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
