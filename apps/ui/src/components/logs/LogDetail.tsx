import { Drawer } from '../ui/Drawer';
import { useT } from '@/hooks/useT';
import type { RequestLog } from '../../lib/logs';

export interface LogDetailProps {
  open: boolean;
  log?: RequestLog;
  onClose: () => void;
}

function formatDate(iso?: string): string {
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
}

function formatLatency(durationMs: number | null): string {
  return durationMs != null ? `${durationMs}ms` : '—';
}

export function LogDetail({ open, log, onClose }: LogDetailProps) {
  const { t } = useT();

  return (
    <Drawer open={open} onClose={onClose} title={t('logs.detail.title')} subtitle={log?.id}>
      {!log ? (
        <p className="text-[13px] text-muted">{t('logs.detail.noSelection')}</p>
      ) : (
        <div className="space-y-5">
          <div className="fact-grid grid grid-cols-2 gap-2.5">
            <Fact k={t('logs.detail.model')} v={log.model || '—'} />
            <Fact k={t('logs.detail.provider')} v={log.provider || '—'} />
            <Fact k={t('logs.detail.capability')} v={log.capability || '—'} />
            <Fact k={t('logs.detail.streamed')} v={log.streamed ? t('yes') : t('no')} />
            <Fact k={t('logs.table.latency')} v={formatLatency(log.duration_ms)} />
            <Fact k={t('traffic.cost')} v={formatCost(log.cost_usd)} />
            <Fact k={t('logs.detail.apiKey')} v={log.api_key_id || '—'} />
            <Fact k={t('recent.timestamp')} v={formatDate(log.created_at)} />
          </div>

          <div>
            <div className="lbl mb-1.5 text-[11px] uppercase tracking-wider text-muted font-medium">
              {t('logs.detail.details')}
            </div>
            <div className="bg-bg border border-border rounded-lg px-3 py-2.5">
              <p className="text-[13px] text-muted">{t('logs.detail.detailsNote')}</p>
            </div>
          </div>
        </div>
      )}
    </Drawer>
  );
}

function Fact({ k, v }: { k: string; v: string }) {
  return (
    <div className="bg-bg border border-border rounded-lg px-3 py-2.5">
      <div className="text-[10.5px] uppercase tracking-wider text-muted mb-0.5">{k}</div>
      <div className="font-mono text-[13px] font-medium">{v}</div>
    </div>
  );
}
