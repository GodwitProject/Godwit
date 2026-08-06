import { Card } from '@/components/ui/Card';
import { clsx } from 'clsx';

interface MetricCardProps {
  title: string;
  value: string;
  trend?: {
    value: string;
    direction: 'up' | 'down' | 'flat';
  };
  note?: string;
  icon?: string;
  trendColor?: 'primary' | 'error' | 'success';
}

export function MetricCard({ title, value, trend, note, icon, trendColor = 'primary' }: MetricCardProps) {
  return (
    <Card className="flex flex-col">
      <div className="flex justify-between items-start mb-4">
        <span className="text-label-sm text-on-surface-variant uppercase tracking-wider">{title}</span>
        {icon && <span className="material-symbols-outlined text-outline">{icon}</span>}
      </div>
      <div className="mt-auto">
        <span className="text-display-lg text-on-surface">{value}</span>
        {note && (
          <p className="text-label-sm text-on-surface-variant mt-2">{note}</p>
        )}
        {trend && (
          <div className={clsx('flex items-center gap-1 mt-2 text-label-sm', trendColor === 'error' ? 'text-error' : 'text-primary')}>
            <span className="material-symbols-outlined text-[16px]">
              {trend.direction === 'up' ? 'trending_up' : trend.direction === 'down' ? 'trending_down' : 'trending_flat'}
            </span>
            <span>{trend.value}</span>
          </div>
        )}
      </div>
    </Card>
  );
}
