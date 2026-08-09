import { describe, it, expect } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'

import contract from '../../../contract/routes.json'

interface ContractEntry {
  id: string
  method: string
  path: string
  frontend: { lib: string; fn: string } | null
  scope: string
}

/**
 * Every backend endpoint the legacy admin console calls must be declared in
 * `contract/routes.json` and match the declared method+path. We assert the server
 * actions and dashboard pages statically because they rely on server-only imports
 * (next/headers, next/navigation) that do not run under jsdom.
 */

type Call = { method: string; path: string; file: string; fn: string }

const ADMIN_ROOT = path.resolve(__dirname, '..')

function stripQuery(p: string): string {
  const i = p.indexOf('?')
  return i >= 0 ? p.slice(0, i) : p
}

/** Convert a concrete path (with an id) back to the contract template (`{id}`). */
function template(p: string): string {
  return p.replace(/[0-9a-fA-F-]{8,}/g, '{id}')
}

/** The exact backend calls made by the admin console (method + path template). */
const EXPECTED_CALLS: Call[] = [
  // auth
  { file: 'app/(auth)/login/actions.ts', fn: 'loginWithPassword', method: 'POST', path: '/api/v1/auth/login' },
  { file: 'app/(auth)/forgot-password/actions.ts', fn: 'requestPasswordReset', method: 'POST', path: '/api/v1/auth/forgot-password' },
  { file: 'app/(auth)/reset-password/actions.ts', fn: 'performPasswordReset', method: 'POST', path: '/api/v1/auth/reset-password' },
  { file: 'app/(dashboard)/admin/users/actions.ts', fn: 'resetUserPassword', method: 'POST', path: '/api/v1/auth/admin/reset-password' },
  { file: 'app/(auth)/login/actions.ts', fn: 'loginWithSSO', method: 'GET', path: '/api/v1/auth/oidc/{provider}' },
  { file: 'app/(auth)/auth/callback/actions.ts', fn: 'exchangeOIDCCode', method: 'GET', path: '/api/v1/auth/oidc/{provider}/callback' },
  { file: 'app/(dashboard)/admin/api-keys/actions.ts', fn: 'listApiKeys', method: 'GET', path: '/api/v1/api-keys' },
  { file: 'app/(dashboard)/admin/api-keys/actions.ts', fn: 'getApiKey', method: 'GET', path: '/api/v1/api-keys/{id}' },
  { file: 'app/(dashboard)/admin/api-keys/actions.ts', fn: 'listModels', method: 'GET', path: '/api/v1/models' },
  { file: 'app/(dashboard)/admin/api-keys/actions.ts', fn: 'createApiKey', method: 'POST', path: '/api/v1/api-keys' },
  { file: 'app/(dashboard)/admin/api-keys/actions.ts', fn: 'deleteApiKey', method: 'DELETE', path: '/api/v1/api-keys/{id}' },
  { file: 'app/(dashboard)/admin/models/actions.ts', fn: 'listModels', method: 'GET', path: '/api/v1/models' },
  { file: 'app/(dashboard)/admin/models/actions.ts', fn: 'getModel', method: 'GET', path: '/api/v1/models/{id}' },
  { file: 'app/(dashboard)/admin/models/actions.ts', fn: 'createModel', method: 'POST', path: '/api/v1/models' },
  { file: 'app/(dashboard)/admin/models/actions.ts', fn: 'updateModel', method: 'PATCH', path: '/api/v1/models/{id}' },
  { file: 'app/(dashboard)/admin/models/actions.ts', fn: 'deleteModel', method: 'DELETE', path: '/api/v1/models/{id}' },
  { file: 'app/(dashboard)/admin/organizations/actions.ts', fn: 'listOrganizations', method: 'GET', path: '/api/v1/organizations' },
  { file: 'app/(dashboard)/admin/organizations/actions.ts', fn: 'getOrganization', method: 'GET', path: '/api/v1/organizations/{id}' },
  { file: 'app/(dashboard)/admin/organizations/actions.ts', fn: 'createOrganization', method: 'POST', path: '/api/v1/organizations' },
  { file: 'app/(dashboard)/admin/organizations/actions.ts', fn: 'updateOrganization', method: 'PATCH', path: '/api/v1/organizations/{id}' },
  { file: 'app/(dashboard)/admin/organizations/actions.ts', fn: 'deleteOrganization', method: 'DELETE', path: '/api/v1/organizations/{id}' },
  { file: 'app/(dashboard)/admin/teams/actions.ts', fn: 'listTeams', method: 'GET', path: '/api/v1/teams' },
  { file: 'app/(dashboard)/admin/teams/actions.ts', fn: 'getTeam', method: 'GET', path: '/api/v1/teams/{id}' },
  { file: 'app/(dashboard)/admin/teams/actions.ts', fn: 'createTeam', method: 'POST', path: '/api/v1/teams' },
  { file: 'app/(dashboard)/admin/teams/actions.ts', fn: 'updateTeam', method: 'PATCH', path: '/api/v1/teams/{id}' },
  { file: 'app/(dashboard)/admin/teams/actions.ts', fn: 'deleteTeam', method: 'DELETE', path: '/api/v1/teams/{id}' },
  { file: 'app/(dashboard)/admin/users/actions.ts', fn: 'listUsers', method: 'GET', path: '/api/v1/users' },
  { file: 'app/(dashboard)/admin/users/actions.ts', fn: 'getUser', method: 'GET', path: '/api/v1/users/{id}' },
  { file: 'app/(dashboard)/admin/users/actions.ts', fn: 'createUser', method: 'POST', path: '/api/v1/users' },
  { file: 'app/(dashboard)/admin/users/actions.ts', fn: 'updateUser', method: 'PATCH', path: '/api/v1/users/{id}' },
  { file: 'app/(dashboard)/admin/users/actions.ts', fn: 'deleteUser', method: 'DELETE', path: '/api/v1/users/{id}' },
  { file: 'app/(dashboard)/admin/page.tsx', fn: 'DashboardPage', method: 'GET', path: '/api/v1/admin/stats' },
  { file: 'app/(dashboard)/admin/page.tsx', fn: 'DashboardPage', method: 'GET', path: '/api/v1/admin/recent-activity' },
  { file: 'app/(dashboard)/admin/page.tsx', fn: 'DashboardPage', method: 'GET', path: '/api/v1/spend' },
  { file: 'app/(dashboard)/admin/spend/page.tsx', fn: 'SpendPage', method: 'GET', path: '/api/v1/spend' },
  // api-client auto-refresh
  { file: 'lib/api-client.ts', fn: 'apiCall', method: 'POST', path: '/api/v1/auth/refresh' },
  // lib/auth user lookup
  { file: 'lib/auth.ts', fn: 'getCurrentUser', method: 'GET', path: '/api/v1/users/{id}' },
]

describe('admin route contract — every admin backend call is declared', () => {
  const entries = contract as ContractEntry[]

  it('every admin backend call matches a contract route (method + path)', () => {
    for (const call of EXPECTED_CALLS) {
      const matched = entries.some(
        (e) => e.method === call.method && stripQuery(e.path) === call.path
      )
      expect(
        matched,
        `${call.file}:${call.fn} — ${call.method} ${call.path} is not declared in contract/routes.json`
      ).toBe(true)
    }
  })

  it('the declared admin frontend pointers resolve to real files', () => {
    const adminEntries = entries.filter(
      (e) => e.scope === 'admin' && e.frontend
    )
    expect(adminEntries.length).toBeGreaterThan(0)
    for (const e of adminEntries) {
      const file = path.resolve(ADMIN_ROOT, e.frontend!.lib.replace('apps/admin/', ''))
      expect(
        fs.existsSync(file),
        `${e.id}: expected admin frontend file ${file} to exist`
      ).toBe(true)
    }
  })
})
