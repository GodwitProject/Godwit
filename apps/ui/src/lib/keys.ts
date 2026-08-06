export interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  owner: string;
  scopes: string[];
  allowedModels: string[];
  budget: number | null;
  rateLimitRpm: number | null;
  rateLimitTpm: number | null;
  expiresAt: string | null;
  spend30d: number;
  requests24h: number;
  lastUsedAt: string | null;
  status: 'active' | 'revoked';
  createdAt: string;
}

export interface CreateKeyRequest {
  name: string;
  owner: string;
  scopes: string[];
  allowedModels: string[];
  budget?: number | null;
  rateLimitRpm?: number | null;
  rateLimitTpm?: number | null;
  expiresAt?: string | null;
}

export type UpdateKeyRequest = Partial<CreateKeyRequest>;

export interface CreatedKey {
  key: ApiKey;
  fullKey: string;
}

export interface KeyUsagePoint {
  day: string;
  spend: number;
}

export interface KeyUsage {
  totalSpend: number;
  totalRequests: number;
  timeseries: KeyUsagePoint[];
}

export interface KeyLog {
  id: string;
  timestamp: string;
  model: string;
  status: number;
  tokensIn: number;
  tokensOut: number;
  cost: number;
}

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}

async function sendJson<T>(path: string, method: string, body: unknown): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Failed to ${method} ${path}`);
  return res.json();
}

export async function fetchKeys(): Promise<ApiKey[]> {
  return getJson<ApiKey[]>('/keys');
}

export async function createKey(req: CreateKeyRequest): Promise<CreatedKey> {
  return sendJson<CreatedKey>('/keys', 'POST', req);
}

export async function updateKey(id: string, req: UpdateKeyRequest): Promise<ApiKey> {
  return sendJson<ApiKey>(`/keys/${id}`, 'PATCH', req);
}

export async function deleteKey(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/keys/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error(`Failed to DELETE /keys/${id}`);
}

export async function revokeKey(id: string): Promise<ApiKey> {
  return sendJson<ApiKey>(`/keys/${id}/revoke`, 'POST', {});
}

export async function fetchKeyUsage(id: string): Promise<KeyUsage> {
  return getJson<KeyUsage>(`/keys/${id}/usage`);
}

export async function fetchKeyLogs(id: string): Promise<KeyLog[]> {
  return getJson<KeyLog[]>(`/keys/${id}/logs`);
}
