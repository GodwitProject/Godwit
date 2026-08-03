'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import { PageHeader } from '@/components/ui/page-header'
import { FormDialog } from '@/components/ui/form-dialog'
import { apiCall } from '@/lib/api-client'
import { updateOrganization, deleteOrganization } from '../actions'

interface Organization {
  id: string
  name: string
  created_at: string
}

export default function OrganizationDetailPage() {
  const { id } = useParams() as { id: string }
  const [organization, setOrganization] = useState<Organization | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)

  useEffect(() => {
    const fetchOrganization = async () => {
      try {
        const response = await apiCall(`/api/v1/organizations/${id}`)
        if (response.ok) {
          const data = await response.json()
          setOrganization(data.data)
        }
      } catch (err) {
        console.error('Failed to fetch organization:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchOrganization()
  }, [id])

  const handleEditSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const result = await updateOrganization(id, name)

    if (result.success && result.organization) {
      setOrganization(result.organization)
      setIsEditDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to update organization')
    }
  }

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this organization?')) return

    const result = await deleteOrganization(id)
    if (result.success) {
      window.location.href = '/admin/organizations'
    } else {
      alert(result.error || 'Failed to delete organization')
    }
  }

  if (isLoading) return <div>Loading...</div>
  if (!organization) return <div>Organization not found</div>

  return (
    <>
      <div className="space-y-6">
        <PageHeader
          title={organization.name}
          action={{ label: 'Edit', onClick: () => setIsEditDialogOpen(true) }}
        />

        <div className="rounded-lg bg-white p-6 shadow">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-gray-600">Created</p>
              <p className="text-lg font-semibold text-gray-900">
                {new Date(organization.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          <button
            onClick={handleDelete}
            className="mt-6 rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
          >
            Delete Organization
          </button>
        </div>
      </div>

      <FormDialog
        isOpen={isEditDialogOpen}
        title="Edit Organization"
        onSubmit={handleEditSubmit}
        onClose={() => setIsEditDialogOpen(false)}
      >
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            id="name"
            name="name"
            type="text"
            defaultValue={organization.name}
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
