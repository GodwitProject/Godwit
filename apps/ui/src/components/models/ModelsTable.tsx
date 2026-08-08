import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { useT } from '@/hooks/useT';
import type { ApiModel } from '@/lib/models';

export interface ModelsTableProps {
  models: ApiModel[];
  latencyByModel: Map<string, number | null>;
  protocolEnabled: Set<string>;
}

function formatLatency(ms: number | null): string {
  if (ms == null) return '—';
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)} s` : `${Math.round(ms)} ms`;
}

function shortProvider(provider: string): string {
  return provider.slice(0, 1).toUpperCase();
}

export function ModelsTable({ models, latencyByModel, protocolEnabled }: ModelsTableProps) {
  const { t } = useT();

  return (
    <Card className="overflow-hidden">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-[13px] font-semibold">{t('models.declared')}</h2>
        <span className="text-[12px] text-muted">{models.length} {t('models.declaredCountUnit')}</span>
      </div>
      {models.length === 0 ? (
        <div className="px-4 py-12 text-center text-[13px] text-muted">{t('models.noModels')}</div>
      ) : (
        <Table>
          <TableHead>
            <TableRow>
              <TableHeadCell>{t('models.exposed')}</TableHeadCell>
              <TableHeadCell>{t('models.providerCol')}</TableHeadCell>
              <TableHeadCell>{t('models.providerSideId')}</TableHeadCell>
              <TableHeadCell className="text-right">{t('models.latency')}</TableHeadCell>
              <TableHeadCell>{t('recent.status')}</TableHeadCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {models.map((m) => {
              const active = protocolEnabled.has(m.provider);
              return (
                <TableRow key={m.id}>
                  <TableCell>
                    <span className="font-mono text-[11.5px] font-medium">{m.public_id}</span>
                  </TableCell>
                  <TableCell>
                    <span className="inline-flex items-center gap-2">
                      <span className="grid place-items-center w-5 h-5 rounded bg-bg border border-border text-[10px] font-bold text-muted">
                        {shortProvider(m.provider)}
                      </span>
                      <span className="tag">{m.provider}</span>
                    </span>
                  </TableCell>
                  <TableCell><span className="font-mono text-[11.5px] text-muted">{m.provider_model_id}</span></TableCell>
                  <TableCell className="text-right font-mono text-[11.5px]">{formatLatency(latencyByModel.get(m.public_id) ?? null)}</TableCell>
                  <TableCell>
                    <span className={`pill ${active ? 'ok' : 'err'}`}>
                      <span className="dot" />
                      {active ? t('state.ok') : t('providers.disabled')}
                    </span>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}
    </Card>
  );
}
