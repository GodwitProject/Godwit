import { Card } from '@/components/ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '@/components/ui/Table';
import type { RequestLog } from '@/lib/logs';

export interface RecentLogsTableProps {
  logs: RequestLog[];
}

function formatLatency(durationMs: number | null): string {
  return durationMs != null ? `${durationMs}ms` : '—';
}

export function RecentLogsTable({ logs }: RecentLogsTableProps) {
  const isEmpty = logs.length === 0;
  return (
    <Card className="overflow-hidden">
      <div className="p-container-padding border-b hairline-border flex justify-between items-center">
        <h3 className="text-section-sm">Recent Proxy Events</h3>
        <a href="/logs" className="text-label-sm text-primary hover:underline">View All Logs</a>
      </div>
      {isEmpty ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="material-symbols-outlined text-4xl text-on-surface-variant mb-2">receipt_long</span>
          <p className="text-body-base text-on-surface-variant">No recent proxy events yet.</p>
        </div>
      ) : (
        <Table>
          <TableHead>
            <TableRow>
              <TableHeadCell>Timestamp</TableHeadCell>
              <TableHeadCell>Log ID</TableHeadCell>
              <TableHeadCell>Model</TableHeadCell>
              <TableHeadCell>Provider</TableHeadCell>
              <TableHeadCell className="text-right">Latency</TableHeadCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {logs.map((log) => (
              <TableRow key={log.id}>
                <TableCell className="text-on-surface-variant">{log.created_at}</TableCell>
                <TableCell className="font-mono text-code-sm">{log.id}</TableCell>
                <TableCell>{log.model}</TableCell>
                <TableCell>{log.provider}</TableCell>
                <TableCell className="text-right font-mono text-code-sm">{formatLatency(log.duration_ms)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </Card>
  );
}
