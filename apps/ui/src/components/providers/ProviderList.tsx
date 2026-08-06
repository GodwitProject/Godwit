import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import type { Provider } from '../../lib/providers';

export interface ProviderListProps {
  providers: Provider[];
}

export function ProviderList({ providers }: ProviderListProps) {
  if (providers.length === 0) {
    return (
      <Card>
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="material-symbols-outlined text-4xl text-on-surface-variant mb-2">hub</span>
          <p className="text-body-base text-on-surface-variant">No providers configured yet.</p>
        </div>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <div className="p-container-padding border-b hairline-border">
        <h3 className="text-section-sm">Provider List</h3>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Provider</TableHeadCell>
            <TableHeadCell>Protocol</TableHeadCell>
            <TableHeadCell>Base URL</TableHeadCell>
            <TableHeadCell>Credentials</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {providers.map((provider) => (
            <TableRow key={provider.id}>
              <TableCell>
                <div className="flex items-center gap-3">
                  <span className="material-symbols-outlined text-on-surface-variant">hub</span>
                  <span className="font-medium">{provider.name}</span>
                </div>
              </TableCell>
              <TableCell className="font-mono text-code-sm">{provider.protocol}</TableCell>
              <TableCell className="font-mono text-code-sm truncate max-w-xs">{provider.base_url}</TableCell>
              <TableCell>
                <Badge variant={provider.has_credentials ? 'success' : 'warning'}>
                  {provider.has_credentials ? 'Configured' : 'Missing'}
                </Badge>
              </TableCell>
              <TableCell>
                <Badge variant={provider.enabled ? 'success' : 'default'}>
                  {provider.enabled ? 'Enabled' : 'Disabled'}
                </Badge>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
