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
