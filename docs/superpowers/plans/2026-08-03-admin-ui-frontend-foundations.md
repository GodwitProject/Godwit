# Admin UI Frontend Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational Next.js admin dashboard shell with auth, protected routes, reusable components, and one fully-functional resource (organizations) to establish patterns for phase C.

**Architecture:** Three-tier components (primitives → smart → pages) with server-side data fetching, RBAC scoping at every layer, and Server Actions for mutations. One resource (organizations) fully implemented as a reference pattern for phase C's remaining resources.

**Tech Stack:**
- Next.js 14+ (App Router, Server Components, Server Actions)
- shadecn/ui + Tailwind CSS
- Recharts (graphs)
- TanStack Table (data tables)
- TypeScript (strict mode)
- Vitest + React Testing Library (unit tests)
- Playwright (E2E tests)

## Global Constraints

- **Next.js version:** 14.0 or later (App Router required)
- **Node.js:** 18+
- **Package manager:** npm (or yarn/pnpm, but use lockfile consistently)
- **TypeScript:** strict mode enabled throughout
- **No `any` types:** all code must be fully typed
- **Naming:** kebab-case for files/routes, camelCase for functions/variables, PascalCase for components
- **Shadecn components:** installed via `npx shadecn-ui@latest add <component>` (don't copy code manually)
- **API base URL:** `https://api.godwit.io` (pulled from environment variable `NEXT_PUBLIC_API_URL`)
- **Token storage:** httpOnly cookies (never localStorage)
- **RBAC:** super_admin sees all, org_admin sees own org only, team_admin/user have no dashboard access

---

### Task 1: Scaffold Next.js App & Dependencies

**Files:**
- Create: `apps/admin/` (new directory)
- Create: `apps/admin/package.json`
- Create: `apps/admin/next.config.ts`
- Create: `apps/admin/tsconfig.json`
- Create: `apps/admin/tailwind.config.ts`
- Create: `apps/admin/.env.local`
- Create: `apps/admin/app/layout.tsx`
- Create: `apps/admin/app/page.tsx`
- Modify: Root `package.json` (workspace configuration, if using monorepo)

**Interfaces:**
- Produces: Next.js project structure, build system ready, environment variables configured

- [ ] **Step 1: Create `apps/admin/` directory and initialize package.json**

```bash
mkdir -p apps/admin
cd apps/admin
npm init -y
```

Edit `apps/admin/package.json`:
```json
{
  "name": "godwit-admin",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "type-check": "tsc --noEmit",
    "test": "vitest",
    "test:e2e": "playwright test"
  },
  "dependencies": {
    "next": "^14.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "typescript": "^5.3.0",
    "@types/node": "^20.0.0",
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "tailwindcss": "^3.3.0",
    "postcss": "^8.4.0",
    "autoprefixer": "^10.4.0",
    "@radix-ui/react-dialog": "^1.1.0",
    "@radix-ui/react-dropdown-menu": "^2.0.0",
    "@radix-ui/react-select": "^2.0.0",
    "class-variance-authority": "^0.7.0",
    "clsx": "^2.0.0",
    "tailwind-merge": "^2.2.0",
    "@tanstack/react-table": "^8.10.0",
    "recharts": "^2.10.0",
    "axios": "^1.6.0"
  },
  "devDependencies": {
    "@testing-library/react": "^14.0.0",
    "@testing-library/jest-dom": "^6.0.0",
    "vitest": "^0.34.0",
    "@vitest/ui": "^0.34.0",
    "@playwright/test": "^1.40.0",
    "eslint": "^8.50.0",
    "eslint-config-next": "^14.0.0"
  }
}
```

- [ ] **Step 2: Install dependencies**

```bash
npm install
```

- [ ] **Step 3: Create Next.js configuration**

File: `apps/admin/next.config.ts`
```typescript
import type { NextConfig } from 'next'

const config: NextConfig = {
  reactStrictMode: true,
  swcMinify: true,
  typescript: {
    tsconfigPath: './tsconfig.json',
  },
  eslint: {
    dirs: ['app', 'components', 'lib'],
  },
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io',
  },
}

export default config
```

- [ ] **Step 4: Create TypeScript configuration**

File: `apps/admin/tsconfig.json`
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "jsx": "preserve",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitAny": true,
    "noImplicitThis": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "strictBindCallApply": true,
    "strictPropertyInitialization": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "incremental": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx"],
  "exclude": ["node_modules"]
}
```

- [ ] **Step 5: Create Tailwind & PostCSS configuration**

File: `apps/admin/tailwind.config.ts`
```typescript
import type { Config } from 'tailwindcss'

const config: Config = {
  content: [
    './app/**/*.{js,ts,jsx,tsx,mdx}',
    './components/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}

export default config
```

File: `apps/admin/postcss.config.js`
```javascript
module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}
```

- [ ] **Step 6: Create app directory structure**

```bash
mkdir -p app/{auth,admin} components/{ui,admin,layout} lib public
touch app/layout.tsx app/page.tsx app/globals.css
```

- [ ] **Step 7: Create .env.local**

File: `apps/admin/.env.local`
```
NEXT_PUBLIC_API_URL=https://api.godwit.io
NEXT_PUBLIC_APP_URL=http://localhost:3000
```

- [ ] **Step 8: Create root layout and globals.css**

File: `apps/admin/app/globals.css`
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
    'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

File: `apps/admin/app/layout.tsx`
```typescript
import type { Metadata } from 'next'
import './globals.css'

export const metadata: Metadata = {
  title: 'Godwit Admin',
  description: 'Admin dashboard for Godwit',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
```

File: `apps/admin/app/page.tsx`
```typescript
import { redirect } from 'next/navigation'

export default function Home() {
  redirect('/login')
}
```

- [ ] **Step 9: Run the dev server to verify setup**

```bash
npm run dev
```

Expected: Dev server runs at http://localhost:3000 without errors.

- [ ] **Step 10: Commit**

```bash
git add apps/admin/
git commit -m "feat(admin): scaffold Next.js app with dependencies and configuration"
```

---

### Task 2: Auth Infrastructure (Middleware, Token Management, Hooks)

**Files:**
- Create: `apps/admin/middleware.ts`
- Create: `apps/admin/lib/auth.ts`
- Create: `apps/admin/lib/hooks.ts`
- Create: `apps/admin/lib/types.ts`

**Interfaces:**
- Produces: `getCurrentUser()` hook, `getAccessToken()` server-side function, `refreshToken()` Server Action, middleware that protects `/admin/*` routes

- [ ] **Step 1: Create types file**

File: `apps/admin/lib/types.ts`
```typescript
export interface User {
  id: string
  email: string
  name?: string
  role: 'super_admin' | 'org_admin' | 'team_admin' | 'user'
  organization_id?: string
  created_at: string
}

export interface Claims {
  sub: string
  user_id: string
  organization_id?: string
  role: string
  exp: number
  iat: number
}
```

- [ ] **Step 2: Create auth utility functions**

File: `apps/admin/lib/auth.ts`
```typescript
import { jwtDecode } from 'jwt-decode'
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

export async function getCurrentUser(): Promise<User | null> {
  const token = await getAccessToken()
  if (!token) return null

  const claims = await getClaimsFromToken(token)
  if (!claims) return null

  return {
    id: claims.user_id,
    email: '', // TODO: fetch from API if needed
    role: claims.role as User['role'],
    organization_id: claims.organization_id,
    created_at: new Date(claims.iat * 1000).toISOString(),
  }
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
```

- [ ] **Step 3: Create custom hooks for client components**

File: `apps/admin/lib/hooks.ts`
```typescript
'use client'

import { useEffect, useState } from 'react'
import { User } from './types'

export function useUser() {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    // Fetch user from a /api/auth/me endpoint (Server Action)
    // This is called client-side to get the user context for UI
    const fetchUser = async () => {
      try {
        const res = await fetch('/api/auth/me')
        if (res.ok) {
          setUser(await res.json())
        }
      } catch (err) {
        console.error('Failed to fetch user:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchUser()
  }, [])

  return { user, loading }
}
```

- [ ] **Step 4: Create middleware to protect routes**

File: `apps/admin/middleware.ts`
```typescript
import { NextRequest, NextResponse } from 'next/server'
import { jwtDecode } from 'jwt-decode'

export async function middleware(request: NextRequest) {
  const accessToken = request.cookies.get('access_token')?.value
  const refreshToken = request.cookies.get('refresh_token')?.value

  // Allow public routes
  if (request.nextUrl.pathname === '/login' || request.nextUrl.pathname === '/auth/callback') {
    // If already logged in, redirect to /admin
    if (accessToken) {
      return NextResponse.redirect(new URL('/admin', request.url))
    }
    return NextResponse.next()
  }

  // Protect /admin routes
  if (request.nextUrl.pathname.startsWith('/admin')) {
    if (!accessToken) {
      return NextResponse.redirect(new URL('/login', request.url))
    }

    try {
      const decoded = jwtDecode(accessToken) as any
      if (decoded.exp * 1000 < Date.now()) {
        // Token expired, redirect to login
        // (refresh logic should happen in a Server Action, not middleware)
        return NextResponse.redirect(new URL('/login', request.url))
      }
    } catch {
      return NextResponse.redirect(new URL('/login', request.url))
    }
  }

  // Root redirect
  if (request.nextUrl.pathname === '/') {
    if (accessToken) {
      return NextResponse.redirect(new URL('/admin', request.url))
    }
    return NextResponse.redirect(new URL('/login', request.url))
  }

  return NextResponse.next()
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico).*)'],
}
```

- [ ] **Step 5: Commit**

```bash
git add apps/admin/lib/ apps/admin/middleware.ts
git commit -m "feat(admin): add auth infrastructure (tokens, middleware, hooks)"
```

---

### Task 3: Login Page (Password + SSO UI)

**Files:**
- Create: `apps/admin/app/(auth)/login/page.tsx`
- Create: `apps/admin/app/(auth)/login/page.module.css`
- Create: `apps/admin/app/(auth)/layout.tsx`
- Create: `apps/admin/lib/api-client.ts` (API wrapper, needed by login actions)

**Interfaces:**
- Consumes: `setTokens()` from auth.ts
- Produces: Login page that calls `/api/auth/login` (password) and `/api/auth/oidc/authorize` (SSO)

- [ ] **Step 1: Create login page UI**

File: `apps/admin/app/(auth)/layout.tsx`
```typescript
export default function AuthLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <div className="flex h-screen items-center justify-center bg-gradient-to-b from-slate-100 to-slate-200">
      {children}
    </div>
  )
}
```

File: `apps/admin/app/(auth)/login/page.tsx`
```typescript
'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
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

      {config.passwordEnabled && config.ssoEnabled && <div className="relative">
        <div className="absolute inset-0 flex items-center">
          <div className="w-full border-t border-gray-300"></div>
        </div>
        <div className="relative flex justify-center text-sm">
          <span className="bg-white px-2 text-gray-500">Or</span>
        </div>
      </div>}

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
```

File: `apps/admin/app/(auth)/login/actions.ts`
```typescript
'use server'

import { redirect } from 'next/navigation'
import { setTokens } from '@/lib/auth'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

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
  // Redirect to OIDC authorize endpoint
  // This will be handled by the backend's OIDC endpoint
  redirect(`${API_URL}/api/v1/auth/oidc/authorize?redirect_uri=${process.env.NEXT_PUBLIC_APP_URL}/auth/callback`)
}
```

- [ ] **Step 2: Create API client wrapper**

File: `apps/admin/lib/api-client.ts`
```typescript
import { getAccessToken, setTokens, getRefreshToken, clearTokens } from './auth'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

export async function apiCall(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  const token = await getAccessToken()

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...options.headers,
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
```

- [ ] **Step 3: Commit**

```bash
git add apps/admin/app/\(auth\)/ apps/admin/lib/api-client.ts
git commit -m "feat(admin): implement login page with password + SSO UI"
```

---

### Task 4: OIDC Callback Handler

**Files:**
- Create: `apps/admin/app/(auth)/auth/callback/page.tsx`

**Interfaces:**
- Consumes: `setTokens()` from auth.ts, backend's OIDC callback endpoint
- Produces: OIDC callback handler that exchanges code for tokens and redirects to `/admin`

- [ ] **Step 1: Create callback page**

File: `apps/admin/app/(auth)/auth/callback/page.tsx`
```typescript
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
```

- [ ] **Step 2: Create backend callback bridge (Server Action)**

File: `apps/admin/app/(auth)/auth/callback/actions.ts`
```typescript
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
```

- [ ] **Step 3: Commit**

```bash
git add apps/admin/app/\(auth\)/auth/
git commit -m "feat(admin): implement OIDC callback handler"
```

---

### Task 5: Dashboard Layout (Sidebar, Top Bar, Root Layout)

**Files:**
- Create: `apps/admin/components/layout/sidebar.tsx`
- Create: `apps/admin/components/layout/top-bar.tsx`
- Create: `apps/admin/app/(dashboard)/layout.tsx`
- Create: `apps/admin/lib/logout.ts`

**Interfaces:**
- Consumes: `getCurrentUser()`, `useUser()` hook
- Produces: Dashboard layout with sidebar nav and top bar

- [ ] **Step 1: Create sidebar component**

File: `apps/admin/components/layout/sidebar.tsx`
```typescript
'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'

const navigation = [
  { name: 'Dashboard', href: '/admin' },
  { name: 'Organizations', href: '/admin/organizations' },
  { name: 'Teams', href: '/admin/teams' },
  { name: 'Users', href: '/admin/users' },
  { name: 'API Keys', href: '/admin/api-keys' },
  { name: 'Models', href: '/admin/models' },
  { name: 'Spend', href: '/admin/spend' },
]

export function Sidebar() {
  const pathname = usePathname()

  return (
    <nav className="w-64 border-r border-gray-200 bg-gray-50 px-4 py-6">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-gray-900">Godwit</h1>
        <p className="text-sm text-gray-600">Admin Dashboard</p>
      </div>

      <ul className="space-y-2">
        {navigation.map((item) => (
          <li key={item.href}>
            <Link
              href={item.href}
              className={`block rounded px-4 py-2 text-sm font-medium ${
                pathname === item.href
                  ? 'bg-blue-100 text-blue-900'
                  : 'text-gray-700 hover:bg-gray-100'
              }`}
            >
              {item.name}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  )
}
```

- [ ] **Step 2: Create top bar component**

File: `apps/admin/components/layout/top-bar.tsx`
```typescript
'use client'

import { useUser } from '@/lib/hooks'
import { logoutAction } from '@/lib/logout'

export function TopBar() {
  const { user, loading } = useUser()

  if (loading) {
    return (
      <div className="border-b border-gray-200 bg-white px-6 py-4">
        <p className="text-sm text-gray-600">Loading...</p>
      </div>
    )
  }

  return (
    <div className="border-b border-gray-200 bg-white px-6 py-4 flex justify-between items-center">
      <div></div>

      <div className="flex items-center space-x-4">
        {user && (
          <>
            <div className="text-right">
              <p className="text-sm font-medium text-gray-900">{user.email}</p>
              <p className="text-xs text-gray-600 capitalize">{user.role.replace('_', ' ')}</p>
            </div>

            <button
              onClick={() => logoutAction()}
              className="text-sm text-gray-600 hover:text-gray-900"
            >
              Logout
            </button>
          </>
        )}
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Create logout action**

File: `apps/admin/lib/logout.ts`
```typescript
'use server'

import { redirect } from 'next/navigation'
import { clearTokens } from './auth'

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'

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
```

- [ ] **Step 4: Create dashboard layout**

File: `apps/admin/app/(dashboard)/layout.tsx`
```typescript
import { getCurrentUser } from '@/lib/auth'
import { redirect } from 'next/navigation'
import { Sidebar } from '@/components/layout/sidebar'
import { TopBar } from '@/components/layout/top-bar'

export default async function DashboardLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const user = await getCurrentUser()

  // Only super_admin and org_admin can access dashboard
  if (!user || !['super_admin', 'org_admin'].includes(user.role)) {
    redirect('/')
  }

  return (
    <div className="flex h-screen">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <TopBar />
        <main className="flex-1 overflow-auto bg-gray-100 p-8">
          {children}
        </main>
      </div>
    </div>
  )
}
```

- [ ] **Step 5: Commit**

```bash
git add apps/admin/components/layout/ apps/admin/lib/logout.ts apps/admin/app/\(dashboard\)/layout.tsx
git commit -m "feat(admin): implement dashboard layout with sidebar and top bar"
```

---

### Task 6: Dashboard Home Page (Stats, Spend Graph, Recent Activity)

**Files:**
- Create: `apps/admin/app/(dashboard)/page.tsx`
- Create: `apps/admin/components/admin/stat-card.tsx`
- Create: `apps/admin/components/admin/spend-graph.tsx`

**Interfaces:**
- Consumes: `apiCall()`, `getCurrentUser()`, Recharts for graphs
- Produces: Dashboard home page with scoped stats, spend graph, recent activity

- [ ] **Step 1: Create stat card component**

File: `apps/admin/components/admin/stat-card.tsx`
```typescript
export function StatCard({
  title,
  value,
  description,
}: {
  title: string
  value: string | number
  description?: string
}) {
  return (
    <div className="rounded-lg bg-white p-6 shadow">
      <p className="text-sm font-medium text-gray-600">{title}</p>
      <p className="mt-2 text-3xl font-bold text-gray-900">{value}</p>
      {description && <p className="mt-2 text-sm text-gray-500">{description}</p>}
    </div>
  )
}
```

- [ ] **Step 2: Create spend graph component**

File: `apps/admin/components/admin/spend-graph.tsx`
```typescript
'use client'

import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

export function SpendGraph({
  data,
}: {
  data: Array<{ date: string; cost: number }>
}) {
  if (!data || data.length === 0) {
    return (
      <div className="rounded-lg bg-white p-6 shadow">
        <h3 className="text-lg font-semibold text-gray-900">Spend (Last 30 Days)</h3>
        <p className="mt-4 text-center text-gray-600">No data available</p>
      </div>
    )
  }

  return (
    <div className="rounded-lg bg-white p-6 shadow">
      <h3 className="text-lg font-semibold text-gray-900">Spend (Last 30 Days)</h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="date" />
          <YAxis />
          <Tooltip formatter={(value) => `$${Number(value).toFixed(2)}`} />
          <Line type="monotone" dataKey="cost" stroke="#3b82f6" dot={false} />
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}
```

- [ ] **Step 3: Create dashboard home page**

File: `apps/admin/app/(dashboard)/page.tsx`
```typescript
import { getCurrentUser } from '@/lib/auth'
import { apiCall } from '@/lib/api-client'
import { StatCard } from '@/components/admin/stat-card'
import { SpendGraph } from '@/components/admin/spend-graph'

export default async function DashboardPage() {
  const user = await getCurrentUser()

  if (!user) {
    return <div>User not found</div>
  }

  // Fetch dashboard stats (scoped by user's organization if org_admin)
  let statsUrl = '/api/v1/admin/stats'
  if (user.role === 'org_admin') {
    statsUrl += `?organization_id=${user.organization_id}`
  }

  // Fetch spend data for graph (last 30 days)
  let spendUrl = '/api/v1/spend?days=30'
  if (user.role === 'org_admin') {
    spendUrl += `&organization_id=${user.organization_id}`
  }

  let stats = { organizations: 0, teams: 0, users: 0, apiKeys: 0 }
  let spendData: Array<{ date: string; cost: number }> = []
  let recentActivity: Array<{ id: string; type: string; name: string; created_at: string }> = []

  try {
    const statsResponse = await apiCall(statsUrl)
    if (statsResponse.ok) {
      stats = await statsResponse.json()
    }
  } catch (err) {
    console.error('Failed to fetch stats:', err)
  }

  try {
    const spendResponse = await apiCall(spendUrl)
    if (spendResponse.ok) {
      const data = await spendResponse.json()
      spendData = data.data || []
    }
  } catch (err) {
    console.error('Failed to fetch spend data:', err)
  }

  try {
    let activityUrl = '/api/v1/admin/recent-activity?limit=5'
    if (user.role === 'org_admin') {
      activityUrl += `&organization_id=${user.organization_id}`
    }
    const activityResponse = await apiCall(activityUrl)
    if (activityResponse.ok) {
      const data = await activityResponse.json()
      recentActivity = data.data || []
    }
  } catch (err) {
    console.error('Failed to fetch recent activity:', err)
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
        <p className="mt-2 text-gray-600">Welcome back, {user.email}</p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4">
        <StatCard title="Organizations" value={stats.organizations} />
        <StatCard title="Teams" value={stats.teams} />
        <StatCard title="Users" value={stats.users} />
        <StatCard title="API Keys" value={stats.apiKeys} />
      </div>

      {/* Spend Graph */}
      <SpendGraph data={spendData} />

      {/* Recent Activity */}
      <div className="rounded-lg bg-white p-6 shadow">
        <h3 className="text-lg font-semibold text-gray-900">Recent Activity</h3>
        {recentActivity.length === 0 ? (
          <p className="mt-4 text-gray-600">No recent activity</p>
        ) : (
          <table className="mt-4 w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200">
                <th className="text-left font-medium text-gray-600">Type</th>
                <th className="text-left font-medium text-gray-600">Name</th>
                <th className="text-left font-medium text-gray-600">Created</th>
              </tr>
            </thead>
            <tbody>
              {recentActivity.map((item) => (
                <tr key={item.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="py-3 capitalize text-gray-700">{item.type}</td>
                  <td className="py-3 text-gray-900">{item.name}</td>
                  <td className="py-3 text-gray-500">
                    {new Date(item.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
```

- [ ] **Step 4: Commit**

```bash
git add apps/admin/app/\(dashboard\)/page.tsx apps/admin/components/admin/
git commit -m "feat(admin): implement dashboard home page with stats, graph, and recent activity"
```

---

### Task 7: Reusable Components (DataTable, FormDialog, PageHeader, EmptyState, ListPage)

**Files:**
- Create: `apps/admin/components/ui/data-table.tsx`
- Create: `apps/admin/components/ui/form-dialog.tsx`
- Create: `apps/admin/components/ui/page-header.tsx`
- Create: `apps/admin/components/ui/empty-state.tsx`
- Create: `apps/admin/components/admin/list-page.tsx`

**Interfaces:**
- Produces: Reusable component hierarchy for all resource pages

- [ ] **Step 1: Create PageHeader component**

File: `apps/admin/components/ui/page-header.tsx`
```typescript
export function PageHeader({
  title,
  description,
  action,
}: {
  title: string
  description?: string
  action?: { label: string; onClick: () => void }
}) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">{title}</h1>
        {description && <p className="mt-2 text-gray-600">{description}</p>}
      </div>
      {action && (
        <button
          onClick={action.onClick}
          className="rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
        >
          {action.label}
        </button>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Create EmptyState component**

File: `apps/admin/components/ui/empty-state.tsx`
```typescript
export function EmptyState({
  message = 'No items found',
  action,
}: {
  message?: string
  action?: { label: string; onClick: () => void }
}) {
  return (
    <div className="rounded-lg border-2 border-dashed border-gray-300 bg-gray-50 p-12 text-center">
      <p className="text-gray-600">{message}</p>
      {action && (
        <button
          onClick={action.onClick}
          className="mt-4 rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
        >
          {action.label}
        </button>
      )}
    </div>
  )
}
```

- [ ] **Step 3: Create DataTable component (stub for now, will use TanStack Table)**

File: `apps/admin/components/ui/data-table.tsx`
```typescript
'use client'

import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getPaginationRowModel,
  ColumnDef,
  flexRender,
} from '@tanstack/react-table'
import { useState } from 'react'

export function DataTable<T>({
  columns,
  data,
  onRowClick,
}: {
  columns: ColumnDef<T>[]
  data: T[]
  onRowClick?: (row: T) => void
}) {
  const [sorting, setSorting] = useState([])

  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    state: { sorting },
    onSortingChange: setSorting,
  })

  return (
    <div className="overflow-x-auto rounded-lg border border-gray-200 bg-white">
      <table className="w-full text-sm">
        <thead className="border-b border-gray-200 bg-gray-50">
          {table.getHeaderGroups().map((headerGroup) => (
            <tr key={headerGroup.id}>
              {headerGroup.headers.map((header) => (
                <th
                  key={header.id}
                  className="px-6 py-3 text-left font-medium text-gray-700 cursor-pointer hover:bg-gray-100"
                  onClick={header.column.getToggleSortingHandler()}
                >
                  {flexRender(header.column.columnDef.header, header.getContext())}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody>
          {table.getRowModel().rows.length === 0 ? (
            <tr>
              <td colSpan={columns.length} className="py-8 text-center text-gray-500">
                No data
              </td>
            </tr>
          ) : (
            table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                className="border-b border-gray-100 hover:bg-gray-50 cursor-pointer"
                onClick={() => onRowClick?.(row.original)}
              >
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-6 py-4">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>

      {/* Pagination */}
      <div className="flex items-center justify-between border-t border-gray-200 bg-gray-50 px-6 py-4">
        <span className="text-sm text-gray-600">
          Page {table.getState().pagination.pageIndex + 1} of {table.getPageCount()}
        </span>
        <div className="flex space-x-2">
          <button
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
            className="rounded border border-gray-300 px-3 py-1 disabled:opacity-50"
          >
            Previous
          </button>
          <button
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
            className="rounded border border-gray-300 px-3 py-1 disabled:opacity-50"
          >
            Next
          </button>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 4: Create FormDialog component**

File: `apps/admin/components/ui/form-dialog.tsx`
```typescript
'use client'

import { useState } from 'react'

export function FormDialog({
  isOpen,
  title,
  children,
  onSubmit,
  onClose,
  submitLabel = 'Save',
  isLoading = false,
}: {
  isOpen: boolean
  title: string
  children: React.ReactNode
  onSubmit: (formData: FormData) => Promise<void>
  onClose: () => void
  submitLabel?: string
  isLoading?: boolean
}) {
  const [error, setError] = useState('')

  if (!isOpen) return null

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    setError('')

    try {
      const formData = new FormData(e.currentTarget)
      await onSubmit(formData)
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred')
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="relative max-h-full w-full max-w-md overflow-auto rounded-lg bg-white p-6 shadow-lg">
        <h2 className="text-xl font-bold text-gray-900">{title}</h2>

        {error && <div className="mt-4 rounded bg-red-100 p-3 text-red-700">{error}</div>}

        <form onSubmit={handleSubmit} className="mt-6 space-y-4">
          {children}

          <div className="flex justify-end space-x-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="rounded border border-gray-300 px-4 py-2 text-gray-700 hover:bg-gray-50"
              disabled={isLoading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
              disabled={isLoading}
            >
              {isLoading ? 'Saving...' : submitLabel}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
```

- [ ] **Step 5: Create ListPage component**

File: `apps/admin/components/admin/list-page.tsx`
```typescript
'use client'

import { ColumnDef } from '@tanstack/react-table'
import { PageHeader } from '@/components/ui/page-header'
import { DataTable } from '@/components/ui/data-table'
import { EmptyState } from '@/components/ui/empty-state'

export function ListPage<T>({
  data,
  columns,
  title,
  description,
  isEmpty,
  onCreateClick,
  emptyStateMessage,
}: {
  data: T[]
  columns: ColumnDef<T>[]
  title: string
  description?: string
  isEmpty?: boolean
  onCreateClick: () => void
  emptyStateMessage?: string
}) {
  return (
    <div className="space-y-6">
      <PageHeader
        title={title}
        description={description}
        action={{ label: 'Create', onClick: onCreateClick }}
      />

      {isEmpty ? (
        <EmptyState
          message={emptyStateMessage || 'No items found'}
          action={{ label: 'Create New', onClick: onCreateClick }}
        />
      ) : (
        <DataTable columns={columns} data={data} />
      )}
    </div>
  )
}
```

- [ ] **Step 6: Commit**

```bash
git add apps/admin/components/ui/ apps/admin/components/admin/list-page.tsx
git commit -m "feat(admin): implement reusable components (DataTable, FormDialog, PageHeader, ListPage)"
```

---

### Task 8: Organizations Resource (List + Create/Edit/Delete)

**Files:**
- Create: `apps/admin/app/(dashboard)/admin/organizations/page.tsx`
- Create: `apps/admin/app/(dashboard)/admin/organizations/[id]/page.tsx`
- Create: `apps/admin/app/(dashboard)/admin/organizations/actions.ts`

**Interfaces:**
- Consumes: `ListPage`, `DataTable`, `FormDialog`, `apiCall`
- Produces: Full CRUD for organizations (list, create, edit, delete)

- [ ] **Step 1: Create organizations list page**

File: `apps/admin/app/(dashboard)/admin/organizations/page.tsx`
```typescript
'use client'

import { useState, useEffect } from 'react'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { apiCall } from '@/lib/api-client'
import { createOrganization } from './actions'

interface Organization {
  id: string
  name: string
  created_at: string
}

const columns: ColumnDef<Organization>[] = [
  {
    accessorKey: 'name',
    header: 'Name',
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function OrganizationsPage() {
  const [organizations, setOrganizations] = useState<Organization[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchOrganizations = async () => {
      try {
        const response = await apiCall('/api/v1/organizations')
        if (response.ok) {
          const data = await response.json()
          setOrganizations(data.data || [])
        }
      } catch (err) {
        console.error('Failed to fetch organizations:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchOrganizations()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const result = await createOrganization(name)
    
    if (result.success) {
      setOrganizations([...organizations, result.organization])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create organization')
    }
  }

  return (
    <>
      <ListPage
        data={organizations}
        columns={columns}
        title="Organizations"
        isEmpty={organizations.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create Organization"
        onSubmit={handleCreateSubmit}
        onClose={() => setIsCreateDialogOpen(false)}
      >
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            id="name"
            name="name"
            type="text"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
```

- [ ] **Step 2: Create organizations actions (Server Actions)**

File: `apps/admin/app/(dashboard)/admin/organizations/actions.ts`
```typescript
'use server'

import { apiCall } from '@/lib/api-client'

interface Organization {
  id: string
  name: string
  created_at: string
}

export async function createOrganization(
  name: string
): Promise<{ success: boolean; organization?: Organization; error?: string }> {
  try {
    const response = await apiCall('/api/v1/organizations', {
      method: 'POST',
      body: JSON.stringify({ name }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create organization' }
    }

    const data = await response.json()
    return { success: true, organization: data.data }
  } catch (err) {
    console.error('Create organization error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function updateOrganization(
  id: string,
  name: string
): Promise<{ success: boolean; organization?: Organization; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/organizations/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ name }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to update organization' }
    }

    const data = await response.json()
    return { success: true, organization: data.data }
  } catch (err) {
    console.error('Update organization error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function deleteOrganization(
  id: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/organizations/${id}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to delete organization' }
    }

    return { success: true }
  } catch (err) {
    console.error('Delete organization error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
```

- [ ] **Step 3: Create organization detail/edit page**

File: `apps/admin/app/(dashboard)/admin/organizations/[id]/page.tsx`
```typescript
'use client'

import { useState, useEffect } from 'react'
import { useParams } from 'next/navigation'
import { PageHeader } from '@/components/ui/page-header'
import { FormDialog } from '@/components/ui/form-dialog'
import { apiCall } from '@/lib/api-client'
import { updateOrganization, deleteOrganization } from '../actions'

interface Organization {
  id: string
  name: string
  created_at: string
}

export default function OrganizationDetailPage() {
  const { id } = useParams() as { id: string }
  const [organization, setOrganization] = useState<Organization | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)

  useEffect(() => {
    const fetchOrganization = async () => {
      try {
        const response = await apiCall(`/api/v1/organizations/${id}`)
        if (response.ok) {
          const data = await response.json()
          setOrganization(data.data)
        }
      } catch (err) {
        console.error('Failed to fetch organization:', err)
      } finally {
        setIsLoading(false)
      }
    }

    fetchOrganization()
  }, [id])

  const handleEditSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const result = await updateOrganization(id, name)
    
    if (result.success && result.organization) {
      setOrganization(result.organization)
      setIsEditDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to update organization')
    }
  }

  const handleDelete = async () => {
    if (!confirm('Are you sure you want to delete this organization?')) return

    const result = await deleteOrganization(id)
    if (result.success) {
      window.location.href = '/admin/organizations'
    } else {
      alert(result.error || 'Failed to delete organization')
    }
  }

  if (isLoading) return <div>Loading...</div>
  if (!organization) return <div>Organization not found</div>

  return (
    <>
      <div className="space-y-6">
        <PageHeader
          title={organization.name}
          action={{ label: 'Edit', onClick: () => setIsEditDialogOpen(true) }}
        />

        <div className="rounded-lg bg-white p-6 shadow">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-gray-600">Created</p>
              <p className="text-lg font-semibold text-gray-900">
                {new Date(organization.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>

          <button
            onClick={handleDelete}
            className="mt-6 rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
          >
            Delete Organization
          </button>
        </div>
      </div>

      <FormDialog
        isOpen={isEditDialogOpen}
        title="Edit Organization"
        onSubmit={handleEditSubmit}
        onClose={() => setIsEditDialogOpen(false)}
      >
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            id="name"
            name="name"
            type="text"
            defaultValue={organization.name}
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
```

- [ ] **Step 4: Commit**

```bash
git add apps/admin/app/\(dashboard\)/admin/organizations/
git commit -m "feat(admin): implement organizations CRUD (list, create, edit, delete)"
```

---

### Task 9: Component Tests (Vitest + React Testing Library)

**Files:**
- Create: `apps/admin/components/ui/__tests__/data-table.test.tsx`
- Create: `apps/admin/components/ui/__tests__/page-header.test.tsx`
- Create: `apps/admin/components/ui/__tests__/form-dialog.test.tsx`
- Create: `apps/admin/vitest.config.ts`

**Interfaces:**
- Produces: Unit tests for all reusable components

- [ ] **Step 1: Create Vitest configuration**

File: `apps/admin/vitest.config.ts`
```typescript
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./test/setup.ts'],
  },
})
```

File: `apps/admin/test/setup.ts`
```typescript
import '@testing-library/jest-dom'
```

- [ ] **Step 2: Create PageHeader tests**

File: `apps/admin/components/ui/__tests__/page-header.test.tsx`
```typescript
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PageHeader } from '../page-header'

describe('PageHeader', () => {
  it('renders title', () => {
    render(<PageHeader title="Test Title" />)
    expect(screen.getByText('Test Title')).toBeInTheDocument()
  })

  it('renders description when provided', () => {
    render(<PageHeader title="Title" description="Test description" />)
    expect(screen.getByText('Test description')).toBeInTheDocument()
  })

  it('renders action button when provided', async () => {
    const handleClick = vi.fn()
    render(<PageHeader title="Title" action={{ label: 'Click me', onClick: handleClick }} />)

    const button = screen.getByRole('button', { name: /click me/i })
    await userEvent.click(button)

    expect(handleClick).toHaveBeenCalled()
  })
})
```

- [ ] **Step 3: Create DataTable tests**

File: `apps/admin/components/ui/__tests__/data-table.test.tsx`
```typescript
import { render, screen } from '@testing-library/react'
import { ColumnDef } from '@tanstack/react-table'
import { DataTable } from '../data-table'

interface TestRow {
  id: string
  name: string
}

describe('DataTable', () => {
  const columns: ColumnDef<TestRow>[] = [
    {
      accessorKey: 'name',
      header: 'Name',
    },
  ]

  const data: TestRow[] = [
    { id: '1', name: 'Row 1' },
    { id: '2', name: 'Row 2' },
  ]

  it('renders table with data', () => {
    render(<DataTable columns={columns} data={data} />)
    expect(screen.getByText('Row 1')).toBeInTheDocument()
    expect(screen.getByText('Row 2')).toBeInTheDocument()
  })

  it('renders "No data" when data is empty', () => {
    render(<DataTable columns={columns} data={[]} />)
    expect(screen.getByText('No data')).toBeInTheDocument()
  })

  it('renders column headers', () => {
    render(<DataTable columns={columns} data={data} />)
    expect(screen.getByText('Name')).toBeInTheDocument()
  })
})
```

- [ ] **Step 4: Create FormDialog tests**

File: `apps/admin/components/ui/__tests__/form-dialog.test.tsx`
```typescript
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { FormDialog } from '../form-dialog'
import { vi } from 'vitest'

describe('FormDialog', () => {
  it('does not render when isOpen is false', () => {
    const handleSubmit = vi.fn()
    const handleClose = vi.fn()

    const { container } = render(
      <FormDialog
        isOpen={false}
        title="Test Dialog"
        onSubmit={handleSubmit}
        onClose={handleClose}
      >
        <input name="test" />
      </FormDialog>
    )

    expect(container.firstChild).toBeEmptyDOMElement()
  })

  it('renders when isOpen is true', () => {
    const handleSubmit = vi.fn()
    const handleClose = vi.fn()

    render(
      <FormDialog
        isOpen={true}
        title="Test Dialog"
        onSubmit={handleSubmit}
        onClose={handleClose}
      >
        <input name="test" />
      </FormDialog>
    )

    expect(screen.getByText('Test Dialog')).toBeInTheDocument()
  })

  it('calls onClose when cancel button is clicked', async () => {
    const handleSubmit = vi.fn()
    const handleClose = vi.fn()

    render(
      <FormDialog
        isOpen={true}
        title="Test Dialog"
        onSubmit={handleSubmit}
        onClose={handleClose}
      >
        <input name="test" />
      </FormDialog>
    )

    const cancelButton = screen.getByRole('button', { name: /cancel/i })
    await userEvent.click(cancelButton)

    expect(handleClose).toHaveBeenCalled()
  })
})
```

- [ ] **Step 5: Update package.json with test dependencies**

Add to `apps/admin/package.json` devDependencies:
```json
{
  "devDependencies": {
    "@testing-library/user-event": "^14.0.0",
    "@vitejs/plugin-react": "^4.0.0",
    "vi": "^0.34.0"
  }
}
```

- [ ] **Step 6: Run tests**

```bash
npm test
```

Expected: All tests pass (6 tests in total).

- [ ] **Step 7: Commit**

```bash
git add apps/admin/components/ui/__tests__/ apps/admin/test/ apps/admin/vitest.config.ts apps/admin/package.json
git commit -m "test(admin): add unit tests for reusable components"
```

---

### Task 10: E2E Tests (Playwright)

**Files:**
- Create: `apps/admin/e2e/auth.spec.ts`
- Create: `apps/admin/e2e/dashboard.spec.ts`
- Create: `apps/admin/playwright.config.ts`

**Interfaces:**
- Produces: E2E tests for critical flows (login, protected routes, CRUD)

- [ ] **Step 1: Create Playwright configuration**

File: `apps/admin/playwright.config.ts`
```typescript
import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
  },
})
```

- [ ] **Step 2: Create auth tests**

File: `apps/admin/e2e/auth.spec.ts`
```typescript
import { test, expect } from '@playwright/test'

test.describe('Authentication', () => {
  test('login with password', async ({ page }) => {
    await page.goto('/login')

    // Fill form
    await page.fill('input[type="email"]', 'test@example.com')
    await page.fill('input[type="password"]', 'password123')

    // Submit
    await page.click('button:has-text("Sign in with password")')

    // Should redirect to dashboard
    await expect(page).toHaveURL('/admin')
    await expect(page.locator('h1')).toContainText('Dashboard')
  })

  test('redirect to login when not authenticated', async ({ page }) => {
    await page.goto('/admin')
    await expect(page).toHaveURL('/login')
  })

  test('redirect to dashboard when already logged in', async ({ page, context }) => {
    // Set auth cookie
    await context.addCookies([
      {
        name: 'access_token',
        value: 'test-token',
        domain: 'localhost',
        path: '/',
      },
    ])

    await page.goto('/login')
    await expect(page).toHaveURL('/admin')
  })

  test('logout clears cookies and redirects to login', async ({ page, context }) => {
    // Set auth cookie
    await context.addCookies([
      {
        name: 'access_token',
        value: 'test-token',
        domain: 'localhost',
        path: '/',
      },
    ])

    await page.goto('/admin')
    await page.click('button:has-text("Logout")')

    // Should redirect to login
    await expect(page).toHaveURL('/login')
  })
})
```

- [ ] **Step 3: Create dashboard tests**

File: `apps/admin/e2e/dashboard.spec.ts`
```typescript
import { test, expect } from '@playwright/test'

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page, context }) => {
    // Set auth cookie before each test
    await context.addCookies([
      {
        name: 'access_token',
        value: 'test-token',
        domain: 'localhost',
        path: '/',
      },
    ])
  })

  test('display dashboard home with stats', async ({ page }) => {
    await page.goto('/admin')

    // Should show stats
    await expect(page.locator('text=Organizations')).toBeVisible()
    await expect(page.locator('text=Teams')).toBeVisible()
    await expect(page.locator('text=Users')).toBeVisible()
    await expect(page.locator('text=API Keys')).toBeVisible()
  })

  test('navigate to organizations page', async ({ page }) => {
    await page.goto('/admin')
    await page.click('a:has-text("Organizations")')

    await expect(page).toHaveURL('/admin/organizations')
    await expect(page.locator('h1')).toContainText('Organizations')
  })

  test('create organization from dashboard', async ({ page }) => {
    await page.goto('/admin/organizations')

    // Click create button
    await page.click('button:has-text("Create")')

    // Fill form
    await page.fill('input[name="name"]', 'Test Org')

    // Submit
    await page.click('button:has-text("Save")')

    // Should show in list
    await expect(page.locator('text=Test Org')).toBeVisible()
  })
})
```

- [ ] **Step 4: Update package.json with test script**

Add to `apps/admin/package.json` scripts:
```json
{
  "scripts": {
    "test:e2e": "playwright test"
  }
}
```

- [ ] **Step 5: Commit**

```bash
git add apps/admin/e2e/ apps/admin/playwright.config.ts apps/admin/package.json
git commit -m "test(admin): add end-to-end tests for critical flows"
```

---

### Task 11: Documentation (README for Adding New Resources)

**Files:**
- Create: `apps/admin/README.md`

**Interfaces:**
- Produces: Documentation for developers to add new resource screens in phase C

- [ ] **Step 1: Create README**

File: `apps/admin/README.md`
```markdown
# Godwit Admin Dashboard

## Getting Started

```bash
cd apps/admin
npm install
npm run dev
```

Navigate to http://localhost:3000 and log in.

## Architecture

Three-tier component system:

1. **Tier 1 — Primitives** (`components/ui/`)
   - `<DataTable>` — generic table with sorting, filtering, pagination
   - `<FormDialog>` — modal form container
   - `<PageHeader>` — page title + action button
   - `<EmptyState>` — "no data" placeholder

2. **Tier 2 — Smart Components** (`components/admin/`)
   - `<ListPage>` — list layout (header + table + empty state)
   - `<EditDialog>` — edit modal with form handling
   - `<ResourceForm>` — base form for common fields

3. **Tier 3 — Resource Pages** (`app/(dashboard)/admin/[resource]/`)
   - Fetch data server-side
   - Define columns/fields
   - Render smart components

## Data Flow

1. **Page (Server Component)** → fetches data via `apiCall()`
2. **Components render** → handle interactions
3. **Form submit** → Server Action → API call → revalidate data

## Adding a New Resource

Example: adding a "Billing Plans" resource.

### Step 1: Create folder structure

```bash
mkdir -p app/(dashboard)/admin/billing-plans/[id]
touch app/(dashboard)/admin/billing-plans/page.tsx
touch app/(dashboard)/admin/billing-plans/[id]/page.tsx
touch app/(dashboard)/admin/billing-plans/actions.ts
```

### Step 2: Define your types

File: `app/(dashboard)/admin/billing-plans/types.ts`

```typescript
export interface BillingPlan {
  id: string
  name: string
  price_usd: number
  created_at: string
}
```

### Step 3: Create Server Actions

File: `app/(dashboard)/admin/billing-plans/actions.ts`

```typescript
'use server'

import { apiCall } from '@/lib/api-client'
import { BillingPlan } from './types'

export async function createBillingPlan(
  name: string,
  price_usd: number
): Promise<{ success: boolean; plan?: BillingPlan; error?: string }> {
  try {
    const response = await apiCall('/api/v1/billing-plans', {
      method: 'POST',
      body: JSON.stringify({ name, price_usd }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create billing plan' }
    }

    const data = await response.json()
    return { success: true, plan: data.data }
  } catch (err) {
    return { success: false, error: 'An error occurred' }
  }
}

// Similarly: updateBillingPlan, deleteBillingPlan
```

### Step 4: Create list page

File: `app/(dashboard)/admin/billing-plans/page.tsx`

```typescript
'use client'

import { useState, useEffect } from 'react'
import { ListPage } from '@/components/admin/list-page'
import { FormDialog } from '@/components/ui/form-dialog'
import { ColumnDef } from '@tanstack/react-table'
import { apiCall } from '@/lib/api-client'
import { BillingPlan } from './types'
import { createBillingPlan } from './actions'

const columns: ColumnDef<BillingPlan>[] = [
  { accessorKey: 'name', header: 'Name' },
  { accessorKey: 'price_usd', header: 'Price (USD)' },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => new Date(info.getValue() as string).toLocaleDateString(),
  },
]

export default function BillingPlansPage() {
  const [plans, setPlans] = useState<BillingPlan[]>([])
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)

  useEffect(() => {
    const fetchPlans = async () => {
      try {
        const response = await apiCall('/api/v1/billing-plans')
        if (response.ok) {
          const data = await response.json()
          setPlans(data.data || [])
        }
      } catch (err) {
        console.error('Failed to fetch billing plans:', err)
      }
    }

    fetchPlans()
  }, [])

  const handleCreateSubmit = async (formData: FormData) => {
    const name = formData.get('name') as string
    const price_usd = parseFloat(formData.get('price_usd') as string)

    const result = await createBillingPlan(name, price_usd)
    if (result.success && result.plan) {
      setPlans([...plans, result.plan])
      setIsCreateDialogOpen(false)
    } else {
      throw new Error(result.error || 'Failed to create billing plan')
    }
  }

  return (
    <>
      <ListPage
        data={plans}
        columns={columns}
        title="Billing Plans"
        isEmpty={plans.length === 0}
        onCreateClick={() => setIsCreateDialogOpen(true)}
      />

      <FormDialog
        isOpen={isCreateDialogOpen}
        title="Create Billing Plan"
        onSubmit={handleCreateSubmit}
        onClose={() => setIsCreateDialogOpen(false)}
      >
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700">
            Name
          </label>
          <input
            id="name"
            name="name"
            type="text"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>

        <div>
          <label htmlFor="price_usd" className="block text-sm font-medium text-gray-700">
            Price (USD)
          </label>
          <input
            id="price_usd"
            name="price_usd"
            type="number"
            step="0.01"
            required
            className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
          />
        </div>
      </FormDialog>
    </>
  )
}
```

### Step 5: Create detail page

Similar pattern to `app/(dashboard)/admin/organizations/[id]/page.tsx` — fetch by ID, render PageHeader with edit/delete actions, FormDialog for edit.

### Step 6: Add to sidebar

Update `components/layout/sidebar.tsx` to include the new resource in the `navigation` array.

### Step 7: Test

1. Unit tests: test the Server Actions in isolation
2. E2E tests: test the full CRUD flow in a browser
3. Manual test: navigate to the page, create/edit/delete an item

## Testing

### Unit Tests

```bash
npm test
```

### E2E Tests

```bash
npm run test:e2e
```

## API Integration

All API calls go through `lib/api-client.ts`, which:
- Reads the access token from httpOnly cookies (server-side only)
- Automatically refreshes tokens on 401
- Adds the `Authorization: Bearer <token>` header

To call an API:

```typescript
import { apiCall } from '@/lib/api-client'

const response = await apiCall('/api/v1/resource')
const data = await response.json()
```

## RBAC Scoping

Every page and API call respects the user's role:
- **super_admin**: unfiltered access
- **org_admin**: scoped to own organization
- **team_admin / user**: no dashboard access

Scoping is applied:
1. At the page level (middleware + `app/(dashboard)/layout.tsx`)
2. At the API level (query params like `?organization_id=...`)

## Environment Variables

- `NEXT_PUBLIC_API_URL`: Backend API base URL (default: `https://api.godwit.io`)
- `NEXT_PUBLIC_APP_URL`: Frontend app URL (for OIDC redirects, default: `http://localhost:3000`)

See `.env.local` for development values.
```

- [ ] **Step 2: Commit**

```bash
git add apps/admin/README.md
git commit -m "docs(admin): add README with architecture and resource-creation guide"
```

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-03-admin-ui-frontend-foundations.md`.

**Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using the skill, batch execution with checkpoints

**Which approach?**

