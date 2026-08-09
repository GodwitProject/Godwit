import { request, expect } from '@playwright/test'

// The Godwit backend is reachable from the E2E runner (same process/network).
// Override via env when the port differs.
export const API_URL = process.env.E2E_API_URL || 'http://localhost:8080'
export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL || 'test@example.com'
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD || 'password123'

export interface AdminSession {
  accessToken: string
  refreshToken: string
}

/**
 * Log into the backend as the seeded admin and return a real token pair.
 * Used instead of a hardcoded cookie so the dashboard/auth middleware
 * (which jwt-decodes the access token) and backend API calls both accept it.
 */
export async function loginAsAdmin(): Promise<AdminSession> {
  const ctx = await request.newContext()
  try {
    const res = await ctx.post(`${API_URL}/api/v1/auth/login`, {
      data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
    })
    expect(res.ok()).toBeTruthy()
    const body = await res.json()
    return { accessToken: body.access_token, refreshToken: body.refresh_token }
  } finally {
    await ctx.dispose()
  }
}
