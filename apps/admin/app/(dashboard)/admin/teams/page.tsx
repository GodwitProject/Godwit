'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { createTeam, listTeams } from './actions'

interface Team {
  id: string
  organization_id: string
  name: string
  created_at: string
}

const columns: ColumnDef<Team>[] = [
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

export default function TeamsPage() {
  const router = useRouter()
  const [teams, setTeams] = useState<Team[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchTeams = async () => {
      try {
        setTeams(await listTeams())
      } catch (err) {
        console.error('Failed to fetch teams:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchTeams()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const organizationId = formData.get('organization_id') as string
    const result = await createTeam(name, organizationId)

    if (result.success && result.team) {
      setTeams([...teams, result.team])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create team')
    }
  }

  return (
    <>
      <ListPage
        data={teams}
        columns={columns}
        title="Teams"
        isEmpty={teams.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
        onRowClick={(team) => router.push(`/admin/teams/${team.id}`)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create Team"
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
          <label htmlFor="organization_id" className="block text-sm font-medium text-gray-700">
            Organization ID
          </label>
          <input
            id="organization_id"
            name="organization_id"
            type="text"
            placeholder="uuid"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
