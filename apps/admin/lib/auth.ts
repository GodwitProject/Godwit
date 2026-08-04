import jwtDecode from 'jwt-decode'
import { cookies } from 'next/headers'
import { Claims, User } from './types'

export async function getAccessToken(): Promise<string | null> {
  const cookieStore = await cookies()
  return cookieStore.get('access_token')?.value || null
}

export async function getRefreshToken(): Promise<string | null> {
  const cookieStore = await cookies()
  return cookieStore.get('refresh_token')?.value || null
}

export async function setTokens(
  accessToken: string,
  refreshToken: string
): Promise<void> {
  const cookieStore = await cookies()

  // Access token: httpOnly, 15 min
  cookieStore.set('access_token', accessToken, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'strict',
    maxAge: 15 * 60, // 15 minutes
  })

  // Refresh token: httpOnly, 7 days
  cookieStore.set('refresh_token', refreshToken, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'strict',
    maxAge: 7 * 24 * 60 * 60, // 7 days
  })
}

export async function clearTokens(): Promise<void> {
  const cookieStore = await cookies()
  cookieStore.delete('access_token')
  cookieStore.delete('refresh_token')
}

export async function getClaimsFromToken(token: string): Promise<Claims | null> {
  try {
    return jwtDecode<Claims>(token)
  } catch {
    return null
  }
}

const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function getCurrentUser(): Promise<User | null> {
  const token = await getAccessToken()
  if (!token) return null

  const claims = await getClaimsFromToken(token)
  if (!claims) return null

  const base: User = {
    id: claims.user_id,
    email: '',
    role: claims.role as User['role'],
    organization_id: claims.organization_id,
    created_at: new Date(claims.iat * 1000).toISOString(),
  }

  // Best-effort: the JWT only carries id/org/role, not email. Roles without access to
  // /users (team_admin, user) simply keep the placeholder rather than failing the request.
  try {
    const response = await fetch(`${API_URL}/api/v1/users/${claims.user_id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    if (response.ok) {
      const { data } = await response.json()
      return { ...base, email: data.email, name: data.name ?? undefined }
    }
  } catch {
    // Network error reaching the API — fall through to the placeholder below.
  }
  return base
}

export async function isTokenExpired(token: string): Promise<boolean> {
  const claims = await getClaimsFromToken(token)
  if (!claims) return true
  return claims.exp * 1000 < Date.now()
}

export async function hasRole(requiredRoles: User['role'][]): Promise<boolean> {
  const user = await getCurrentUser()
  if (!user) return false
  return requiredRoles.includes(user.role)
}
