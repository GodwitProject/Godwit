'use client'

import { useState, useEffect } from 'react'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { apiCall } from '@/lib/api-client'
import { createOrganization } from './actions'

interface Organization {
  id: string
  name: string
  created_at: string
}

const columns: ColumnDef<Organization>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function OrganizationsPage() {
  const [organizations, setOrganizations] = useState<Organization[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchOrganizations = async () => {
      try {
        const response = await apiCall('/api/v1/organizations')
        if (response.ok) {
          const data = await response.json()
          setOrganizations(data.data || [])
        }
      } catch (err) {
        console.error('Failed to fetch organizations:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchOrganizations()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const result = await createOrganization(name)

    if (result.success && result.organization) {
      setOrganizations([...organizations, result.organization])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create organization')
    }
  }

  return (
    <>
      <ListPage
        data={organizations}
        columns={columns}
        title="Organizations"
        isEmpty={organizations.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create Organization"
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
      </FormDialog>
    </>
  )
}
