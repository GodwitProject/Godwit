import { Card } from '@/components/ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';

interface Log {
  timestamp: string;
  requestId: string;
  model: string;
  status: number;
  latencyMs: number;
}

interface RecentLogsTableProps {
  logs: Log[];
}

export function RecentLogsTable({ logs }: RecentLogsTableProps) {
  return (
    <Card className="overflow-hidden">
      <div className="p-container-padding border-b hairline-border flex justify-between items-center">
        <h3 className="text-section-sm">Recent Proxy Events</h3>
        <a href="/logs" className="text-label-sm text-primary hover:underline">View All Logs</a>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Timestamp</TableHeadCell>
            <TableHeadCell>Request ID</TableHeadCell>
            <TableHeadCell>Model</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell className="text-right">Latency</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {logs.map((log) => (
            <TableRow key={log.requestId}>
              <TableCell className="text-on-surface-variant">{log.timestamp}</TableCell>
              <TableCell className="font-mono text-code-sm">{log.requestId}</TableCell>
              <TableCell>{log.model}</TableCell>
              <TableCell>
                <Badge variant={log.status === 200 ? 'success' : 'error'}>
                  {log.status} {log.status === 200 ? 'OK' : 'Error'}
                </Badge>
              </TableCell>
              <TableCell className="text-right font-mono text-code-sm">{log.latencyMs}ms</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
