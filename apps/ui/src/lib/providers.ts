import { apiFetch } from './http';

export interface Provider {
  id: string;
  name: string;
  protocol: string;
  base_url: string;
  allow_wildcard: boolean;
  enabled: boolean;
  has_credentials: boolean;
  created_at: string;
}

const API_BASE = '/api/v1';

async function getJson<T>(path: string): Promise<T> {
  const res = await apiFetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}

export async function fetchProviders(): Promise<Provider[]> {
  const data = await getJson<{ data: Provider[] }>('/provider-profiles');
  return data.data || [];
}

export async function setProviderEnabled(id: string, enabled: boolean): Promise<Provider> {
  const res = await apiFetch(`${API_BASE}/provider-profiles/${id}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ enabled }),
  });
  if (!res.ok) throw new Error(`Failed to PATCH /provider-profiles/${id}`);
  return res.json();
}
