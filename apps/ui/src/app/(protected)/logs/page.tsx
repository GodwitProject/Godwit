'use client';

import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Toggle } from '@/components/ui/Toggle';
import { LogFilters } from '@/components/logs/LogFilters';
import { LogsTable } from '@/components/logs/LogsTable';
import { LogDetail } from '@/components/logs/LogDetail';
import { useLogs } from '@/hooks/useLogs';
import { useT } from '@/hooks/useT';
import { ExportIcon } from '@/components/icons';
import type { LogFilters as LogFiltersType } from '@/lib/logs';

const MODELS = ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo', 'claude-3-opus', 'claude-3-sonnet'];

function freshFilters(): LogFiltersType {
  return {};
}

export default function LogsPage() {
  const { t } = useT();
  const [filters, setFilters] = useState<LogFiltersType>(freshFilters);
  const [draftFilters, setDraftFilters] = useState<LogFiltersType>(freshFilters);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [liveTail, setLiveTail] = useState(false);

  const {
    data,
    isLoading,
    isFetchingNextPage,
    hasNextPage,
    fetchNextPage,
  } = useLogs(filters);

  const logs = useMemo(() => data?.pages.flatMap((page) => page.items) ?? [], [data]);
  const selected = useMemo(() => logs.find((log) => log.id === selectedId) ?? null, [logs, selectedId]);

  function handleApply() {
    setFilters(Object.assign({}, draftFilters));
  }

  function handleClear() {
    setDraftFilters(freshFilters());
    setFilters(freshFilters());
  }

  return (
    <div className="view-fade space-y-4">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b border-border pb-4">
        <div>
          <h1 className="text-display-lg">{t('page.traffic.title')}</h1>
          <p className="text-[13px] text-muted mt-1 max-w-[62ch]">{t('page.traffic.subtitle')}</p>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="secondary" size="sm" disabled title="Coming soon">
            <ExportIcon width={14} height={14} />
            {t('traffic.exportCsv')}
          </Button>
          <Toggle checked={liveTail} onChange={(e) => setLiveTail(e.target.checked)} label={t('logs.liveTail')} />
        </div>
      </div>

      <LogFilters
        filters={draftFilters}
        models={MODELS}
        onChange={setDraftFilters}
        onApply={handleApply}
        onClear={handleClear}
      />

      {isLoading ? (
        <div className="flex items-center gap-3 py-16 justify-center text-muted">
          <span className="animate-spin">◌</span>
          {t('loading.loading')}…
        </div>
      ) : (
        <LogsTable
          logs={logs}
          hasMore={!!hasNextPage}
          onLoadMore={() => fetchNextPage()}
          loadingMore={isFetchingNextPage}
          onSelect={(log) => setSelectedId(log.id)}
        />
      )}

      <LogDetail open={!!selectedId} log={selected || undefined} onClose={() => setSelectedId(null)} />
    </div>
  );
}
