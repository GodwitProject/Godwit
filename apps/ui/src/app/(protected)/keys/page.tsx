'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { KeyList } from '@/components/keys/KeyList';
import { KeyForm } from '@/components/keys/KeyForm';
import { KeyDetails } from '@/components/keys/KeyDetails';
import { useT } from '@/hooks/useT';
import {
  useKeys,
  useCreateKey,
  useDeleteKey,
  useBlockKey,
  useUnblockKey,
} from '@/hooks/useKeys';
import type { ApiKey, CreateKeyRequest } from '@/lib/keys';
import { useModels } from '@/hooks/useModels';

export default function KeysPage() {
  const { t } = useT();
  const { data: keys, isLoading } = useKeys();
  const { data: models = [] } = useModels();
  const availableModels = models.map((m) => m.public_id);
  const [createOpen, setCreateOpen] = useState(false);
  const [selected, setSelected] = useState<ApiKey | null>(null);

  const createMutation = useCreateKey();
  const deleteMutation = useDeleteKey();
  const blockMutation = useBlockKey();
  const unblockMutation = useUnblockKey();

  function handleCreateSuccess() {
    setCreateOpen(false);
  }

  function handleToggleActive(key: ApiKey) {
    if (key.disabled) {
      unblockMutation.mutate(key.id);
    } else {
      blockMutation.mutate(key.id);
    }
  }

  function handleDelete(key: ApiKey) {
    deleteMutation.mutate(key.id, {
      onSuccess: () => {
        if (selected?.id === key.id) setSelected(null);
      },
    });
  }

  return (
    <div className="view-fade space-y-4">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b border-border pb-4">
        <div>
          <h1 className="text-display-lg">{t('page.keys.title')}</h1>
          <p className="text-[13px] text-muted mt-1 max-w-[62ch]">{t('page.keys.subtitle')}</p>
        </div>
        <Button onClick={() => setCreateOpen(true)}>{t('keys.new')}</Button>
      </div>

      {isLoading ? (
        <div className="flex items-center gap-3 py-16 justify-center text-muted">
          <span className="animate-spin">◌</span>
          {t('loading.loading')}…
        </div>
      ) : (
        <KeyList
          keys={keys || []}
          onSelect={setSelected}
          onToggleActive={handleToggleActive}
          onDelete={handleDelete}
        />
      )}

      <KeyForm
        open={createOpen}
        availableModels={availableModels}
        submitting={createMutation.isPending}
        onClose={() => setCreateOpen(false)}
        onSubmit={async (req: CreateKeyRequest) => {
          const result = await createMutation.mutateAsync(req);
          handleCreateSuccess();
          return result;
        }}
      />

      {selected && (
        <KeyDetails
          apiKey={selected}
          onToggleActive={() => handleToggleActive(selected)}
          onDelete={() => handleDelete(selected)}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
