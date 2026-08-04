'use server'

import { redirect } from 'next/navigation'
import { clearTokens } from './auth'

const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function logoutAction() {
  const token = await import('./auth').then((m) => m.getAccessToken())

  if (token) {
    try {
      await fetch(`${API_URL}/api/v1/auth/logout`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
      })
    } catch (err) {
      console.error('Logout API call failed:', err)
    }
  }

  await clearTokens()
  redirect('/login')
}
