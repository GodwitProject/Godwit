import type { MetricsUpdate } from './websocket';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export interface AdminStats {
  organizations: number;
  teams: number;
  users: number;
  apiKeys: number;
}

export interface SpendPoint {
  date: string;
  cost: number;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`Failed to fetch ${path}`);
  return res.json();
}

export function fetchStats(): Promise<AdminStats> {
  return getJson<AdminStats>('/admin/stats');
}

export async function fetchSpend(days = 30): Promise<SpendPoint[]> {
  const data = await getJson<{ data: Array<{ date: string; cost: string | number }> }>(`/spend?days=${days}`);
  return (data.data || []).map((point) => ({
    date: point.date,
    cost: typeof point.cost === 'string' ? parseFloat(point.cost) : point.cost,
  }));
}

export function parsePrometheusMetrics(text: string): MetricsUpdate {
  const getValue = (name: string): number => {
    const re = new RegExp(`^${name}(?:\\{[^}]*\\})?\\s+([0-9.eE+-]+)$`, 'gm');
    let sum = 0;
    let matched = false;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      matched = true;
      const parsed = parseFloat(m[1]);
      if (Number.isFinite(parsed)) sum += parsed;
    }
    return matched ? sum : 0;
  };

  return {
    requestsTotal: getValue('godwit_requests_total'),
    tokensTotal: getValue('godwit_tokens_total'),
    costUsdTotal: getValue('godwit_cost_usd_total'),
    activeRequests: getValue('godwit_active_requests'),
    timestamp: new Date().toISOString(),
  };
}

export async function fetchPrometheusMetrics(): Promise<MetricsUpdate> {
  const res = await fetch('/metrics');
  if (!res.ok) throw new Error('Failed to fetch metrics');
  return parsePrometheusMetrics(await res.text());
}
