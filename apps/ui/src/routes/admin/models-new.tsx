import { useNavigate } from 'react-router-dom';
import { PageHeader } from '@/components/ui/PageHeader';
import { ModelForm } from '@/components/models/ModelForm';
import { useCreateModel } from '@/hooks/useModels';
import type { CreateModelInput } from '@/lib/models';

export function AdminModelsNew() {
  const navigate = useNavigate();
  const mutation = useCreateModel();

  return (
    <div className="mx-auto max-w-2xl space-y-4">
      <PageHeader title="New model" description="Create a public model alias" />
      <ModelForm
        mode="create"
        onSubmit={async (data) => {
          await mutation.mutateAsync(data as CreateModelInput);
          navigate('/admin/models');
        }}
      />
    </div>
  );
}
