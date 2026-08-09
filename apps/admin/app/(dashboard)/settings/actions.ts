'use server'

import { apiCall } from '@/lib/api-client'

export async function changePassword(
  currentPassword: string,
  newPassword: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall('/api/v1/auth/change-password', {
      method: 'POST',
      body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
    })

    if (!response.ok) {
      try {
        const data = await response.json()
        return { success: false, error: data.detail || 'Failed to change password' }
      } catch {
        return { success: false, error: 'Failed to change password' }
      }
    }

    return { success: true }
  } catch (err) {
    console.error('Change password error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
