'use client'

import { useState } from 'react'
import { PageHeader } from '@/components/ui/page-header'
import { changePassword } from './actions'

export default function SettingsPage() {
  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setMessage('')
    setError('')

    if (newPassword !== confirmPassword) {
      setError('New passwords do not match')
      return
    }

    setLoading(true)

    try {
      const result = await changePassword(currentPassword, newPassword)
      if (result.success) {
        setMessage('Password changed successfully')
        setCurrentPassword('')
        setNewPassword('')
        setConfirmPassword('')
      } else {
        setError(result.error || 'Failed to change password')
      }
    } catch (err) {
      setError('An unexpected error occurred')
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Settings" description="Manage your account" />

      <div className="rounded-lg bg-white p-6 shadow">
        <h2 className="text-xl font-bold text-gray-900">Change password</h2>

        {message && <div className="mt-4 rounded bg-green-100 p-3 text-green-700">{message}</div>}
        {error && <div className="mt-4 rounded bg-red-100 p-3 text-red-700">{error}</div>}

        <form onSubmit={handleSubmit} className="mt-6 space-y-4 max-w-md">
          <div>
            <label htmlFor="current_password" className="block text-sm font-medium text-gray-700">
              Current password
            </label>
            <input
              id="current_password"
              type="password"
              required
              value={currentPassword}
              onChange={(e) => setCurrentPassword(e.target.value)}
              className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              disabled={loading}
            />
          </div>
          <div>
            <label htmlFor="new_password" className="block text-sm font-medium text-gray-700">
              New password
            </label>
            <input
              id="new_password"
              type="password"
              required
              minLength={8}
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              disabled={loading}
            />
          </div>
          <div>
            <label htmlFor="confirm_password" className="block text-sm font-medium text-gray-700">
              Confirm new password
            </label>
            <input
              id="confirm_password"
              type="password"
              required
              minLength={8}
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              className="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              disabled={loading}
            />
          </div>
          <button
            type="submit"
            disabled={loading}
            className="rounded bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Changing...' : 'Change password'}
          </button>
        </form>
      </div>
    </div>
  )
}
