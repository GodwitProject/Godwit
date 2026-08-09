'use server'

const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function requestPasswordReset(
  email: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await fetch(`${API_URL}/api/v1/auth/forgot-password`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email }),
    })

    if (!response.ok) {
      return { success: false, error: 'Unable to process your request' }
    }

    return { success: true }
  } catch (err) {
    console.error('Password reset request error:', err)
    return { success: false, error: 'Password reset request failed' }
  }
}
