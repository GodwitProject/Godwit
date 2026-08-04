'use server'

import { apiCall } from '@/lib/api-client'

interface Model {
  id: string
  public_id: string
  provider: string
  provider_profile_id: string
  provider_model_id: string
  capabilities: string[]
  pricing: Record<string, unknown>
  config: Record<string, unknown>
  created_at: string
}

export async function listModels(): Promise<Model[]> {
  const response = await apiCall('/api/v1/models')
  if (!response.ok) return []
  const data = await response.json()
  return data.data || []
}

export async function getModel(id: string): Promise<Model | null> {
  const response = await apiCall(`/api/v1/models/${id}`)
  if (!response.ok) return null
  const data = await response.json()
  return data.data
}

export async function createModel(
  publicId: string,
  provider: string,
  providerProfileId: string,
  providerModelId: string,
  capabilitiesCsv: string
): Promise<{ success: boolean; model?: Model; error?: string }> {
  try {
    const response = await apiCall('/api/v1/models', {
      method: 'POST',
      body: JSON.stringify({
        public_id: publicId,
        provider,
        provider_profile_id: providerProfileId,
        provider_model_id: providerModelId,
        capabilities: capabilitiesCsv,
      }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to create model' }
    }

    const data = await response.json()
    return { success: true, model: data.data }
  } catch (err) {
    console.error('Create model error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function updateModel(
  id: string,
  publicId: string,
  capabilitiesCsv: string
): Promise<{ success: boolean; model?: Model; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/models/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({
        public_id: publicId,
        capabilities: capabilitiesCsv,
      }),
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to update model' }
    }

    const data = await response.json()
    return { success: true, model: data.data }
  } catch (err) {
    console.error('Update model error:', err)
    return { success: false, error: 'An error occurred' }
  }
}

export async function deleteModel(
  id: string
): Promise<{ success: boolean; error?: string }> {
  try {
    const response = await apiCall(`/api/v1/models/${id}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      return { success: false, error: 'Failed to delete model' }
    }

    return { success: true }
  } catch (err) {
    console.error('Delete model error:', err)
    return { success: false, error: 'An error occurred' }
  }
}
