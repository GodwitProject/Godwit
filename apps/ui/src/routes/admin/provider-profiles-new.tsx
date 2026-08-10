import { useNavigate } from 'react-router-dom';
import { PageHeader } from '@/components/ui/PageHeader';
import { ProviderProfileForm } from '@/components/provider-profiles/ProviderProfileForm';
import { useCreateProviderProfile } from '@/hooks/useProviderProfiles';
import type { CreateProviderProfileInput } from '@/lib/providerProfiles';

export function AdminProviderProfilesNew() {
  const navigate = useNavigate();
  const mutation = useCreateProviderProfile();

  return (
    <div className="mx-auto max-w-2xl space-y-4">
      <PageHeader title="New provider profile" description="Configure a provider connection" />
      <ProviderProfileForm
        mode="create"
        onSubmit={async (data) => {
          await mutation.mutateAsync(data as CreateProviderProfileInput);
          navigate('/admin/provider-profiles');
        }}
      />
    </div>
  );
}
