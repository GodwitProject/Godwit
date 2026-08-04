'use client'

import { useState, useEffect } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { PageHeader } from '@/components/ui/page-header'
import { FormDialog } from '@/components/ui/form-dialog'
import { updateUser, deleteUser, getUser } from '../actions'

interface User {
  id: string
  organization_id: string | null
  email: string
  name: string | null
  role: string
  sso_provider: string | null
  sso_subject: string | null
  created_at: string
}

export default function UserDetailPage() {
  const { id } = useParams() as { id: string }
  const router = useRouter()
  const [user, setUser] = useState<User | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)

  useEffect(() => {
    const fetchUser = async () => {
      try {
        setUser(await getUser(id))
      } catch (err) {
        console.error('Failed to fetch user:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchUser()
  }, [id])

  const handleEditSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const role = formData.get('role') as string
    const result = await updateUser(id, name, role)

    if (result.success && result.user) {
      setUser(result.user)
      setIsEditDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to update user')
    }
  }

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this user?')) return

    const result = await deleteUser(id)
    if (result.success) {
      router.push('/admin/users')
    } else {
      alert(result.error || 'Failed to delete user')
    }
  }

  if (isLoading) return <div>Loading...</div>
  if (!user) return <div>User not found</div>

  return (
    <>
      <div className="space-y-6">
        <PageHeader
          title={user.email}
          action={{ label: 'Edit', onClick: () => setIsEditDialogOpen(true) }}
        />

        <div className="rounded-lg bg-white p-6 shadow">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-gray-600">Role</p>
              <p className="text-lg font-semibold text-gray-900">{user.role}</p>
            </div>
            <div>
              <p className="text-sm text-gray-600">Created</p>
              <p className="text-lg font-semibold text-gray-900">
                {new Date(user.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          <button
            onClick={handleDelete}
            className="mt-6 rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
          >
            Delete User
          </button>
        </div>
      </div>

      <FormDialog
        isOpen={isEditDialogOpen}
        title="Edit User"
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
            defaultValue={user.name || ''}
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
        <div>
          <label htmlFor="role" className="block text-sm font-medium text-gray-700">
            Role
          </label>
          <select
            id="role"
            name="role"
            defaultValue={user.role}
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          >
            <option value="user">user</option>
            <option value="team_admin">team_admin</option>
            <option value="org_admin">org_admin</option>
            <option value="super_admin">super_admin</option>
          </select>
        </div>
      </FormDialog>
    </>
  )
}
