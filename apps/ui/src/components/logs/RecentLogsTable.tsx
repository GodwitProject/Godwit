import Link from 'next/link';
import { Card } from '@/components/ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '@/components/ui/Table';
import { useT } from '@/hooks/useT';
import type { RequestLog } from '@/lib/logs';

export interface RecentLogsTableProps {
  logs: RequestLog[];
}

function formatTime(iso: string): string {
  if (!iso) return '—';
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return iso;
  }
}

function formatLatency(durationMs: number | null): string {
  return durationMs != null ? `${durationMs}ms` : '—';
}

export function RecentLogsTable({ logs }: RecentLogsTableProps) {
  const { t } = useT();
  const isEmpty = logs.length === 0;
  return (
    <Card className="overflow-hidden">
      <div className="px-4 py-3 border-b border-border flex justify-between items-center">
        <h2 className="text-[13px] font-semibold">{t('recent.errors')}</h2>
        <Link href="/logs" className="text-[11.5px] text-accent-strong font-medium hover:underline">
          {t('recent.viewAll')}
        </Link>
      </div>
      {isEmpty ? (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <span className="text-3xl text-muted mb-2">📋</span>
          <p className="text-[13px] text-muted">{t('kpi.noLiveData')}</p>
        </div>
      ) : (
        <Table>
          <TableHead>
            <TableRow>
              <TableHeadCell>{t('recent.timestamp')}</TableHeadCell>
              <TableHeadCell>{t('logs.table.id')}</TableHeadCell>
              <TableHeadCell>{t('recent.model')}</TableHeadCell>
              <TableHeadCell>{t('traffic.provider')}</TableHeadCell>
              <TableHeadCell className="text-right">{t('logs.table.latency')}</TableHeadCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {logs.map((log) => (
              <TableRow key={log.id}>
                <TableCell className="text-muted font-mono">{formatTime(log.created_at)}</TableCell>
                <TableCell className="font-mono text-[11.5px] text-muted">{log.id}</TableCell>
                <TableCell className="font-mono text-[11.5px] font-medium">{log.model}</TableCell>
                <TableCell><span className="tag">{log.provider}</span></TableCell>
                <TableCell className="text-right font-mono text-[11.5px]">{formatLatency(log.duration_ms)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </Card>
  );
}
