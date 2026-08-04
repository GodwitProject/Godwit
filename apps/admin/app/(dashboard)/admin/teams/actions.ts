'use server'

import { apiCall } from '@/lib/api-client'

interface Team {
  id: string
  organization_id: string
  name: string
  created_at: string
}

export async function listTeams(): Promise<Team[]> {
  const response = await apiCall('/api/v1/teams')
  if (!response.ok) return []
  const data = await response.json()
  return data.data || []
}

export async function getTeam(id: string): Promise<Team | null> {
  const response = await apiCall(`/api/v1/teams/${id}`)
  if (!response.ok) return null
  const data = await response.json()
  return data.data
}

export async function createTeam(
  name: string,
  organizationId: string
): Promise<{ success: boolean; team?: Team; error?: string }> {
  try {
    const response = await apiCall('/api/v1/teams', {
      method: 'POST',
      body: JSON.stringify({ name, organization_id: organizationId }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create team' }
    }

    const data = await response.json()
    return { success: true, team: data.data }
  } catch (err) {
    console.error('Create team error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function updateTeam(
  id: string,
  name: string
): Promise<{ success: boolean; team?: Team; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/teams/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ name }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to update team' }
    }

    const data = await response.json()
    return { success: true, team: data.data }
  } catch (err) {
    console.error('Update team error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function deleteTeam(
  id: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/teams/${id}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to delete team' }
    }

    return { success: true }
  } catch (err) {
    console.error('Delete team error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
