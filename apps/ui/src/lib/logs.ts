export interface RequestLog {
  id: string;
  timestamp: string;
  requestId: string;
  model: string;
  provider: string;
  status: number;
  tokensIn: number;
  tokensOut: number;
  cost: number;
  latencyMs: number;
  apiKeyPrefix: string;
  requestBody: unknown;
  responseBody: unknown;
  finishReason: string | null;
  piiDetected: boolean;
  moderationStatus: 'not_checked' | 'allowed' | 'blocked';
  fallbackUsed: boolean;
  timeline: Array<{ time: string; event: string }>;
}

export interface LogFilters {
  search?: string;
  model?: string;
  status?: number | string;
  dateFrom?: string;
  dateTo?: string;
}

export interface LogsQuery {
  page?: number;
  pageSize?: number;
  filters?: LogFilters;
}

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

function buildQuery(query: LogsQuery): string {
  const params = new URLSearchParams();
  if (query.page != null) params.set('page', String(query.page));
  if (query.pageSize != null) params.set('pageSize', String(query.pageSize));
  const f = query.filters;
  if (f?.search) params.set('search', f.search);
  if (f?.model) params.set('model', f.model);
  if (f?.status != null && f.status !== '') params.set('status', String(f.status));
  if (f?.dateFrom) params.set('dateFrom', f.dateFrom);
  if (f?.dateTo) params.set('dateTo', f.dateTo);
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

export async function fetchLogs(query: LogsQuery = {}): Promise<RequestLog[]> {
  const res = await fetch(`${API_BASE}/logs${buildQuery(query)}`);
  if (!res.ok) throw new Error('Failed to fetch logs');
  const data = await res.json();
  return Array.isArray(data) ? data : data.logs ?? [];
}

export async function fetchLog(id: string): Promise<RequestLog> {
  const res = await fetch(`${API_BASE}/logs/${id}`);
  if (!res.ok) throw new Error(`Failed to fetch log ${id}`);
  return res.json();
}
