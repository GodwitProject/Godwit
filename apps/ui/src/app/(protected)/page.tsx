'use client';

import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { KpiCard } from '@/components/metrics/KpiCard';
import { AreaChart } from '@/components/metrics/AreaChart';
import { RecentLogsTable } from '@/components/logs/RecentLogsTable';
import { useRealtimeMetrics } from '@/hooks/useRealtimeMetrics';
import { useT } from '@/hooks/useT';
import { fetchStats, fetchSpend } from '@/lib/api';
import { fetchLogs } from '@/lib/logs';
import { Card } from '@/components/ui/Card';
import { CalendarIcon, PlusIcon, BoltIcon, SearchIcon, BoxIcon } from '@/components/icons';

function ConnectionBadge({ status }: { status: 'connecting' | 'live' | 'polling' | 'error' }) {
  const { t } = useT();
  const config = {
    connecting: { label: t('conn.connecting'), cls: 'bg-muted', pulse: false },
    live: { label: t('conn.live'), cls: 'bg-success', pulse: true },
    polling: { label: t('conn.polling'), cls: 'bg-warn', pulse: false },
    error: { label: t('conn.offline'), cls: 'bg-danger', pulse: false },
  }[status];
  return (
    <span className="flex items-center gap-1.5 text-[12.5px] text-muted">
      <span className={`h-2 w-2 rounded-full ${config.cls} ${config.pulse ? 'animate-pulse' : ''}`} />
      {config.label}
    </span>
  );
}

function formatCost(value: number | null): string {
  return value != null ? `$${value.toFixed(2)}` : '—';
}

function fmtCount(value: number | null | undefined): string {
  return value != null ? String(value) : '—';
}

export default function Dashboard() {
  const { t } = useT();
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
    <div className="view-fade space-y-5">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b border-border pb-4">
        <div>
          <h1 className="text-display-lg">{t('page.overview.title')}</h1>
          <p className="text-[13px] text-muted mt-1 max-w-[62ch]">{t('page.overview.subtitle')}</p>
        </div>
        <div className="flex items-center gap-2">
          <ConnectionBadge status={status} />
          <button className="btn">
            <CalendarIcon width={14} height={14} />
            24 h
          </button>
          <button className="btn primary">
            <PlusIcon width={14} height={14} />
            {t('top.newRequest')}
          </button>
        </div>
      </div>

      <section>
        <h2 className="text-[13px] font-semibold mb-3">{t('accounts.title')}</h2>
        <div className="kpi-grid grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
          <KpiCard label={t('accounts.organizations')} value={fmtCount(stats?.organizations)} />
          <KpiCard label={t('accounts.teams')} value={fmtCount(stats?.teams)} />
          <KpiCard label={t('accounts.users')} value={fmtCount(stats?.users)} />
          <KpiCard label={t('accounts.apiKeys')} value={fmtCount(stats?.apiKeys)} />
        </div>
      </section>

      <section>
        <h2 className="text-[13px] font-semibold mb-3">{t('liveMetrics.title')}</h2>
        <div className="kpi-grid grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
          <KpiCard
            label={t('kpi.requestsTotal')}
            value={hasLiveMetrics ? fmtCount(metrics.requestsTotal) : '—'}
            icon={<SearchIcon className="text-muted" />}
          />
          <KpiCard
            label={t('kpi.tokens')}
            value={hasLiveMetrics ? fmtCount(metrics.tokensTotal) : '—'}
            icon={<BoxIcon className="text-muted" />}
          />
          <KpiCard
            label={t('kpi.activeRequests')}
            value={hasLiveMetrics ? fmtCount(metrics.activeRequests) : '—'}
            icon={<BoltIcon className="text-muted" />}
          />
          <KpiCard
            label={t('kpi.totalCost')}
            value={hasLiveMetrics ? formatCost(metrics.costUsdTotal) : '—'}
            icon={<BoxIcon className="text-muted" />}
          />
        </div>
      </section>

      <Card className="grid grid-cols-1 xl:grid-cols-[1.35fr_1fr] gap-0 overflow-hidden">
        <div className="border-b xl:border-b-0 xl:border-r border-border">
          <div className="flex items-center justify-between px-4 py-3 border-b border-border">
            <h2 className="text-[13px] font-semibold">{t('chart.requestsPerMinute')}</h2>
            <span className="text-[12px] text-muted">30 j</span>
          </div>
          <AreaChart data={spendSeries} />
          <div className="flex gap-3.5 text-[11.5px] text-muted px-4 pb-3.5">
            <span><i className="inline-block w-2 h-2 rounded-[2px] mr-1 align-middle bg-accent-strong" />{t('chart.requests')}</span>
          </div>
        </div>
        <div>
          <div className="flex items-center justify-between px-4 py-3 border-b border-border">
            <h2 className="text-[13px] font-semibold">{t('chart.providerSplit')}</h2>
            <span className="text-[12px] text-muted">24 h</span>
          </div>
          <div className="px-4 py-3">
            <ProviderSplit logs={recentLogs} />
          </div>
        </div>
      </Card>

      <div className="grid grid-cols-1 xl:grid-cols-1">
        <RecentLogsTable logs={recentLogs} />
      </div>
    </div>
  );
}

function ProviderSplit({ logs }: { logs: Array<{ provider: string }> }) {
  const { t } = useT();
  const byProvider = useMemo(() => {
    const counts = new Map<string, number>();
    logs.forEach((l) => {
      const p = l.provider || 'Unknown';
      counts.set(p, (counts.get(p) || 0) + 1);
    });
    const total = logs.length || 1;
    return Array.from(counts.entries())
      .map(([name, count]) => ({ name, pct: Math.round((count / total) * 100) }))
      .sort((a, b) => b.pct - a.pct);
  }, [logs]);

  if (logs.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center text-center py-8 text-muted">
        <span className="text-2xl mb-2">—</span>
        <p className="text-[12px]">{t('kpi.noLiveData')}</p>
      </div>
    );
  }

  return (
    <div>
      {byProvider.map((p) => (
        <div key={p.name} className="flex items-center gap-3 py-2.5 border-b border-bg last:border-b-0">
          <div className="font-mono text-xs w-[110px] truncate flex-none">{p.name}</div>
          <div className="flex-1 h-2 rounded-full bg-bg overflow-hidden">
            <div className="h-full bg-accent" style={{ width: `${p.pct}%` }} />
          </div>
          <div className="font-mono text-[11.5px] w-11 text-right">{p.pct}%</div>
        </div>
      ))}
    </div>
  );
}
