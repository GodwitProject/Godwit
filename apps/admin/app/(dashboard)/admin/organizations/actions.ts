'use server'

import { apiCall } from '@/lib/api-client'

interface Organization {
  id: string
  name: string
  created_at: string
}

export async function listOrganizations(): Promise<Organization[]> {
  const response = await apiCall('/api/v1/organizations')
  if (!response.ok) return []
  const data = await response.json()
  return data.data || []
}

export async function getOrganization(id: string): Promise<Organization | null> {
  const response = await apiCall(`/api/v1/organizations/${id}`)
  if (!response.ok) return null
  const data = await response.json()
  return data.data
}

export async function createOrganization(
  name: string
): Promise<{ success: boolean; organization?: Organization; error?: string }> {
  try {
    const response = await apiCall('/api/v1/organizations', {
      method: 'POST',
      body: JSON.stringify({ name }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create organization' }
    }

    const data = await response.json()
    return { success: true, organization: data.data }
  } catch (err) {
    console.error('Create organization error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function updateOrganization(
  id: string,
  name: string
): Promise<{ success: boolean; organization?: Organization; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/organizations/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ name }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to update organization' }
    }

    const data = await response.json()
    return { success: true, organization: data.data }
  } catch (err) {
    console.error('Update organization error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function deleteOrganization(
  id: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/organizations/${id}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to delete organization' }
    }

    return { success: true }
  } catch (err) {
    console.error('Delete organization error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
