'use client'

import { useState, useEffect } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { PageHeader } from '@/components/ui/page-header'
import { deleteApiKey, getApiKey } from '../actions'

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

export default function ApiKeyDetailPage() {
  const { id } = useParams() as { id: string }
  const router = useRouter()
  const [apiKey, setApiKey] = useState<ApiKey | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const fetchApiKey = async () => {
      try {
        setApiKey(await getApiKey(id))
      } catch (err) {
        console.error('Failed to fetch API key:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchApiKey()
  }, [id])

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this API key?')) return

    const result = await deleteApiKey(id)
    if (result.success) {
      router.push('/admin/api-keys')
    } else {
      alert(result.error || 'Failed to delete API key')
    }
  }

  if (isLoading) return <div>Loading...</div>
  if (!apiKey) return <div>API key not found</div>

  return (
    <div className="space-y-6">
      <PageHeader title={apiKey.name} />

      <div className="rounded-lg bg-white p-6 shadow">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className="text-sm text-gray-600">Key Prefix</p>
            <p className="text-lg font-semibold text-gray-900">{apiKey.key_prefix}</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Organization ID</p>
            <p className="text-lg font-semibold text-gray-900">{apiKey.organization_id}</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Scopes</p>
            <p className="text-lg font-semibold text-gray-900">
              {(apiKey.scopes || []).join(', ')}
            </p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Budget Spent (USD)</p>
            <p className="text-lg font-semibold text-gray-900">{apiKey.budget_spent_usd}</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Disabled</p>
            <p className="text-lg font-semibold text-gray-900">
              {apiKey.disabled ? 'Yes' : 'No'}
            </p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Created</p>
            <p className="text-lg font-semibold text-gray-900">
              {new Date(apiKey.created_at).toLocaleDateString()}
            </p>
          </div>
        </div>

        <button
          onClick={handleDelete}
          className="mt-6 rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
        >
          Delete API Key
        </button>
      </div>
    </div>
  )
}
