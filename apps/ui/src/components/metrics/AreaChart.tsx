interface Point {
  time: string;
  value: number;
}

interface AreaChartProps {
  data: Point[];
  height?: number;
  accent?: string;
  baselineColor?: string;
}

export function AreaChart({ data, height = 220, accent = 'var(--accent-strong)', baselineColor = 'oklch(50% 0.01 240)' }: AreaChartProps) {
  const w = 640;
  const h = height;
  const pad = 6;

  if (data.length < 2) {
    return (
      <div className="flex flex-col items-center justify-center text-center" style={{ height }}>
        <span className="text-3xl text-muted mb-2">📈</span>
        <p className="text-[12px] text-muted">No data yet</p>
      </div>
    );
  }

  const values = data.map((d) => d.value);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;

  const coords = data.map((d, i) => {
    const x = (i / (data.length - 1)) * w;
    const y = h - pad - ((d.value - min) / range) * (h - pad * 2);
    return [x, y];
  });

  const line = coords.map(([x, y], i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`).join(' ');
  const area = `${line} L${w},${h} L0,${h} Z`;
  const baseline = `M0,${h - pad} L${w},${h - pad}`;

  const gridLines = [0.2, 0.5, 0.8];

  return (
    <div className="px-3 py-2">
      <svg viewBox={`0 0 ${w} ${h}`} width="100%" height={height} role="img" aria-label="Chart">
        <defs>
          <linearGradient id="areaFill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="oklch(58% 0.16 145 / 0.55)" />
            <stop offset="1" stopColor="oklch(58% 0.16 145 / 0.02)" />
          </linearGradient>
        </defs>
        {gridLines.map((g, i) => (
          <line key={i} x1="0" y1={h * g} x2={w} y2={h * g} stroke="var(--border)" strokeWidth="1" />
        ))}
        <path d={area} fill="url(#areaFill)" stroke="none" />
        <path d={line} fill="none" stroke={accent} strokeWidth="2" />
        <path d={baseline} fill="none" stroke={baselineColor} strokeWidth="1.5" strokeDasharray="4 4" />
      </svg>
    </div>
  );
}
