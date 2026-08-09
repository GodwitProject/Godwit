'use server'

import { setTokens } from '@/lib/auth'

const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

// Must match the provider id used by loginWithSSO's redirect (backend config
// `auth.oidc_providers`). The IdP's redirect_uri must point at this admin
// `/auth/callback` page so `code`/`state` land here for the browser.
const OIDC_PROVIDER_ID = process.env.OIDC_PROVIDER_ID || 'google'

export async function exchangeOIDCCode(code: string, state: string): Promise<void> {
  const response = await fetch(
    `${API_URL}/api/v1/auth/oidc/${OIDC_PROVIDER_ID}/callback?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`,
    {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    }
  )

  if (!response.ok) {
    throw new Error('Token exchange failed')
  }

  const data = await response.json() as { access_token: string; refresh_token: string }
  await setTokens(data.access_token, data.refresh_token)
}
