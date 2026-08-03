'use client'

import { useEffect } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { exchangeOIDCCode } from './actions'

export default function AuthCallbackPage() {
  const router = useRouter()
  const searchParams = useSearchParams()

  useEffect(() => {
    const handleCallback = async () => {
      const code = searchParams.get('code')
      const state = searchParams.get('state')
      const error = searchParams.get('error')

      if (error) {
        console.error('OIDC error:', error)
        router.push('/login?error=oidc_failed')
        return
      }

      if (!code || !state) {
        router.push('/login?error=no_code')
        return
      }

      try {
        const result = await exchangeOIDCCode(code, state)

        if (!result.success) {
          throw new Error(result.error || 'Token exchange failed')
        }

        router.push('/admin')
      } catch (err) {
        console.error('Callback error:', err)
        router.push('/login?error=callback_failed')
      }
    }

    handleCallback()
  }, [searchParams, router])

  return (
    <div className="flex h-screen items-center justify-center">
      <p className="text-gray-600">Completing sign-in...</p>
    </div>
  )
}
