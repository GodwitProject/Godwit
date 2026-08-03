'use client'

import { useUser } from '@/lib/hooks'
import { logoutAction } from '@/lib/logout'

export function TopBar() {
  const { user, loading } = useUser()

  if (loading) {
    return (
      <div className="border-b border-gray-200 bg-white px-6 py-4">
        <p className="text-sm text-gray-600">Loading...</p>
      </div>
    )
  }

  return (
    <div className="border-b border-gray-200 bg-white px-6 py-4 flex justify-between items-center">
      <div></div>

      <div className="flex items-center space-x-4">
        {user && (
          <>
            <div className="text-right">
              <p className="text-sm font-medium text-gray-900">{user.email}</p>
              <p className="text-xs text-gray-600 capitalize">{user.role.replace('_', ' ')}</p>
            </div>

            <button
              onClick={() => logoutAction()}
              className="text-sm text-gray-600 hover:text-gray-900"
            >
              Logout
            </button>
          </>
        )}
      </div>
    </div>
  )
}
