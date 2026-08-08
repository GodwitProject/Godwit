import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { useT } from '@/hooks/useT';
import type { Provider } from '@/lib/providers';

export interface ProvidersCardProps {
  providers: Provider[];
  onToggle: (id: string, enabled: boolean) => void;
  toggling?: boolean;
}

function short(name: string): string {
  return name
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0])
    .join('')
    .toUpperCase()
    .slice(0, 3) || 'PR';
}

export function ProvidersCard({ providers, onToggle, toggling }: ProvidersCardProps) {
  const { t } = useT();

  const activeCount = providers.filter((p) => p.enabled).length;

  return (
    <Card className="overflow-hidden">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-[13px] font-semibold">{t('models.providers')}</h2>
        <span className="text-[12px] text-muted">{t('models.providersCount')} {activeCount}/{providers.length}</span>
      </div>
      <div>
        {providers.length === 0 ? (
          <div className="px-4 py-12 text-center text-[13px] text-muted">{t('providers.noProviders')}</div>
        ) : (
          providers.map((p) => (
            <div key={p.id} className="flex items-center justify-between gap-3 px-4 py-3 border-b border-bg last:border-b-0 hover:bg-[oklch(97.5%_0.004_250)]">
              <div className="flex items-center gap-3 min-w-0">
                <span
                  className="grid place-items-center w-[30px] h-[30px] rounded-lg font-bold text-xs text-white flex-none"
                  style={{ background: 'oklch(58% 0.16 145)' }}
                >
                  {short(p.name)}
                </span>
                <div className="min-w-0">
                  <div className="font-medium text-[13px] truncate">{p.name}</div>
                  <div className="text-[11.5px] text-muted font-mono truncate">{p.protocol}</div>
                </div>
              </div>
              <div className="flex items-center gap-2.5 flex-none">
                <Badge variant={p.has_credentials ? 'success' : 'warning'}>
                  {p.has_credentials ? t('providers.configured') : t('providers.missing')}
                </Badge>
                <button
                  type="button"
                  className="border border-border rounded-md text-[11.5px] font-medium px-2.5 py-1 text-muted hover:text-fg hover:bg-bg"
                >
                  {t('providers.configure')}
                </button>
                <label className="inline-flex cursor-pointer items-center">
                  <input
                    type="checkbox"
                    className="sr-only peer"
                    checked={p.enabled}
                    disabled={toggling}
                    onChange={(e) => onToggle(p.id, e.target.checked)}
                    aria-label={`Toggle ${p.name}`}
                  />
                  <span className="relative inline-flex h-5 w-9 items-center rounded-full bg-border transition-colors peer-checked:bg-accent peer-disabled:opacity-60">
                    <span className="inline-block h-4 w-4 transform rounded-full bg-white transition-transform peer-checked:translate-x-4 shadow-sm" />
                  </span>
                </label>
              </div>
            </div>
          ))
        )}
      </div>
    </Card>
  );
}
