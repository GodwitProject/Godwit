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
  { name: 'Settings', href: '/admin/settings' },
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
