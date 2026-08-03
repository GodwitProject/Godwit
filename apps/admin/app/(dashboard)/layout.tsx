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
