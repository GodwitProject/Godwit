import { useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import type { RequestLog } from '../../lib/logs';

export interface LogsTableProps {
  logs: RequestLog[];
  onSelect: (log: RequestLog) => void;
  total: number;
  page: number;
  pageSize: number;
  onPageChange: (page: number) => void;
}

type SortKey = 'timestamp' | 'latencyMs' | 'cost';

function statusVariant(status: number): 'success' | 'warning' | 'error' | 'info' {
  if (status >= 200 && status < 300) return 'success';
  if (status === 429) return 'warning';
  if (status >= 500) return 'error';
  return 'warning';
}

function statusLabel(status: number): string {
  if (status >= 200 && status < 300) return 'OK';
  if (status === 429) return 'Ratelimit';
  if (status >= 500) return 'Error';
  return 'Client';
}

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
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

const PAGE_SIZES = [10, 25, 50];

export function LogsTable({ logs, onSelect, total, page, pageSize, onPageChange }: LogsTableProps) {
  const [sortKey, setSortKey] = useState<SortKey>('timestamp');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('desc');

  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  function handleSort(key: SortKey) {
    if (key === sortKey) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('desc');
    }
  }

  const sorted = [...logs].sort((a, b) => {
    const av = a[sortKey];
    const bv = b[sortKey];
    let cmp = 0;
    if (typeof av === 'string' && typeof bv === 'string') {
      cmp = av.localeCompare(bv);
    } else {
      cmp = (av as number) - (bv as number);
    }
    return sortDir === 'asc' ? cmp : -cmp;
  });

  const sortArrow = (key: SortKey) => sortKey === key ? (sortDir === 'asc' ? ' ↑' : ' ↓') : '';

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
                <TableHeadCell className="cursor-pointer select-none" onClick={() => handleSort('timestamp')}>
                  Timestamp{sortArrow('timestamp')}
                </TableHeadCell>
                <TableHeadCell>Request ID</TableHeadCell>
                <TableHeadCell>Model</TableHeadCell>
                <TableHeadCell>Provider</TableHeadCell>
                <TableHeadCell>Status</TableHeadCell>
                <TableHeadCell>Tokens</TableHeadCell>
                <TableHeadCell className="cursor-pointer select-none" onClick={() => handleSort('cost')}>
                  Cost{sortArrow('cost')}
                </TableHeadCell>
                <TableHeadCell className="text-right cursor-pointer select-none" onClick={() => handleSort('latencyMs')}>
                  Latency{sortArrow('latencyMs')}
                </TableHeadCell>
                <TableHeadCell />
              </TableRow>
            </TableHead>
            <TableBody>
              {sorted.map((log) => (
                <TableRow key={log.id}>
                  <TableCell className="text-on-surface-variant whitespace-nowrap">{formatDate(log.timestamp)}</TableCell>
                  <TableCell>
                    <button
                      type="button"
                      className="font-mono text-code-sm text-primary hover:underline cursor-pointer"
                      onClick={() => onSelect(log)}
                    >
                      {log.requestId}
                    </button>
                  </TableCell>
                  <TableCell className="font-mono text-code-sm">{log.model}</TableCell>
                  <TableCell className="text-on-surface-variant">{log.provider}</TableCell>
                  <TableCell>
                    <Badge variant={statusVariant(log.status)}>
                      {log.status} {statusLabel(log.status)}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-mono text-code-sm">
                    {log.tokensIn}/{log.tokensOut}
                  </TableCell>
                  <TableCell className="font-mono text-code-sm">{formatCost(log.cost)}</TableCell>
                  <TableCell className="text-right font-mono text-code-sm">{log.latencyMs}ms</TableCell>
                  <TableCell>
                    <button
                      type="button"
                      aria-label={`Open details for ${log.requestId}`}
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
            <span className="text-body-base text-on-surface-variant">
              Showing {sorted.length} of {total} logs
            </span>
            <div className="flex items-center gap-2">
              <Button variant="secondary" size="sm" disabled={page <= 1} onClick={() => onPageChange(page - 1)}>
                Previous
              </Button>
              <span className="text-body-base text-on-surface-variant px-2">
                {page} / {totalPages}
              </span>
              <Button variant="secondary" size="sm" disabled={page >= totalPages} onClick={() => onPageChange(page + 1)}>
                Next
              </Button>
            </div>
          </div>
        </>
      )}
    </Card>
  );
}
