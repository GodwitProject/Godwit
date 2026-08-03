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
