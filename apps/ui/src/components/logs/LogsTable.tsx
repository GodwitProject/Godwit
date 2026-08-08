import { useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Button } from '../ui/Button';
import { StatusPill } from './StatusPill';
import { useT } from '@/hooks/useT';
import type { RequestLog } from '../../lib/logs';

export interface LogsTableProps {
  logs: RequestLog[];
  onSelect: (log: RequestLog) => void;
  hasMore: boolean;
  onLoadMore: () => void;
  loadingMore?: boolean;
}

type SortKey = 'created_at' | 'model' | 'provider' | 'tokens_in' | 'tokens_out' | 'cost_usd' | 'duration_ms' | 'status';

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
}

function formatLatency(durationMs: number | null): string {
  return durationMs != null ? `${durationMs}ms` : '—';
}

function formatTokens(v: number | null): string {
  return v != null ? String(v) : '—';
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

const SORTABLE: Record<SortKey, boolean> = {
  created_at: true,
  model: true,
  provider: true,
  tokens_in: true,
  tokens_out: true,
  cost_usd: true,
  duration_ms: true,
  status: true,
};

export function LogsTable({ logs, onSelect, hasMore, onLoadMore, loadingMore }: LogsTableProps) {
  const { t } = useT();
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

  const arrow = () => (sortDir === 'asc' ? '▲' : '▼');
  const sortCell = (key: SortKey, label: string, align: 'left' | 'right' = 'left') => (
    <TableHeadCell
      className={`cursor-pointer select-none ${align === 'right' ? 'text-right' : ''}`}
      onClick={() => handleSort(key)}
    >
      {label} {sortKey === key && arrow()}
    </TableHeadCell>
  );

  return (
    <Card className="overflow-hidden">
      {sorted.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="text-3xl text-muted mb-2">📋</span>
          <p className="text-[13px] text-muted">{t('logs.noLogs')}</p>
        </div>
      ) : (
        <>
          <Table>
            <TableHead>
              <TableRow>
                {sortCell('created_at', t('traffic.request'))}
                {sortCell('model', t('recent.model'))}
                {sortCell('provider', t('traffic.provider'))}
                {sortCell('tokens_in', t('traffic.tokensIn'), 'right')}
                {sortCell('tokens_out', t('traffic.tokensOut'), 'right')}
                {sortCell('cost_usd', t('traffic.cost'), 'right')}
                {sortCell('duration_ms', t('logs.table.latency'), 'right')}
                {sortCell('status', t('recent.status'))}
                <TableHeadCell />
              </TableRow>
            </TableHead>
            <TableBody>
              {sorted.map((log) => (
                <TableRow key={log.id} className="cursor-pointer" onClick={() => onSelect(log)}>
                  <TableCell className="whitespace-nowrap">
                    <div className="font-mono text-[11.5px] text-muted">{log.id}</div>
                    <div className="text-[11px] text-muted font-mono">{formatDate(log.created_at)}</div>
                  </TableCell>
                  <TableCell className="font-mono text-[11.5px] font-medium">{log.model}</TableCell>
                  <TableCell><span className="tag">{log.provider || log.capability}</span></TableCell>
                  <TableCell className="text-right font-mono text-[11.5px]">{formatTokens(log.tokens_in)}</TableCell>
                  <TableCell className="text-right font-mono text-[11.5px]">{formatTokens(log.tokens_out)}</TableCell>
                  <TableCell className="text-right font-mono text-[11.5px]">{formatCost(log.cost_usd)}</TableCell>
                  <TableCell className="text-right font-mono text-[11.5px]">{formatLatency(log.duration_ms)}</TableCell>
                  <TableCell><StatusPill status={log.status} /></TableCell>
                  <TableCell>
                    <button
                      type="button"
                      aria-label={`Open details for ${log.id}`}
                      className="text-muted hover:text-fg text-lg leading-none"
                      onClick={(e) => { e.stopPropagation(); onSelect(log); }}
                    >
                      →
                    </button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          <div className="flex flex-col md:flex-row items-center justify-between gap-3 px-4 py-3 border-t border-border">
            <span className="text-[12.5px] text-muted">{t('logs.table.showing')} {sorted.length}</span>
            {hasMore && (
              <Button variant="secondary" size="sm" disabled={loadingMore} onClick={onLoadMore}>
                {loadingMore ? t('logs.table.loading') : t('logs.table.loadMore')}
              </Button>
            )}
          </div>
        </>
      )}
    </Card>
  );
}
