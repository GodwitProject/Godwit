import { useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Button } from '../ui/Button';
import type { RequestLog } from '../../lib/logs';

export interface LogsTableProps {
  logs: RequestLog[];
  onSelect: (log: RequestLog) => void;
  hasMore: boolean;
  onLoadMore: () => void;
  loadingMore?: boolean;
}

type SortKey = 'created_at' | 'cost_usd' | 'duration_ms';

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
}

function formatLatency(durationMs: number | null): string {
  return durationMs != null ? `${durationMs}ms` : '—';
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  } catch {
    return iso;
  }
}

export function LogsTable({ logs, onSelect, hasMore, onLoadMore, loadingMore }: LogsTableProps) {
  const [sortKey, setSortKey] = useState<SortKey>('created_at');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('desc');

  function handleSort(key: SortKey) {
    if (key === sortKey) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('desc');
    }
  }

  // NOTE: `logs` is the accumulated "load more" set from the parent, so sorting
  // here applies to the full currently-loaded list (client-side only).
  const sorted = [...logs].sort((a, b) => {
    const av = a[sortKey];
    const bv = b[sortKey];
    if (typeof av === 'string' && typeof bv === 'string') {
      const cmp = av.localeCompare(bv);
      return sortDir === 'asc' ? cmp : -cmp;
    }
    const an = av == null ? Number.NEGATIVE_INFINITY : (av as number);
    const bn = bv == null ? Number.NEGATIVE_INFINITY : (bv as number);
    const cmp = an - bn;
    return sortDir === 'asc' ? cmp : -cmp;
  });

  const sortArrow = (key: SortKey) => (sortKey === key ? (sortDir === 'asc' ? ' ↑' : ' ↓') : '');

  return (
    <Card className="overflow-hidden">
      {sorted.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="material-symbols-outlined text-4xl text-on-surface-variant mb-2">receipt_long</span>
          <p className="text-body-base text-on-surface-variant">No logs found.</p>
        </div>
      ) : (
        <>
          <Table>
            <TableHead>
              <TableRow>
                <TableHeadCell className="cursor-pointer select-none" onClick={() => handleSort('created_at')}>
                  Timestamp{sortArrow('created_at')}
                </TableHeadCell>
                <TableHeadCell>Log ID</TableHeadCell>
                <TableHeadCell>Model</TableHeadCell>
                <TableHeadCell>Provider</TableHeadCell>
                <TableHeadCell className="cursor-pointer select-none" onClick={() => handleSort('cost_usd')}>
                  Cost{sortArrow('cost_usd')}
                </TableHeadCell>
                <TableHeadCell className="text-right cursor-pointer select-none" onClick={() => handleSort('duration_ms')}>
                  Latency{sortArrow('duration_ms')}
                </TableHeadCell>
                <TableHeadCell />
              </TableRow>
            </TableHead>
            <TableBody>
              {sorted.map((log) => (
                <TableRow key={log.id}>
                  <TableCell className="text-on-surface-variant whitespace-nowrap">{formatDate(log.created_at)}</TableCell>
                  <TableCell>
                    <button
                      type="button"
                      className="font-mono text-code-sm text-primary hover:underline cursor-pointer"
                      onClick={() => onSelect(log)}
                    >
                      {log.id}
                    </button>
                  </TableCell>
                  <TableCell className="font-mono text-code-sm">{log.model}</TableCell>
                  <TableCell className="text-on-surface-variant">{log.provider}</TableCell>
                  <TableCell className="font-mono text-code-sm">{formatCost(log.cost_usd)}</TableCell>
                  <TableCell className="text-right font-mono text-code-sm">{formatLatency(log.duration_ms)}</TableCell>
                  <TableCell>
                    <button
                      type="button"
                      aria-label={`Open details for ${log.id}`}
                      className="material-symbols-outlined p-1 rounded-full hover:bg-surface-container-high text-on-surface-variant"
                      onClick={() => onSelect(log)}
                    >
                      open_in_full
                    </button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          <div className="flex flex-col md:flex-row items-center justify-between gap-3 p-container-padding border-t hairline-border">
            <span className="text-body-base text-on-surface-variant">Showing {sorted.length} logs</span>
            {hasMore && (
              <Button variant="secondary" size="sm" disabled={loadingMore} onClick={onLoadMore}>
                {loadingMore ? 'Loading...' : 'Load more'}
              </Button>
            )}
          </div>
        </>
      )}
    </Card>
  );
}
