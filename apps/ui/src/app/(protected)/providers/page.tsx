'use client';

import { ProviderList } from '@/components/providers/ProviderList';
import { useProviders } from '@/hooks/useProviders';
import { useT } from '@/hooks/useT';

export default function ProvidersPage() {
  const { t } = useT();
  const { data: providers, isLoading } = useProviders();

  return (
    <div className="view-fade space-y-4">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b border-border pb-4">
        <div>
          <h1 className="text-display-lg">{t('page.providers.title')}</h1>
          <p className="text-[13px] text-muted mt-1 max-w-[62ch]">{t('page.providers.subtitle')}</p>
        </div>
      </div>

      {isLoading ? (
        <div className="flex items-center gap-3 py-16 justify-center text-muted">
          <span className="animate-spin">◌</span>
          {t('loading.loading')}…
        </div>
      ) : (
        <ProviderList providers={providers || []} />
      )}
    </div>
  );
}
