'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { KeyList } from '@/components/keys/KeyList';
import { KeyForm } from '@/components/keys/KeyForm';
import { KeyDetails } from '@/components/keys/KeyDetails';
import {
  useKeys,
  useCreateKey,
  useDeleteKey,
  useBlockKey,
  useUnblockKey,
} from '@/hooks/useKeys';
import type { ApiKey, CreateKeyRequest } from '@/lib/keys';

const MOCK_MODELS = ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo', 'claude-3-opus', 'claude-3-sonnet'];

export default function KeysPage() {
  const { data: keys, isLoading } = useKeys();
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
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">API Keys</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">
            Manage API keys used to authenticate requests to the proxy.
          </p>
        </div>
        <Button onClick={() => setCreateOpen(true)}>
          <span className="material-symbols-outlined mr-1">add</span>
          Create Key
        </Button>
      </div>

      <section>
        {isLoading ? (
          <div className="flex items-center gap-3 py-16 justify-center text-on-surface-variant">
            <span className="material-symbols-outlined animate-spin">progress_activity</span>
            Loading keys...
          </div>
        ) : (
          <KeyList
            keys={keys || []}
            onSelect={setSelected}
            onToggleActive={handleToggleActive}
            onDelete={handleDelete}
          />
        )}
      </section>

      <KeyForm
        open={createOpen}
        availableModels={MOCK_MODELS}
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
    </>
  );
}
