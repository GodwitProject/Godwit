import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/Button';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '@/components/ui/Table';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { ModelForm } from './ModelForm';
import { useModels, useUpdateModel, useDeleteModel } from '@/hooks/useModels';
import type { UpdateModelInput } from '@/lib/models';
import type { Model } from '@/types';

export function ModelList() {
  const { data: models = [], isLoading } = useModels();
  const updateMutation = useUpdateModel();
  const deleteMutation = useDeleteModel();
  const [editing, setEditing] = useState<Model | null>(null);
  const [deleting, setDeleting] = useState<Model | null>(null);

  if (isLoading) return <p className="text-on-surface-variant">Loading…</p>;

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Link
          to="/admin/models/new"
          className="inline-flex items-center justify-center rounded bg-primary px-4 py-2 text-body-base font-medium text-on-primary hover:bg-primary/90"
        >
          New model
        </Link>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Public ID</TableHeadCell>
            <TableHeadCell>Provider</TableHeadCell>
            <TableHeadCell>Provider Model ID</TableHeadCell>
            <TableHeadCell>Capabilities</TableHeadCell>
            <TableHeadCell>Pricing (input / output)</TableHeadCell>
            <TableHeadCell>Actions</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {models.map((model) => (
            <TableRow key={model.id}>
              <TableCell>{model.public_id}</TableCell>
              <TableCell>{model.provider}</TableCell>
              <TableCell>{model.provider_model_id}</TableCell>
              <TableCell>{model.capabilities.join(', ')}</TableCell>
              <TableCell>
                {model.pricing.input_price_per_million} / {model.pricing.output_price_per_million}
              </TableCell>
              <TableCell>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => setEditing(model)}>
                    Edit
                  </Button>
                  <Button variant="danger" size="sm" onClick={() => setDeleting(model)}>
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
            <h2 className="mb-4 text-headline-sm">Edit model</h2>
            <ModelForm
              mode="edit"
              defaultValues={{
                public_id: editing.public_id,
                capabilities: editing.capabilities,
              }}
              onSubmit={async (input) => {
                await updateMutation.mutateAsync({ id: editing.id, input: input as UpdateModelInput });
                setEditing(null);
              }}
              onCancel={() => setEditing(null)}
            />
          </div>
        </div>
      )}

      <ConfirmDialog
        open={!!deleting}
        title="Delete model"
        description={`Are you sure you want to delete "${deleting?.public_id}"?`}
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
