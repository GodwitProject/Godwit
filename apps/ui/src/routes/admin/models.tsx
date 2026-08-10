import { PageHeader } from '@/components/ui/PageHeader';
import { ModelList } from '@/components/models/ModelList';

export function AdminModels() {
  return (
    <div className="space-y-4">
      <PageHeader title="Models" description="Manage available models" />
      <ModelList />
    </div>
  );
}
