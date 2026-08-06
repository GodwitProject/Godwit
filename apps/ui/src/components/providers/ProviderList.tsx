import { Fragment, useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import type { Provider } from '../../lib/providers';

export interface ProviderListProps {
  providers: Provider[];
}

function maskKey(key: string): string {
  if (key.length <= 0) return '••••';
  return `${key.slice(0, 2)}-****-${key.slice(-3)}`;
}

function statusBadge(status: Provider['status']) {
  switch (status) {
    case 'healthy':
      return { label: 'Healthy', variant: 'success' as const };
    case 'degraded':
      return { label: 'Degraded', variant: 'warning' as const };
    case 'down':
      return { label: 'Down', variant: 'error' as const };
  }
}

export function ProviderList({ providers }: ProviderListProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);

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
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell>Models</TableHeadCell>
            <TableHeadCell>Avg Latency (p95)</TableHeadCell>
            <TableHeadCell>Error Rate</TableHeadCell>
            <TableHeadCell />
          </TableRow>
        </TableHead>
        <TableBody>
          {providers.map((provider) => {
            const badge = statusBadge(provider.status);
            const isExpanded = expandedId === provider.id;
            return (
              <Fragment key={provider.id}>
                <TableRow
                  className="cursor-pointer"
                  onClick={() => setExpandedId(isExpanded ? null : provider.id)}
                >
                  <TableCell>
                    <div className="flex items-center gap-3">
                      <span className="material-symbols-outlined text-on-surface-variant">hub</span>
                      <span className="font-medium">{provider.name}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant={badge.variant}>{badge.label}</Badge>
                  </TableCell>
                  <TableCell>{provider.modelCount}</TableCell>
                  <TableCell className="font-mono text-code-sm">{provider.latencyP95}ms</TableCell>
                  <TableCell className="font-mono text-code-sm">
                    {(provider.errorRate * 100).toFixed(2)}%
                  </TableCell>
                  <TableCell>
                    <span className="material-symbols-outlined text-on-surface-variant">
                      {isExpanded ? 'expand_less' : 'expand_more'}
                    </span>
                  </TableCell>
                </TableRow>
                {isExpanded && (
                  <TableRow>
                    <TableCell colSpan={6} className="bg-surface-container-low">
                      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 py-2">
                        <div>
                          <h4 className="text-caption-xs font-medium text-on-surface-variant uppercase tracking-wider mb-2">Config</h4>
                          <dl className="space-y-2 text-body-base">
                            <div className="flex justify-between gap-4">
                              <dt className="text-on-surface-variant">Base URL</dt>
                              <dd className="font-mono text-code-sm truncate">{provider.baseUrl}</dd>
                            </div>
                            <div className="flex justify-between gap-4">
                              <dt className="text-on-surface-variant">API Key</dt>
                              <dd className="font-mono text-code-sm">{maskKey(provider.apiKey)}</dd>
                            </div>
                            <div className="flex justify-between gap-4">
                              <dt className="text-on-surface-variant">Timeout</dt>
                              <dd className="font-mono text-code-sm">{provider.timeoutMs}ms</dd>
                            </div>
                          </dl>
                        </div>
                        <div>
                          <h4 className="text-caption-xs font-medium text-on-surface-variant uppercase tracking-wider mb-2">Enabled Models</h4>
                          <div className="flex flex-wrap gap-2">
                            {provider.enabledModels.map((model) => (
                              <Badge key={model} variant="info" className="font-mono text-code-sm">{model}</Badge>
                            ))}
                          </div>
                        </div>
                        <div>
                          <h4 className="text-caption-xs font-medium text-on-surface-variant uppercase tracking-wider mb-2">Fallback Chain</h4>
                          {provider.fallbackChain.length === 0 ? (
                            <p className="text-body-base text-on-surface-variant">No fallback configured</p>
                          ) : (
                            <div className="flex items-center gap-2 flex-wrap">
                              <Badge variant="default">{provider.name}</Badge>
                              {provider.fallbackChain.map((fb) => (
                                <span key={fb} className="flex items-center gap-2">
                                  <span className="material-symbols-outlined text-sm text-on-surface-variant">arrow_forward</span>
                                  <Badge variant="default" className="font-mono text-code-sm">{fb}</Badge>
                                </span>
                              ))}
                            </div>
                          )}
                          <p className="text-body-base mt-3 text-on-surface-variant">
                            {provider.fallbackTriggered === 0
                              ? 'No fallbacks triggered'
                              : `Fallback triggered ${provider.fallbackTriggered} times`}
                          </p>
                        </div>
                      </div>
                    </TableCell>
                  </TableRow>
                )}
              </Fragment>
            );
          })}
        </TableBody>
      </Table>
    </Card>
  );
}
