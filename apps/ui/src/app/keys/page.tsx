'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/Button';
import { KeyList } from '@/components/keys/KeyList';
import { KeyForm } from '@/components/keys/KeyForm';
import { KeyDetails } from '@/components/keys/KeyDetails';
import {
  useKeys,
  useCreateKey,
  useUpdateKey,
  useDeleteKey,
  useRevokeKey,
  useKeyUsage,
  useKeyLogs,
} from '@/hooks/useKeys';
import type { ApiKey, CreateKeyRequest } from '@/lib/keys';

const MOCK_OWNERS = ['Platform Team', 'Growth', 'Data Science'];
const MOCK_MODELS = ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo', 'claude-3-opus', 'claude-3-sonnet'];

export default function KeysPage() {
  const { data: keys, isLoading } = useKeys();
  const [createOpen, setCreateOpen] = useState(false);
  const [selected, setSelected] = useState<ApiKey | null>(null);
  const [editing, setEditing] = useState(false);

  const createMutation = useCreateKey();
  const updateMutation = useUpdateKey();
  const deleteMutation = useDeleteKey();
  const revokeMutation = useRevokeKey();

  const { data: usage } = useKeyUsage(selected?.id);
  const { data: logs } = useKeyLogs(selected?.id);

  function handleCreateSuccess() {
    setCreateOpen(false);
  }

  function handleSave(req: CreateKeyRequest) {
    if (!selected) return;
    updateMutation.mutate(
      { id: selected.id, req },
      { onSuccess: () => setEditing(false) }
    );
  }

  function handleRevoke(key: ApiKey) {
    revokeMutation.mutate(key.id);
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
            onEdit={(key) => {
              setSelected(key);
              setEditing(true);
            }}
            onRevoke={handleRevoke}
            onDelete={handleDelete}
          />
        )}
      </section>

      <KeyForm
        open={createOpen}
        owners={MOCK_OWNERS}
        availableModels={MOCK_MODELS}
        submitting={createMutation.isPending}
        onClose={() => setCreateOpen(false)}
        onSubmit={async (req) => {
          const result = await createMutation.mutateAsync(req);
          handleCreateSuccess();
          return result;
        }}
      />

      {selected && (
        <KeyDetails
          apiKey={selected}
          usage={usage}
          logs={logs}
          owners={MOCK_OWNERS}
          availableModels={MOCK_MODELS}
          editing={editing}
          onStartEdit={() => setEditing(true)}
          onClose={() => {
            setSelected(null);
            setEditing(false);
          }}
          onSave={handleSave}
        />
      )}
    </>
  );
}
