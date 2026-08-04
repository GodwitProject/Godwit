import { getCurrentUser } from '@/lib/auth'
import { apiCall } from '@/lib/api-client'
import { PageHeader } from '@/components/ui/page-header'
import { EmptyState } from '@/components/ui/empty-state'

interface SpendRow {
  organization_id: string
  team_id: string | null
  user_id: string | null
  total_cost_usd: string
  request_count: number
  tokens_in: number
  tokens_out: number
}

export default async function SpendPage() {
  const user = await getCurrentUser()

  let spendUrl = '/api/v1/spend'
  if (user?.role === 'org_admin' && user.organization_id) {
    spendUrl += `?organization_id=${user.organization_id}`
  }

  let rows: SpendRow[] = []
  try {
    const response = await apiCall(spendUrl)
    if (response.ok) {
      const data = await response.json()
      rows = data.data || []
    }
  } catch (err) {
    console.error('Failed to fetch spend data:', err)
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Spend" description="Aggregated cost by organization, team, and user" />

      {rows.length === 0 ? (
        <EmptyState message="No spend recorded yet" />
      ) : (
        <div className="overflow-x-auto rounded-lg border border-gray-200 bg-white">
          <table className="w-full text-sm">
            <thead className="border-b border-gray-200 bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left font-medium text-gray-700">Organization</th>
                <th className="px-6 py-3 text-left font-medium text-gray-700">Team</th>
                <th className="px-6 py-3 text-left font-medium text-gray-700">User</th>
                <th className="px-6 py-3 text-left font-medium text-gray-700">Requests</th>
                <th className="px-6 py-3 text-left font-medium text-gray-700">Tokens In</th>
                <th className="px-6 py-3 text-left font-medium text-gray-700">Tokens Out</th>
                <th className="px-6 py-3 text-left font-medium text-gray-700">Total Cost</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row, i) => (
                <tr key={i} className="border-b border-gray-100">
                  <td className="px-6 py-4 text-gray-700">{row.organization_id}</td>
                  <td className="px-6 py-4 text-gray-700">{row.team_id ?? '—'}</td>
                  <td className="px-6 py-4 text-gray-700">{row.user_id ?? '—'}</td>
                  <td className="px-6 py-4 text-gray-700">{row.request_count}</td>
                  <td className="px-6 py-4 text-gray-700">{row.tokens_in}</td>
                  <td className="px-6 py-4 text-gray-700">{row.tokens_out}</td>
                  <td className="px-6 py-4 font-medium text-gray-900">
                    ${Number(row.total_cost_usd).toFixed(4)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
