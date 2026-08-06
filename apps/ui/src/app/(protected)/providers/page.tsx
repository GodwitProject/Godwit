'use client';

import { ProviderList } from '@/components/providers/ProviderList';
import { useProviders } from '@/hooks/useProviders';

export default function ProvidersPage() {
  const { data: providers, isLoading } = useProviders();

  return (
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">Providers</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">
            Configure LLM providers and fallback chains.
          </p>
        </div>
      </div>

      <section>
        {isLoading ? (
          <div className="flex items-center gap-3 py-16 justify-center text-on-surface-variant">
            <span className="material-symbols-outlined animate-spin">progress_activity</span>
            Loading providers...
          </div>
        ) : (
          <ProviderList providers={providers || []} />
        )}
      </section>
    </>
  );
}
