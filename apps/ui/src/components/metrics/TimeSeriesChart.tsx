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
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data}>
            <XAxis dataKey="time" tick={{ fontSize: 12 }} />
            <YAxis tick={{ fontSize: 12 }} />
            <Tooltip />
            <Line type="monotone" dataKey="value" stroke="#004ac6" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
