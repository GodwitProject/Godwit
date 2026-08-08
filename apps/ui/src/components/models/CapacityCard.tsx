import { Card } from '../ui/Card';
import { useT } from '@/hooks/useT';

export interface CapacityCardProps {
  tokensPerMinByModel: Map<string, number>;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)} M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)} k`;
  return String(Math.round(n));
}

export function CapacityCard({ tokensPerMinByModel }: CapacityCardProps) {
  const { t } = useT();

  const entries = Array.from(tokensPerMinByModel.entries()).sort((a, b) => b[1] - a[1]);
  const max = entries.length ? Math.max(...entries.map(([, v]) => v)) : 0;

  return (
    <Card className="overflow-hidden">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-[13px] font-semibold">{t('models.capacity')}</h2>
        <span className="text-[12px] text-muted">{t('models.capacitySub')}</span>
      </div>
      <div className="px-4 py-3">
        {entries.length === 0 ? (
          <div className="py-10 text-center text-[13px] text-muted">{t('models.noCapacity')}</div>
        ) : (
          entries.map(([model, value]) => {
            const pct = max > 0 ? Math.max(2, (value / max) * 100) : 0;
            return (
              <div key={model} className="flex items-center gap-3 py-2.5 border-b border-bg last:border-b-0">
                <div className="font-mono text-xs w-[190px] truncate flex-none">{model}</div>
                <div className="flex-1 h-2 rounded-full bg-bg overflow-hidden">
                  <div className="h-full bg-accent" style={{ width: `${pct}%` }} />
                </div>
                <div className="font-mono text-[11.5px] w-16 text-right">{formatTokens(value)}/mi</div>
              </div>
            );
          })
        )}
      </div>
    </Card>
  );
}
