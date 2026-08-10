import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/Button';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '@/components/ui/Table';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { ProviderProfileForm } from './ProviderProfileForm';
import {
  useProviderProfiles,
  useUpdateProviderProfile,
  useDeleteProviderProfile,
} from '@/hooks/useProviderProfiles';
import type { UpdateProviderProfileInput } from '@/lib/providerProfiles';
import type { ProviderProfile } from '@/types';

export function ProviderProfileList() {
  const { data: profiles = [], isLoading } = useProviderProfiles();
  const updateMutation = useUpdateProviderProfile();
  const deleteMutation = useDeleteProviderProfile();
  const [editing, setEditing] = useState<ProviderProfile | null>(null);
  const [deleting, setDeleting] = useState<ProviderProfile | null>(null);

  if (isLoading) return <p className="text-on-surface-variant">Loading…</p>;

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Link
          to="/admin/provider-profiles/new"
          className="inline-flex items-center justify-center rounded bg-primary px-4 py-2 text-body-base font-medium text-on-primary hover:bg-primary/90"
        >
          New provider profile
        </Link>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Name</TableHeadCell>
            <TableHeadCell>Protocol</TableHeadCell>
            <TableHeadCell>Base URL</TableHeadCell>
            <TableHeadCell>Credentials</TableHeadCell>
            <TableHeadCell>Enabled</TableHeadCell>
            <TableHeadCell>Actions</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {profiles.map((profile) => (
            <TableRow key={profile.id}>
              <TableCell>{profile.name}</TableCell>
              <TableCell>{profile.protocol}</TableCell>
              <TableCell>{profile.base_url ?? '-'}</TableCell>
              <TableCell>{profile.has_credentials ? 'Set' : 'None'}</TableCell>
              <TableCell>{profile.enabled ? 'Yes' : 'No'}</TableCell>
              <TableCell>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => setEditing(profile)}>
                    Edit
                  </Button>
                  <Button variant="danger" size="sm" onClick={() => setDeleting(profile)}>
                    Delete
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {editing && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="w-full max-w-lg rounded-xl bg-surface p-6 shadow-lg">
            <h2 className="mb-4 text-headline-sm">Edit provider profile</h2>
            <ProviderProfileForm
              mode="edit"
              defaultValues={{
                base_url: editing.base_url ?? '',
                allow_wildcard: editing.allow_wildcard,
                enabled: editing.enabled,
              }}
              onSubmit={async (input) => {
                await updateMutation.mutateAsync({ id: editing.id, input: input as UpdateProviderProfileInput });
                setEditing(null);
              }}
              onCancel={() => setEditing(null)}
            />
          </div>
        </div>
      )}

      <ConfirmDialog
        open={!!deleting}
        title="Delete provider profile"
        description={`Are you sure you want to delete "${deleting?.name}"? This is blocked if models still reference it.`}
        confirmLabel="Delete"
        destructive
        onConfirm={async () => {
          if (deleting) {
            await deleteMutation.mutateAsync(deleting.id);
            setDeleting(null);
          }
        }}
        onCancel={() => setDeleting(null)}
      />
    </div>
  );
}
