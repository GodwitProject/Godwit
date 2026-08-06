'use client';

import { MetricCard } from '@/components/metrics/MetricCard';
import { TimeSeriesChart } from '@/components/metrics/TimeSeriesChart';
import { RecentLogsTable } from '@/components/logs/RecentLogsTable';
import { useMetrics, useLatency, useTokens } from '@/hooks/useMetrics';
import { fetchRecentLogs } from '@/lib/api';
import { useQuery } from '@tanstack/react-query';

export default function Dashboard() {
  const { data: metrics, isLoading: metricsLoading } = useMetrics();
  const { data: latency } = useLatency();
  const { data: tokens } = useTokens();
  const { data: logs } = useQuery({ queryKey: ['recent-logs'], queryFn: () => fetchRecentLogs(10) });

  if (metricsLoading) return <div>Loading...</div>;

  return (
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">Dashboard</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">Real-time LLM proxy performance metrics.</p>
        </div>
        <div className="flex gap-3">
          <button className="bg-surface-container-lowest hairline-border px-4 py-2 rounded flex items-center gap-2">
            <span className="material-symbols-outlined">calendar_month</span>
            Last 24 Hours
          </button>
          <button className="bg-primary text-on-primary px-4 py-2 rounded flex items-center gap-2">
            <span className="material-symbols-outlined">download</span>
            Export
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <MetricCard
          title="Total Requests"
          value={metrics?.totalRequests || '0'}
          trend={{ value: '+12.5% from yesterday', direction: 'up' }}
          icon="swap_vert"
        />
        <MetricCard
          title="Avg Latency"
          value={`${latency?.p95 || 0}ms`}
          trend={{ value: '+42ms from yesterday', direction: 'up' }}
          trendColor="error"
          icon="timer"
        />
        <MetricCard
          title="Token Usage"
          value={`${tokens?.total || 0}M`}
          trend={{ value: '+5.2% from yesterday', direction: 'up' }}
          icon="toll"
        />
        <MetricCard
          title="Error Rate"
          value={`${metrics?.errorRate || 0}%`}
          trend={{ value: 'Stable', direction: 'flat' }}
          icon="error"
        />
      </div>

      <TimeSeriesChart
        data={metrics?.timeseries || []}
        title="Request Volume"
      />

      <RecentLogsTable logs={logs || []} />
    </>
  );
}
