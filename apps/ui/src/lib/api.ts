const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export async function fetchMetrics() {
  const res = await fetch(`${API_BASE}/metrics/summary`);
  if (!res.ok) throw new Error('Failed to fetch metrics');
  return res.json();
}

export async function fetchLatency() {
  const res = await fetch(`${API_BASE}/metrics/latency`);
  if (!res.ok) throw new Error('Failed to fetch latency');
  return res.json();
}

export async function fetchTokens() {
  const res = await fetch(`${API_BASE}/metrics/tokens`);
  if (!res.ok) throw new Error('Failed to fetch tokens');
  return res.json();
}

export async function fetchRecentLogs(limit = 10) {
  const res = await fetch(`${API_BASE}/logs/recent?limit=${limit}`);
  if (!res.ok) throw new Error('Failed to fetch logs');
  return res.json();
}
