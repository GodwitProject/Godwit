'use client'

import { useState, useEffect } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { PageHeader } from '@/components/ui/page-header'
import { FormDialog } from '@/components/ui/form-dialog'
import { updateTeam, deleteTeam, getTeam } from '../actions'

interface Team {
  id: string
  organization_id: string
  name: string
  created_at: string
}

export default function TeamDetailPage() {
  const { id } = useParams() as { id: string }
  const router = useRouter()
  const [team, setTeam] = useState<Team | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)

  useEffect(() => {
    const fetchTeam = async () => {
      try {
        setTeam(await getTeam(id))
      } catch (err) {
        console.error('Failed to fetch team:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchTeam()
  }, [id])

  const handleEditSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const result = await updateTeam(id, name)

    if (result.success && result.team) {
      setTeam(result.team)
      setIsEditDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to update team')
    }
  }

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this team?')) return

    const result = await deleteTeam(id)
    if (result.success) {
      router.push('/admin/teams')
    } else {
      alert(result.error || 'Failed to delete team')
    }
  }

  if (isLoading) return <div>Loading...</div>
  if (!team) return <div>Team not found</div>

  return (
    <>
      <div className="space-y-6">
        <PageHeader
          title={team.name}
          action={{ label: 'Edit', onClick: () => setIsEditDialogOpen(true) }}
        />

        <div className="rounded-lg bg-white p-6 shadow">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-gray-600">Organization ID</p>
              <p className="text-lg font-semibold text-gray-900">{team.organization_id}</p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Created</p>
              <p className="text-lg font-semibold text-gray-900">
                {new Date(team.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          <button
            onClick={handleDelete}
            className="mt-6 rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
          >
            Delete Team
          </button>
        </div>
      </div>

      <FormDialog
        isOpen={isEditDialogOpen}
        title="Edit Team"
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
            defaultValue={team.name}
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
