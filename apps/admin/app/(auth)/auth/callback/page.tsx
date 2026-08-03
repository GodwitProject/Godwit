'use client'

import { useEffect } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { setTokens } from '@/lib/auth'

export default function AuthCallbackPage() {
  const router = useRouter()
  const searchParams = useSearchParams()

  useEffect(() => {
    const exchangeCode = async () => {
      const code = searchParams.get('code')
      const state = searchParams.get('state')
      const error = searchParams.get('error')

      if (error) {
        console.error('OIDC error:', error)
        router.push('/login?error=oidc_failed')
        return
      }

      if (!code) {
        router.push('/login?error=no_code')
        return
      }

      try {
        const response = await fetch('/api/auth/oidc-callback', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ code, state }),
        })

        if (!response.ok) {
          throw new Error('Token exchange failed')
        }

        const { access_token, refresh_token } = await response.json()
        await setTokens(access_token, refresh_token)

        router.push('/admin')
      } catch (err) {
        console.error('Callback error:', err)
        router.push('/login?error=callback_failed')
      }
    }

    exchangeCode()
  }, [searchParams, router])

  return (
    <div className="flex h-screen items-center justify-center">
      <p className="text-gray-600">Completing sign-in...</p>
    </div>
  )
}
