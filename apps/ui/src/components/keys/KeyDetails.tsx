import { Drawer } from '../ui/Drawer';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import { useT } from '@/hooks/useT';
import type { ApiKey } from '../../lib/keys';

export interface KeyDetailsProps {
  apiKey: ApiKey;
  onClose: () => void;
  onToggleActive: () => void;
  onDelete: () => void;
}

function formatNullable(value: number | null, suffix = ''): string {
  return value != null ? `${value.toLocaleString()}${suffix}` : '—';
}

function formatExpiry(iso: string | null): string {
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return iso;
  }
}

export function KeyDetails({ apiKey, onClose, onToggleActive, onDelete }: KeyDetailsProps) {
  const { t } = useT();
  const active = !apiKey.disabled;

  return (
    <Drawer
      open
      onClose={onClose}
      title={t('keys.details.title')}
      subtitle={`${apiKey.name} · ${apiKey.key_prefix || 'sk_live_****'}`}
      header={
        <span className={`pill ${active ? 'ok' : 'err'}`}><span className="dot" />{active ? t('keys.status.active') : t('keys.status.revoked')}</span>
      }
    >
      <div className="flex items-center justify-end gap-2 mb-5">
        <Button variant="secondary" size="sm" onClick={onToggleActive}>
          {active ? t('keys.details.revoke') : t('keys.details.unblock')}
        </Button>
        <Button variant="danger" size="sm" onClick={onDelete}>
          {t('keys.delete')}
        </Button>
        <Button variant="secondary" size="sm" onClick={onClose}>
          {t('keys.details.close')}
        </Button>
      </div>

      <div className="fact-grid grid grid-cols-2 gap-2.5">
        <Fact k={t('keys.details.spent')} v={apiKey.budget_spent_usd != null ? `$${apiKey.budget_spent_usd.toFixed(2)}` : '—'} />
        <Fact k={t('keys.details.budget')} v={apiKey.budget_limit_usd != null ? `$${apiKey.budget_limit_usd.toFixed(2)}` : t('keys.unlimited')} />
        <Fact k={t('keys.details.rateRpm')} v={formatNullable(apiKey.rate_limit_requests_per_minute)} />
        <Fact k={t('keys.details.rateTpm')} v={formatNullable(apiKey.rate_limit_tokens_per_minute)} />
        <Fact k={t('keys.details.expires')} v={formatExpiry(apiKey.expires_at)} />
        <Fact k={t('keys.details.created')} v={apiKey.created_at || '—'} />
        <Fact k={t('keys.details.organization')} v={apiKey.organization_id || '—'} />
        <Fact k={t('keys.details.user')} v={apiKey.user_id || '—'} />
      </div>

      <div className="mt-5">
        <div className="lbl mb-1.5 text-[11px] uppercase tracking-wider text-muted font-medium">{t('keys.scopes')}</div>
        <div className="flex flex-wrap gap-1.5">
          {apiKey.scopes.length === 0 ? (
            <span className="text-[13px] text-muted">—</span>
          ) : (
            apiKey.scopes.map((scope) => <Badge key={scope} variant="info">{scope}</Badge>)
          )}
        </div>
      </div>

      <div className="mt-5">
        <div className="lbl mb-1.5 text-[11px] uppercase tracking-wider text-muted font-medium">{t('keys.create.allowedModels')}</div>
        {apiKey.allowed_models.length === 0 ? (
          <p className="text-[13px] text-muted">{t('keys.details.noModels')}</p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {apiKey.allowed_models.map((model) => (
              <span key={model} className="tag font-mono">{model}</span>
            ))}
          </div>
        )}
      </div>

      <div className="mt-5">
        <div className="lbl mb-1.5 text-[11px] uppercase tracking-wider text-muted font-medium">{t('keys.details.spendTrend')}</div>
        <div className="bg-bg border border-border rounded-lg px-3 py-2.5">
          <p className="text-[13px] text-muted">{t('keys.details.noSpend')}</p>
        </div>
      </div>
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
