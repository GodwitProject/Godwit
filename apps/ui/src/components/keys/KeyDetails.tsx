import { Modal } from '../ui/Modal';
import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';
import { KeyForm } from './KeyForm';
import type { ApiKey, KeyUsage, KeyLog, CreateKeyRequest } from '../../lib/keys';

export interface KeyDetailsProps {
  apiKey: ApiKey;
  usage?: KeyUsage;
  logs?: KeyLog[];
  owners: string[];
  availableModels: string[];
  onClose: () => void;
  onSave: (req: CreateKeyRequest) => void;
  editing: boolean;
  onStartEdit: () => void;
}

export function KeyDetails({
  apiKey,
  usage,
  logs,
  owners,
  availableModels,
  onClose,
  onSave,
  editing,
  onStartEdit,
}: KeyDetailsProps) {
  return (
    <Modal open onClose={onClose} title={apiKey.name} maxWidth="max-w-3xl">
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Badge variant={apiKey.status === 'active' ? 'success' : 'error'}>
              {apiKey.status}
            </Badge>
            <span className="font-mono text-code-sm text-on-surface-variant">{apiKey.prefix || 'sk_live_****'}</span>
          </div>
          {!editing && (
            <Button variant="secondary" size="sm" onClick={onStartEdit}>
              Edit
            </Button>
          )}
        </div>

        {editing ? (
          <KeyForm
            open
            initial={apiKey}
            owners={owners}
            availableModels={availableModels}
            onClose={onClose}
            onSubmit={async (req) => {
              onSave(req);
            }}
          />
        ) : (
          <>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <Card variant="filled" className="p-4">
                <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Spend (30d)</p>
                <p className="font-mono text-title-md mt-1">${(usage?.totalSpend ?? 0).toFixed(2)}</p>
              </Card>
              <Card variant="filled" className="p-4">
                <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Requests</p>
                <p className="font-mono text-title-md mt-1">{usage?.totalRequests ?? 0}</p>
              </Card>
              <Card variant="filled" className="p-4">
                <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Budget</p>
                <p className="font-mono text-title-md mt-1">{apiKey.budget != null ? `$${apiKey.budget.toFixed(2)}` : 'Unlimited'}</p>
              </Card>
              <Card variant="filled" className="p-4">
                <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Models</p>
                <p className="font-mono text-title-md mt-1">{apiKey.allowedModels?.length ?? 0}</p>
              </Card>
            </div>

            <Card className="p-container-padding">
              <h3 className="text-section-sm mb-4">Spend Last 30 Days</h3>
              <div className="h-[220px]">
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={usage?.timeseries || []}>
                    <XAxis dataKey="day" tick={{ fontSize: 12 }} />
                    <YAxis tick={{ fontSize: 12 }} />
                    <Tooltip />
                    <Line type="monotone" dataKey="spend" stroke="#004ac6" strokeWidth={2} dot={false} />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </Card>

            <Card className="overflow-hidden">
              <div className="p-container-padding border-b hairline-border">
                <h3 className="text-section-sm">Recent Activity</h3>
              </div>
              {!logs || logs.length === 0 ? (
                <p className="text-body-base text-on-surface-variant p-container-padding">No recent activity.</p>
              ) : (
                <Table>
                  <TableHead>
                    <TableRow>
                      <TableHeadCell>Timestamp</TableHeadCell>
                      <TableHeadCell>Model</TableHeadCell>
                      <TableHeadCell>Status</TableHeadCell>
                      <TableHeadCell>Tokens</TableHeadCell>
                      <TableHeadCell className="text-right">Cost</TableHeadCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {logs.map((log) => (
                      <TableRow key={log.id}>
                        <TableCell className="text-on-surface-variant">{log.timestamp}</TableCell>
                        <TableCell className="font-mono text-code-sm">{log.model}</TableCell>
                        <TableCell>
                          <Badge variant={log.status === 200 ? 'success' : 'error'}>{log.status}</Badge>
                        </TableCell>
                        <TableCell className="font-mono text-code-sm">
                          {log.tokensIn + log.tokensOut}
                        </TableCell>
                        <TableCell className="text-right font-mono text-code-sm">${log.cost.toFixed(4)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </Card>
          </>
        )}
      </div>
    </Modal>
  );
}
