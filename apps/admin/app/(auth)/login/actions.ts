'use server'

import { redirect } from 'next/navigation'
import { setTokens } from '@/lib/auth'

// Server-side fetches use the internal Docker network URL when set (API_URL);
// the OIDC redirect below sends the browser to the publicly reachable URL instead.
const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'
const PUBLIC_API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function loginWithPassword(
  email: string,
  password: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await fetch(`${API_URL}/api/v1/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password }),
    })

    if (!response.ok) {
      return { success: false, error: 'Invalid email or password' }
    }

    const { access_token, refresh_token } = await response.json()
    await setTokens(access_token, refresh_token)

    return { success: true }
  } catch (err) {
    console.error('Login error:', err)
    return { success: false, error: 'Login failed' }
  }
}

export async function loginWithSSO() {
  // Redirect to OIDC authorize endpoint
  // This will be handled by the backend's OIDC endpoint
  redirect(`${PUBLIC_API_URL}/api/v1/auth/oidc/authorize?redirect_uri=${process.env.NEXT_PUBLIC_APP_URL}/auth/callback`)
}
