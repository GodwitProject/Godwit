import { getAccessToken, setTokens, getRefreshToken, clearTokens } from './auth'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function apiCall(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  const token = await getAccessToken()

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }

  if (options.headers instanceof Headers) {
    options.headers.forEach((value, key) => {
      headers[key] = value
    })
  } else if (options.headers) {
    Object.assign(headers, options.headers)
  }

  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  let response = await fetch(`${API_URL}${endpoint}`, {
    ...options,
    headers,
  })

  // Auto-refresh on 401
  if (response.status === 401 && token) {
    const refreshToken = await getRefreshToken()
    if (refreshToken) {
      try {
        const refreshResponse = await fetch(`${API_URL}/api/v1/auth/refresh`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ refresh_token: refreshToken }),
        })

        if (refreshResponse.ok) {
          const { access_token, refresh_token } = await refreshResponse.json()
          await setTokens(access_token, refresh_token)

          // Retry the original request
          headers['Authorization'] = `Bearer ${access_token}`
          response = await fetch(`${API_URL}${endpoint}`, {
            ...options,
            headers,
          })
        } else {
          await clearTokens()
        }
      } catch (err) {
        console.error('Refresh failed:', err)
        await clearTokens()
      }
    }
  }

  return response
}
