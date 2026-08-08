import { apiFetch } from './http';

export interface ApiModel {
  id: string;
  public_id: string;
  provider: string;
  provider_model_id: string;
  capabilities: string[];
  pricing: unknown;
  created_at: string;
}

const API_BASE = '/api/v1';

function parseModel(raw: {
  id: string;
  public_id: string;
  provider?: string;
  provider_profile_id?: string;
  provider_model_id: string;
  capabilities?: string[] | string;
  pricing?: unknown;
  created_at?: string;
}): ApiModel {
  return {
    id: raw.id,
    public_id: raw.public_id,
    provider: raw.provider ?? '',
    provider_model_id: raw.provider_model_id,
    capabilities: Array.isArray(raw.capabilities)
      ? raw.capabilities
      : typeof raw.capabilities === 'string'
        ? (raw.capabilities || '').split(',').map((s) => s.trim()).filter(Boolean)
        : [],
    pricing: raw.pricing ?? null,
    created_at: raw.created_at ?? '',
  };
}

export async function fetchModels(): Promise<ApiModel[]> {
  const res = await apiFetch(`${API_BASE}/models`);
  if (!res.ok) throw new Error('Failed to fetch models');
  const data = await res.json();
  const rawItems = Array.isArray(data?.data) ? data.data : [];
  return rawItems.map(parseModel);
}

export interface CreateModelRequest {
  public_id: string;
  provider: string;
  provider_profile_id: string;
  provider_model_id: string;
  capabilities: string;
  pricing: {
    input_price_per_million: number;
    output_price_per_million: number;
  };
}

export async function createModel(req: CreateModelRequest): Promise<ApiModel> {
  const res = await apiFetch(`${API_BASE}/models`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error('Failed to create model');
  const data = await res.json();
  return parseModel(data.data);
}
