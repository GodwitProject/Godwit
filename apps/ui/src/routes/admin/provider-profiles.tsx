import { PageHeader } from '@/components/ui/PageHeader';
import { ProviderProfileList } from '@/components/provider-profiles/ProviderProfileList';

export function AdminProviderProfiles() {
  return (
    <div className="space-y-4">
      <PageHeader title="Provider Profiles" description="Configure provider profiles" />
      <ProviderProfileList />
    </div>
  );
}
