'use server'

import { setTokens } from '@/lib/auth'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function exchangeOIDCCode(code: string, state: string) {
  try {
    const response = await fetch(`${API_URL}/api/v1/auth/oidc/callback`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code, state }),
    })

    if (!response.ok) {
      throw new Error('Token exchange failed')
    }

    const { access_token, refresh_token } = await response.json()
    await setTokens(access_token, refresh_token)

    return { success: true }
  } catch (err) {
    console.error('OIDC callback error:', err)
    return { success: false, error: 'Token exchange failed' }
  }
}
