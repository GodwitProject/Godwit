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

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}

export async function fetchProviders(): Promise<Provider[]> {
  const data = await getJson<{ data: Provider[] }>('/provider-profiles');
  return data.data || [];
}
