'use server'

const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function performPasswordReset(
  token: string,
  newPassword: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await fetch(`${API_URL}/api/v1/auth/reset-password`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token, new_password: newPassword }),
    })

    if (!response.ok) {
      return { success: false, error: 'Unable to reset your password' }
    }

    return { success: true }
  } catch (err) {
    console.error('Password reset error:', err)
    return { success: false, error: 'Password reset failed' }
  }
}
