'use client';

import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Toggle } from '@/components/ui/Toggle';
import { LogFilters } from '@/components/logs/LogFilters';
import { LogsTable } from '@/components/logs/LogsTable';
import { LogDetail } from '@/components/logs/LogDetail';
import { useLogs } from '@/hooks/useLogs';
import type { LogFilters as LogFiltersType } from '@/lib/logs';

const MODELS = ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo', 'claude-3-opus', 'claude-3-sonnet'];

function freshFilters(): LogFiltersType {
  return {};
}

export default function LogsPage() {
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
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">Request Logs</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">
            Inspect proxy request history.
          </p>
        </div>
        <div className="flex items-center gap-4">
          <Button variant="secondary" size="sm" disabled title="Export coming in a later release">
            Export
          </Button>
          <Toggle
            checked={liveTail}
            onChange={(e) => setLiveTail(e.target.checked)}
            label="Live Tail"
          />
        </div>
      </div>

      <section className="space-y-4">
        <LogFilters
          filters={draftFilters}
          models={MODELS}
          onChange={setDraftFilters}
          onApply={handleApply}
          onClear={handleClear}
        />

        {isLoading ? (
          <div className="flex items-center gap-3 py-16 justify-center text-on-surface-variant">
            <span className="material-symbols-outlined animate-spin">progress_activity</span>
            Loading logs...
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
      </section>

      <LogDetail
        open={!!selectedId}
        log={selected || undefined}
        onClose={() => setSelectedId(null)}
      />
    </>
  );
}
