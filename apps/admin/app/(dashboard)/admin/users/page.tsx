'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { createUser, listUsers } from './actions'

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

const columns: ColumnDef<User>[] = [
  {
    accessorKey: 'email',
    header: 'Email',
  },
  {
    accessorKey: 'role',
    header: 'Role',
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function UsersPage() {
  const router = useRouter()
  const [users, setUsers] = useState<User[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchUsers = async () => {
      try {
        setUsers(await listUsers())
      } catch (err) {
        console.error('Failed to fetch users:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchUsers()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const email = formData.get('email') as string
    const name = formData.get('name') as string
    const role = formData.get('role') as string
    const result = await createUser(email, name, role)

    if (result.success && result.user) {
      setUsers([...users, result.user])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create user')
    }
  }

  return (
    <>
      <ListPage
        data={users}
        columns={columns}
        title="Users"
        isEmpty={users.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
        onRowClick={(user) => router.push(`/admin/users/${user.id}`)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create User"
        onSubmit={handleCreateSubmit}
        onClose={() => setIsCreateDialogOpen(false)}
      >
        <div>
          <label htmlFor="email" className="block text-sm font-medium text-gray-700">
            Email
          </label>
          <input
            id="email"
            name="email"
            type="email"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            id="name"
            name="name"
            type="text"
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
            defaultValue="user"
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
