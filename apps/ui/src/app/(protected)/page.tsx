'use client';

import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { MetricCard } from '@/components/metrics/MetricCard';
import { TimeSeriesChart } from '@/components/metrics/TimeSeriesChart';
import { RecentLogsTable } from '@/components/logs/RecentLogsTable';
import { useRealtimeMetrics } from '@/hooks/useRealtimeMetrics';
import { fetchStats, fetchSpend } from '@/lib/api';
import { fetchLogs } from '@/lib/logs';

function ConnectionBadge({ status }: { status: 'connecting' | 'live' | 'polling' | 'error' }) {
  const config = {
    connecting: { label: 'Connecting…', dot: 'bg-on-surface-variant', pulse: false },
    live: { label: 'Live', dot: 'bg-success', pulse: true },
    polling: { label: 'Polling', dot: 'bg-warning', pulse: false },
    error: { label: 'Offline', dot: 'bg-error', pulse: false },
  }[status];

  return (
    <div className="flex items-center gap-2 text-label-sm text-on-surface-variant">
      <span className={`h-2 w-2 rounded-full ${config.dot} ${config.pulse ? 'animate-pulse' : ''}`} />
      <span>{config.label}</span>
    </div>
  );
}

function formatCost(value: number | null): string {
  return value != null ? `$${value.toFixed(2)}` : '—';
}

export default function Dashboard() {
  const { data: metrics, status } = useRealtimeMetrics();
  const { data: stats } = useQuery({ queryKey: ['admin-stats'], queryFn: fetchStats });
  const { data: spend } = useQuery({ queryKey: ['spend', 30], queryFn: () => fetchSpend(30) });
  const { data: logsPage } = useQuery({ queryKey: ['recent-logs'], queryFn: () => fetchLogs({ limit: 10 }) });

  const spendSeries = useMemo(
    () => (spend || []).map((point) => ({ time: point.date, value: point.cost })),
    [spend]
  );

  const recentLogs = logsPage?.items ?? [];
  const hasLiveMetrics = metrics.requestsTotal > 0;

  return (
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">Dashboard</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">LLM proxy performance metrics.</p>
        </div>
        <div className="flex items-center gap-3">
          <ConnectionBadge status={status} />
          <button className="bg-surface-container-lowest hairline-border px-4 py-2 rounded flex items-center gap-2">
            <span className="material-symbols-outlined">calendar_month</span>
            Last 30 Days
          </button>
          <button className="bg-primary text-on-primary px-4 py-2 rounded flex items-center gap-2">
            <span className="material-symbols-outlined">download</span>
            Export
          </button>
        </div>
      </div>

      <section>
        <h2 className="text-headline-md mb-4">Accounts</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <MetricCard title="Organizations" value={String(stats?.organizations ?? 0)} icon="business" />
          <MetricCard title="Teams" value={String(stats?.teams ?? 0)} icon="groups" />
          <MetricCard title="Users" value={String(stats?.users ?? 0)} icon="person" />
          <MetricCard title="API Keys" value={String(stats?.apiKeys ?? 0)} icon="vpn_key" />
        </div>
      </section>

      <section>
        <h2 className="text-headline-md mb-4">Live Metrics</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <MetricCard
            title="Total Requests"
            value={hasLiveMetrics ? String(metrics.requestsTotal) : '—'}
            icon="swap_vert"
            note={hasLiveMetrics ? undefined : 'No live metric data yet'}
          />
          <MetricCard
            title="Tokens"
            value={hasLiveMetrics ? String(metrics.tokensTotal) : '—'}
            icon="toll"
            note={hasLiveMetrics ? undefined : 'No live metric data yet'}
          />
          <MetricCard
            title="Active Requests"
            value={hasLiveMetrics ? String(metrics.activeRequests) : '—'}
            icon="bolt"
            note={hasLiveMetrics ? undefined : 'No live metric data yet'}
          />
          <MetricCard
            title="Total Cost"
            value={hasLiveMetrics ? formatCost(metrics.costUsdTotal) : '—'}
            icon="payments"
            note={hasLiveMetrics ? undefined : 'No live metric data yet'}
          />
        </div>
      </section>

      <TimeSeriesChart data={spendSeries} title="Spend (Last 30 Days)" />

      <RecentLogsTable logs={recentLogs} />
    </>
  );
}
