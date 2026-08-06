'use client';

import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Toggle } from '@/components/ui/Toggle';
import { LogFilters } from '@/components/logs/LogFilters';
import { LogsTable } from '@/components/logs/LogsTable';
import { LogDetail } from '@/components/logs/LogDetail';
import { useLogs, useLog } from '@/hooks/useLogs';
import type { LogFilters as LogFiltersType } from '@/lib/logs';

const MODELS = ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo', 'claude-3-opus', 'claude-3-sonnet'];

const EMPTY_FILTERS: LogFiltersType = {};

export default function LogsPage() {
  const [filters, setFilters] = useState<LogFiltersType>(EMPTY_FILTERS);
  const [draftFilters, setDraftFilters] = useState<LogFiltersType>(EMPTY_FILTERS);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(50);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [liveTail, setLiveTail] = useState(false);

  const { data: logs, isLoading } = useLogs(filters, page, pageSize);
  const { data: selected } = useLog(selectedId || undefined);

  const total = useMemo(() => logs?.length ?? 0, [logs]);

  function handleApply() {
    setFilters(draftFilters);
    setPage(1);
  }

  function handleClear() {
    setDraftFilters(EMPTY_FILTERS);
    setFilters(EMPTY_FILTERS);
    setPage(1);
  }

  return (
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">Request Logs</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">
            Inspect proxy request history, bodies and guardrail checks.
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
            logs={logs || []}
            total={total}
            page={page}
            pageSize={pageSize}
            onPageChange={setPage}
            onSelect={(log) => setSelectedId(log.id)}
          />
        )}
      </section>

      <LogDetail
        open={!!selectedId}
        log={selected}
        onClose={() => setSelectedId(null)}
      />
    </>
  );
}
