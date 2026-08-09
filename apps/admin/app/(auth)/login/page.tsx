'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { loginWithPassword, loginWithSSO } from './actions'

export default function LoginPage() {
  const router = useRouter()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const [config, setConfig] = useState({ passwordEnabled: true, ssoEnabled: true })

  // Fetch login config on mount (from /api/v1/auth/config)
  // For now, assume both are enabled

  const handlePasswordLogin = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')

    try {
      const result = await loginWithPassword(email, password)
      if (result.success) {
        router.push('/admin')
      } else {
        setError(result.error || 'Login failed')
      }
    } catch (err) {
      setError('An unexpected error occurred')
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  const handleSSO = async () => {
    setLoading(true)
    try {
      const result = await loginWithSSO()
      // Redirected by the server action
    } catch (err) {
      setError('SSO login failed')
      console.error(err)
      setLoading(false)
    }
  }

  return (
    <div className="w-full max-w-md space-y-8 rounded-lg bg-white p-8 shadow-lg">
      <div>
        <h1 className="text-center text-3xl font-bold">Godwit Admin</h1>
        <p className="mt-2 text-center text-sm text-gray-600">Sign in to your account</p>
      </div>

      {error && <div className="rounded bg-red-100 p-4 text-red-700">{error}</div>}

      {config.passwordEnabled && (
        <form onSubmit={handlePasswordLogin} className="space-y-6">
          <div>
            <label htmlFor="email" className="block text-sm font-medium text-gray-700">
              Email
            </label>
            <input
              id="email"
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              disabled={loading}
            />
          </div>

          <div>
            <label htmlFor="password" className="block text-sm font-medium text-gray-700">
              Password
            </label>
            <input
              id="password"
              type="password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              disabled={loading}
            />
          </div>

          <button
            type="submit"
            disabled={loading}
            className="w-full rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Signing in...' : 'Sign in with password'}
          </button>
        </form>
      )}

      {config.passwordEnabled && (
        <div className="text-center">
          <Link href="/forgot-password" className="text-sm text-blue-600 hover:underline">
            Forgot password?
          </Link>
        </div>
      )}

      {config.passwordEnabled && config.ssoEnabled && (
        <div className="relative">
          <div className="absolute inset-0 flex items-center">
            <div className="w-full border-t border-gray-300"></div>
          </div>
          <div className="relative flex justify-center text-sm">
            <span className="bg-white px-2 text-gray-500">Or</span>
          </div>
        </div>
      )}

      {config.ssoEnabled && (
        <button
          type="button"
          onClick={handleSSO}
          disabled={loading}
          className="w-full rounded border border-gray-300 bg-white px-4 py-2 text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          {loading ? 'Redirecting...' : 'Sign in with Google'}
        </button>
      )}

      {!config.passwordEnabled && !config.ssoEnabled && (
        <div className="rounded bg-yellow-100 p-4 text-yellow-700">
          Sign-in methods are not configured. Contact your administrator.
        </div>
      )}
    </div>
  )
}
