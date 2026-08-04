'use server'

import { apiCall } from '@/lib/api-client'

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

async function extractErrorMessage(response: Response, fallback: string): Promise<string> {
  try {
    const data = await response.json()
    return data.detail || fallback
  } catch {
    return fallback
  }
}

export async function listUsers(): Promise<User[]> {
  const response = await apiCall('/api/v1/users')
  if (!response.ok) return []
  const data = await response.json()
  return data.data || []
}

export async function getUser(id: string): Promise<User | null> {
  const response = await apiCall(`/api/v1/users/${id}`)
  if (!response.ok) return null
  const data = await response.json()
  return data.data
}

export async function createUser(
  email: string,
  name: string,
  role: string
): Promise<{ success: boolean; user?: User; error?: string }> {
  try {
    const response = await apiCall('/api/v1/users', {
      method: 'POST',
      body: JSON.stringify({ email, name: name || undefined, role }),
    })

    if (!response.ok) {
      return { success: false, error: await extractErrorMessage(response, 'Failed to create user') }
    }

    const data = await response.json()
    return { success: true, user: data.data }
  } catch (err) {
    console.error('Create user error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function updateUser(
  id: string,
  name: string,
  role: string
): Promise<{ success: boolean; user?: User; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/users/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ name, role }),
    })

    if (!response.ok) {
      return { success: false, error: await extractErrorMessage(response, 'Failed to update user') }
    }

    const data = await response.json()
    return { success: true, user: data.data }
  } catch (err) {
    console.error('Update user error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function deleteUser(
  id: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/users/${id}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      return { success: false, error: await extractErrorMessage(response, 'Failed to delete user') }
    }

    return { success: true }
  } catch (err) {
    console.error('Delete user error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
