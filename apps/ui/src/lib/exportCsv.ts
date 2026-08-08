import type { RequestLog } from './logs';

function escapeCell(value: unknown): string {
  const s = value == null ? '' : String(value);
  if (/[",\n\r]/.test(s)) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}

export function logsToCsv(logs: RequestLog[]): string {
  const header = [
    'id',
    'api_key_id',
    'model',
    'provider',
    'capability',
    'tokens_in',
    'tokens_out',
    'duration_ms',
    'cost_usd',
    'status',
    'created_at',
  ];
  const rows = logs.map((l) =>
    [
      l.id,
      l.api_key_id,
      l.model,
      l.provider,
      l.capability,
      l.tokens_in,
      l.tokens_out,
      l.duration_ms,
      l.cost_usd,
      l.status,
      l.created_at,
    ]
      .map(escapeCell)
      .join(',')
  );
  return [header.join(','), ...rows].join('\n');
}

export function downloadCsv(filename: string, csv: string): void {
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
