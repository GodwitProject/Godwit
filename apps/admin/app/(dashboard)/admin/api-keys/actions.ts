'use server'

import { apiCall } from '@/lib/api-client'
import { ApiKey, Model } from '@/lib/types'

export async function listApiKeys(): Promise<ApiKey[]> {
  const response = await apiCall('/api/v1/api-keys')
  if (!response.ok) return []
  const data = await response.json()
  return data.data || []
}

export async function getApiKey(id: string): Promise<ApiKey | null> {
  const response = await apiCall(`/api/v1/api-keys/${id}`)
  if (!response.ok) return null
  const data = await response.json()
  return data.data
}

export async function listModels(): Promise<Model[]> {
  const response = await apiCall('/api/v1/models')
  if (!response.ok) return []
  const data = await response.json()
  return data.data || []
}

export async function createApiKey(
  name: string,
  scopesCsv: string,
  allowedModels: string[]
): Promise<{ success: boolean; apiKey?: string; name?: string; error?: string }> {
  try {
    const scopes = scopesCsv
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0)

    const response = await apiCall('/api/v1/api-keys', {
      method: 'POST',
      body: JSON.stringify({ name, scopes, allowed_models: allowedModels }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create API key' }
    }

    const data = await response.json()
    return { success: true, apiKey: data.key, name: data.name }
  } catch (err) {
    console.error('Create API key error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function deleteApiKey(id: string): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/api-keys/${id}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to delete API key' }
    }

    return { success: true }
  } catch (err) {
    console.error('Delete API key error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
