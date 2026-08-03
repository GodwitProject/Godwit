'use server'

import { setTokens } from '@/lib/auth'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function exchangeOIDCCode(code: string, state: string): Promise<void> {
  const response = await fetch(`${API_URL}/api/v1/auth/oidc/callback`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, state }),
  })

  if (!response.ok) {
    throw new Error('Token exchange failed')
  }

  const data = await response.json() as { access_token: string; refresh_token: string }
  await setTokens(data.access_token, data.refresh_token)
}
