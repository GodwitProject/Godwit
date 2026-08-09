import { test, expect } from '@playwright/test'
import { loginAsAdmin } from './helpers'

async function setAuthCookies(context: import('@playwright/test').BrowserContext) {
  const { accessToken, refreshToken } = await loginAsAdmin()
  await context.addCookies([
    { name: 'access_token', value: accessToken, domain: 'localhost', path: '/' },
    { name: 'refresh_token', value: refreshToken, domain: 'localhost', path: '/' },
  ])
}

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
    await expect(page.getByRole('heading', { level: 1, name: 'Dashboard' })).toBeVisible()
  })

  test('redirect to login when not authenticated', async ({ page }) => {
    await page.goto('/admin')
    await expect(page).toHaveURL('/login')
  })

  test('redirect to dashboard when already logged in', async ({ page, context }) => {
    // Real admin JWT from the backend, so the middleware accepts it as authenticated.
    await setAuthCookies(context)

    await page.goto('/login')
    await expect(page).toHaveURL('/admin')
  })

  test('logout clears cookies and redirects to login', async ({ page, context }) => {
    await setAuthCookies(context)

    await page.goto('/admin')
    await page.click('button:has-text("Logout")')

    // Should redirect to login
    await expect(page).toHaveURL('/login')
  })
})
