'use server'

import { redirect } from 'next/navigation'
import { setTokens } from '@/lib/auth'

// Server-side fetches use the internal Docker network URL when set (API_URL);
// the OIDC redirect below sends the browser to the publicly reachable URL instead.
const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'
const PUBLIC_API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

// OIDC provider id must match an entry in the backend's `auth.oidc_providers` config
// (backend route: GET /api/v1/auth/oidc/{provider}). Defaults to "google".
const OIDC_PROVIDER_ID = process.env.OIDC_PROVIDER_ID || 'google'

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
  // The backend builds the authorize URL for this provider: GET /api/v1/auth/oidc/{provider}.
  // The IdP's redirect_uri (configured in the provider entry) must point at this admin
  // `/auth/callback` page so the browser lands here with `code`/`state`, which
  // exchangeOIDCCode then sends back to the backend callback (GET .../oidc/{provider}/callback)
  // to receive the token pair (HttpOnly cookies + JSON body).
  redirect(`${PUBLIC_API_URL}/api/v1/auth/oidc/${OIDC_PROVIDER_ID}`)
}
