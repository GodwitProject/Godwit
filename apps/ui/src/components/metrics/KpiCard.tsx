import { clsx } from 'clsx';

interface KpiCardProps {
  label: string;
  value: string;
  delta?: {
    text: string;
    tone: 'up' | 'down' | 'no';
  };
  spark?: number[];
  sparkColor?: string;
  icon?: React.ReactNode;
}

function Sparkline({ data, color }: { data: number[]; color: string }) {
  if (data.length < 2) return null;
  const w = 120;
  const h = 34;
  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;
  const pts = data.map((v, i) => {
    const x = (i / (data.length - 1)) * w;
    const y = h - 5 - ((v - min) / range) * (h - 10);
    return [x, y];
  });
  const d = pts.map(([x, y], i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`).join(' ');
  return (
    <svg className="spark" width={w} height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" aria-hidden="true">
      <path d={d} fill="none" stroke={color} strokeWidth={2} />
    </svg>
  );
}

export function KpiCard({ label, value, delta, spark, sparkColor, icon }: KpiCardProps) {
  return (
    <div className="kpi">
      <div className="lab">
        {icon}
        {label}
      </div>
      <div className="val">{value}</div>
      {delta && (
        <div className={clsx('delta', delta.tone)}>
          {delta.text}
        </div>
      )}
      {spark && <Sparkline data={spark} color={sparkColor || 'oklch(55% 0.15 155)'} />}
    </div>
  );
}
