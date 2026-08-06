'use client';

import { Card } from '@/components/ui/Card';
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

interface TimeSeriesChartProps {
  data: Array<{ time: string; value: number }>;
  title: string;
}

export function TimeSeriesChart({ data, title }: TimeSeriesChartProps) {
  return (
    <Card className="p-container-padding">
      <h3 className="text-section-sm mb-6">{title}</h3>
      <div className="h-[300px]">
        {data.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <span className="material-symbols-outlined text-4xl text-on-surface-variant mb-2">show_chart</span>
            <p className="text-body-base text-on-surface-variant">No live metric data yet</p>
          </div>
        ) : (
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data}>
              <XAxis dataKey="time" tick={{ fontSize: 12 }} />
              <YAxis tick={{ fontSize: 12 }} />
              <Tooltip />
              <Line
                type="monotone"
                dataKey="value"
                stroke="var(--color-primary)"
                strokeWidth={2}
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </div>
    </Card>
  );
}
