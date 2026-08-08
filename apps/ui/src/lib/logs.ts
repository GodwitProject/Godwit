import { apiFetch } from './http';

export interface RequestLog {
  id: string;
  api_key_id: string | null;
  model: string;
  provider: string;
  capability: string;
  tokens_in: number | null;
  tokens_out: number | null;
  duration_ms: number | null;
  streamed: boolean;
  cost_usd: number;
  status: string;
  created_at: string;
}

export interface LogFilters {
  model?: string;
  api_key_id?: string;
  from?: string;
  to?: string;
}

export interface LogsQuery {
  limit?: number;
  offset?: number;
  filters?: LogFilters;
}

export interface LogsPage {
  items: RequestLog[];
  offset: number;
  limit: number;
}

const API_BASE = ''; // same-origin via next rewrites

function buildQuery(query: LogsQuery): string {
  const params = new URLSearchParams();
  if (query.limit != null) params.set('limit', String(query.limit));
  if (query.offset != null) params.set('offset', String(query.offset));
  const f = query.filters;
  if (f?.model) params.set('model', f.model);
  if (f?.api_key_id) params.set('api_key_id', f.api_key_id);
  if (f?.from) params.set('from', f.from);
  if (f?.to) params.set('to', f.to);
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

interface RawSpendLog {
  id: string;
  api_key_id?: string | null;
  model?: string;
  provider?: string;
  capability?: string;
  tokens_in?: number | string | null;
  tokens_out?: number | string | null;
  duration_ms?: number | string | null;
  streamed?: boolean;
  cost_usd?: string | number | null;
  status?: string;
  created_at?: string;
}

function toNullableInt(v: number | string | null | undefined): number | null {
  if (v == null) return null;
  const n = typeof v === 'string' ? Number(v) : v;
  return Number.isFinite(n) ? n : null;
}

function parseLog(raw: RawSpendLog): RequestLog {
  const duration = typeof raw.duration_ms === 'string' ? Number(raw.duration_ms) : raw.duration_ms;
  const cost = typeof raw.cost_usd === 'string' ? parseFloat(raw.cost_usd) : raw.cost_usd;
  return {
    id: raw.id,
    api_key_id: raw.api_key_id ?? null,
    model: raw.model ?? '',
    provider: raw.provider ?? '',
    capability: raw.capability ?? '',
    tokens_in: toNullableInt(raw.tokens_in),
    tokens_out: toNullableInt(raw.tokens_out),
    duration_ms: duration != null && Number.isFinite(duration) ? duration : null,
    streamed: !!raw.streamed,
    cost_usd: cost != null && Number.isFinite(cost) ? cost : 0,
    status: raw.status ?? '',
    created_at: raw.created_at ?? '',
  };
}

export async function fetchLogs(query: LogsQuery = {}): Promise<LogsPage> {
  const res = await apiFetch(`${API_BASE}/spend/logs${buildQuery(query)}`);
  if (!res.ok) throw new Error('Failed to fetch logs');
  const data = await res.json();
  const rawItems: RawSpendLog[] = Array.isArray(data?.data) ? data.data : [];
  return {
    items: rawItems.map(parseLog),
    offset: typeof data?.offset === 'number' ? data.offset : query.offset ?? 0,
    limit: typeof data?.limit === 'number' ? data.limit : query.limit ?? 0,
  };
}
