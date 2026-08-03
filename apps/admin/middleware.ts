import { NextRequest, NextResponse } from 'next/server'
import jwtDecode from 'jwt-decode'
import { Claims } from './lib/types'

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
      const decoded = jwtDecode<Claims>(accessToken)
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
