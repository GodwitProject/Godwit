import { test, expect } from '@playwright/test'
import { loginAsAdmin } from './helpers'

test.describe('Dashboard', () => {
  test.beforeEach(async ({ context }) => {
    // Real admin JWT from the backend so the middleware and API calls accept it.
    const { accessToken, refreshToken } = await loginAsAdmin()
    await context.addCookies([
      { name: 'access_token', value: accessToken, domain: 'localhost', path: '/' },
      { name: 'refresh_token', value: refreshToken, domain: 'localhost', path: '/' },
    ])
  })

  test('display dashboard home with stats', async ({ page }) => {
    await page.goto('/admin')

    // Should show stats
    await expect(page.locator('text=Organizations').first()).toBeVisible()
    await expect(page.locator('text=Teams').first()).toBeVisible()
    await expect(page.locator('text=Users').first()).toBeVisible()
    await expect(page.locator('text=API Keys').first()).toBeVisible()
  })

  test('navigate to organizations page', async ({ page }) => {
    await page.goto('/admin')
    await page.click('a:has-text("Organizations")')

    await expect(page).toHaveURL('/admin/organizations')
    await expect(page.getByRole('heading', { level: 1, name: 'Organizations' })).toBeVisible()
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
