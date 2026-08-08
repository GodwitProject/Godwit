'use client';

import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Button } from '@/components/ui/Button';
import { ModelsTable } from '@/components/models/ModelsTable';
import { CapacityCard } from '@/components/models/CapacityCard';
import { ProvidersCard } from '@/components/models/ProvidersCard';
import { ModelForm } from '@/components/models/ModelForm';
import { useModels, useCreateModel } from '@/hooks/useModels';
import { useProviders } from '@/hooks/useProviders';
import { useSetProviderEnabled } from '@/hooks/useSetProviderEnabled';
import { useT } from '@/hooks/useT';
import { fetchLogs } from '@/lib/logs';
import { PlusIcon } from '@/components/icons';

export default function ProvidersPage() {
  const { t } = useT();
  const [formOpen, setFormOpen] = useState(false);
  const { data: models, isLoading: modelsLoading } = useModels();
  const { data: providers, isLoading: providersLoading } = useProviders();
  const setEnabled = useSetProviderEnabled();
  const createModel = useCreateModel();
  const { data: logsPage } = useQuery({
    queryKey: ['models-recent-logs'],
    queryFn: () => fetchLogs({ limit: 200 }),
  });

  const recentLogs = logsPage?.items ?? [];

  const latencyByModel = useMemo(() => {
    const sum = new Map<string, number>();
    const count = new Map<string, number>();
    recentLogs.forEach((l) => {
      if (l.duration_ms == null) return;
      sum.set(l.model, (sum.get(l.model) ?? 0) + l.duration_ms);
      count.set(l.model, (count.get(l.model) ?? 0) + 1);
    });
    const out = new Map<string, number | null>();
    count.forEach((c, m) => {
      const s = sum.get(m) ?? 0;
      out.set(m, c > 0 ? s / c : null);
    });
    return out;
  }, [recentLogs]);

  const tokensPerMinByModel = useMemo(() => {
    const total = new Map<string, number>();
    recentLogs.forEach((l) => {
      const n = (l.tokens_in ?? 0) + (l.tokens_out ?? 0);
      total.set(l.model, (total.get(l.model) ?? 0) + n);
    });
    return new Map(Array.from(total.entries()).map(([m, v]) => [m, v]));
  }, [recentLogs]);

  const protocolEnabled = useMemo(() => {
    const set = new Set<string>();
    (providers ?? []).forEach((p) => {
      if (p.enabled) set.add(p.protocol);
      set.add(p.name);
    });
    return set;
  }, [providers]);

  function handleToggle(id: string, enabled: boolean) {
    setEnabled.mutate({ id, enabled });
  }

  const isLoading = modelsLoading && providersLoading;

  return (
    <div className="view-fade space-y-4">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b border-border pb-4">
        <div>
          <h1 className="text-display-lg">{t('page.models.title')}</h1>
          <p className="text-[13px] text-muted mt-1 max-w-[62ch]">{t('page.models.subtitle')}</p>
        </div>
        <Button onClick={() => setFormOpen(true)}>
          <PlusIcon width={14} height={14} />
          {t('models.add')}
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center gap-3 py-16 justify-center text-muted">
          <span className="animate-spin">◌</span>
          {t('loading.loading')}…
        </div>
      ) : (
        <>
          <ModelsTable
            models={models ?? []}
            latencyByModel={latencyByModel}
            protocolEnabled={protocolEnabled}
          />
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
            <CapacityCard tokensPerMinByModel={tokensPerMinByModel} />
            <ProvidersCard
              providers={providers ?? []}
              onToggle={handleToggle}
              toggling={setEnabled.isPending}
            />
          </div>
        </>
      )}

      <ModelForm
        open={formOpen}
        providers={providers ?? []}
        submitting={createModel.isPending}
        onClose={() => setFormOpen(false)}
        onSubmit={async (req) => {
          await createModel.mutateAsync(req);
          setFormOpen(false);
        }}
      />
    </div>
  );
}
