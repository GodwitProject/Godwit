import { apiFetch } from './http';

export interface ApiKey {
  id: string;
  user_id: string | null;
  team_id: string | null;
  organization_id: string | null;
  name: string;
  key_prefix: string;
  scopes: string[];
  allowed_models: string[];
  budget_limit_usd: number | null;
  budget_spent_usd: number | null;
  rate_limit_requests_per_minute: number | null;
  rate_limit_tokens_per_minute: number | null;
  expires_at: string | null;
  disabled: boolean;
  created_at: string;
}

export interface CreateKeyRequest {
  name: string;
  scopes: string[];
  allowed_models: string[];
  rate_limit_requests_per_minute?: number | null;
  rate_limit_tokens_per_minute?: number | null;
}

export interface CreatedKey {
  id: string;
  key: string;
  name: string;
}

export interface ApiKeyActionResponse {
  data: ApiKey;
}

const API_BASE = ''; // same-origin via next rewrites

function toNullableNumber(value: string | number | null | undefined): number | null {
  if (value == null || value === '') return null;
  const n = typeof value === 'string' ? parseFloat(value) : value;
  return Number.isFinite(n) ? n : null;
}

function parseApiKey(raw: {
  id: string;
  user_id?: string | null;
  team_id?: string | null;
  organization_id?: string | null;
  name: string;
  key_prefix?: string;
  scopes?: string[];
  allowed_models?: string[];
  budget_limit_usd?: string | number | null;
  budget_spent_usd?: string | number | null;
  rate_limit_requests_per_minute?: string | number | null;
  rate_limit_tokens_per_minute?: string | number | null;
  expires_at?: string | null;
  disabled?: boolean;
  created_at?: string;
}): ApiKey {
  return {
    id: raw.id,
    user_id: raw.user_id ?? null,
    team_id: raw.team_id ?? null,
    organization_id: raw.organization_id ?? null,
    name: raw.name,
    key_prefix: raw.key_prefix ?? '',
    scopes: raw.scopes ?? [],
    allowed_models: raw.allowed_models ?? [],
    budget_limit_usd: toNullableNumber(raw.budget_limit_usd),
    budget_spent_usd: toNullableNumber(raw.budget_spent_usd),
    rate_limit_requests_per_minute: toNullableNumber(raw.rate_limit_requests_per_minute),
    rate_limit_tokens_per_minute: toNullableNumber(raw.rate_limit_tokens_per_minute),
    expires_at: raw.expires_at ?? null,
    disabled: !!raw.disabled,
    created_at: raw.created_at ?? '',
  };
}

async function getJson<T>(path: string): Promise<T> {
  const res = await apiFetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}

async function sendJson<T>(path: string, method: string, body?: unknown): Promise<T> {
  const res = await apiFetch(`${API_BASE}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body == null ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Failed to ${method} ${path}`);
  return res.json();
}

export async function fetchKeys(): Promise<ApiKey[]> {
  const data = await getJson<{ data: RawApiKey[] }>('/api-keys');
  return (data.data || []).map(parseApiKey);
}

export async function createKey(req: CreateKeyRequest): Promise<CreatedKey> {
  return sendJson<CreatedKey>('/api-keys', 'POST', req);
}

export async function blockKey(id: string): Promise<ApiKey> {
  const data = await sendJson<ApiKeyActionResponse>(`/api-keys/${id}/block`, 'POST');
  return parseApiKey(data.data);
}

export async function unblockKey(id: string): Promise<ApiKey> {
  const data = await sendJson<ApiKeyActionResponse>(`/api-keys/${id}/unblock`, 'POST');
  return parseApiKey(data.data);
}

export async function deleteKey(id: string): Promise<void> {
  const res = await apiFetch(`${API_BASE}/api-keys/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error(`Failed to DELETE /api-keys/${id}`);
}

type RawApiKey = Parameters<typeof parseApiKey>[0];
