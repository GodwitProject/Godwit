'use client'

import { useState, useEffect } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { PageHeader } from '@/components/ui/page-header'
import { FormDialog } from '@/components/ui/form-dialog'
import { updateModel, deleteModel, getModel } from '../actions'

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

export default function ModelDetailPage() {
  const { id } = useParams() as { id: string }
  const router = useRouter()
  const [model, setModel] = useState<Model | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)

  useEffect(() => {
    const fetchModel = async () => {
      try {
        setModel(await getModel(id))
      } catch (err) {
        console.error('Failed to fetch model:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchModel()
  }, [id])

  const handleEditSubmit = async (formData: FormData) => {
    const publicId = formData.get('public_id') as string
    const capabilities = formData.get('capabilities') as string
    const result = await updateModel(id, publicId, capabilities)

    if (result.success && result.model) {
      setModel(result.model)
      setIsEditDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to update model')
    }
  }

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this model?')) return

    const result = await deleteModel(id)
    if (result.success) {
      router.push('/admin/models')
    } else {
      alert(result.error || 'Failed to delete model')
    }
  }

  if (isLoading) return <div>Loading...</div>
  if (!model) return <div>Model not found</div>

  return (
    <>
      <div className="space-y-6">
        <PageHeader
          title={model.public_id}
          action={{ label: 'Edit', onClick: () => setIsEditDialogOpen(true) }}
        />

        <div className="rounded-lg bg-white p-6 shadow">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-gray-600">Provider</p>
              <p className="text-lg font-semibold text-gray-900">{model.provider}</p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Provider Model ID</p>
              <p className="text-lg font-semibold text-gray-900">{model.provider_model_id}</p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Provider Profile ID</p>
              <p className="text-lg font-semibold text-gray-900">{model.provider_profile_id}</p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Capabilities</p>
              <p className="text-lg font-semibold text-gray-900">
                {(model.capabilities || []).join(', ')}
              </p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Created</p>
              <p className="text-lg font-semibold text-gray-900">
                {new Date(model.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          <button
            onClick={handleDelete}
            className="mt-6 rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
          >
            Delete Model
          </button>
        </div>
      </div>

      <FormDialog
        isOpen={isEditDialogOpen}
        title="Edit Model"
        onSubmit={handleEditSubmit}
        onClose={() => setIsEditDialogOpen(false)}
      >
        <div>
          <label htmlFor="public_id" className="block text-sm font-medium text-gray-700">
            Public ID
          </label>
          <input
            id="public_id"
            name="public_id"
            type="text"
            defaultValue={model.public_id}
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
            defaultValue={(model.capabilities || []).join(',')}
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
