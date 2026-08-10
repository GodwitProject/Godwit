import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Checkbox } from '@/components/ui/Checkbox';
import {
  createProviderProfileSchema,
  updateProviderProfileSchema,
  type CreateProviderProfileInput,
  type UpdateProviderProfileInput,
} from '@/lib/providerProfiles';

const PROTOCOLS = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'azure_openai', label: 'Azure OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'google', label: 'Google' },
  { value: 'custom', label: 'Custom' },
];

type FormValues = {
  name?: string;
  protocol?: CreateProviderProfileInput['protocol'];
  base_url?: string;
  api_key?: string;
  allow_wildcard?: boolean;
  enabled?: boolean;
};

interface ProviderProfileFormProps {
  mode: 'create' | 'edit';
  defaultValues?: Partial<FormValues>;
  onSubmit: (data: CreateProviderProfileInput | UpdateProviderProfileInput) => Promise<void>;
  onCancel?: () => void;
}

export function ProviderProfileForm({ mode, defaultValues, onSubmit, onCancel }: ProviderProfileFormProps) {
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const schema = mode === 'create' ? createProviderProfileSchema : updateProviderProfileSchema;
  const {
    register,
    handleSubmit,
    setValue,
    watch,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: '',
      protocol: 'openai',
      base_url: '',
      api_key: '',
      allow_wildcard: false,
      enabled: true,
      ...defaultValues,
    },
  });

  const protocol = watch('protocol');
  const allowWildcard = watch('allow_wildcard');
  const enabled = watch('enabled');

  const handleFormSubmit = handleSubmit(async (data) => {
    setBusy(true);
    setSubmitError(null);
    try {
      await onSubmit(data);
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : 'Submission failed');
    } finally {
      setBusy(false);
    }
  });

  return (
    <form onSubmit={handleFormSubmit} className="space-y-4">
      {mode === 'create' && (
        <Input
          label="Name"
          error={errors.name?.message}
          {...register('name')}
        />
      )}
      {mode === 'create' && (
        <Select
          label="Protocol"
          value={protocol ?? 'openai'}
          options={PROTOCOLS}
          onChange={(value) => setValue('protocol', value as CreateProviderProfileInput['protocol'], { shouldValidate: true })}
          error={errors.protocol?.message}
        />
      )}
      <Input
        label="Base URL"
        placeholder="https://api.example.com/v1"
        error={errors.base_url?.message}
        {...register('base_url')}
      />
      <Input
        label="API key"
        type="password"
        placeholder={mode === 'edit' ? 'Leave blank to keep existing' : ''}
        error={errors.api_key?.message}
        {...register('api_key')}
      />
      <Checkbox
        label="Allow wildcard models"
        checked={!!allowWildcard}
        onChange={(checked) => setValue('allow_wildcard', checked)}
        error={errors.allow_wildcard?.message}
      />
      <Checkbox
        label="Enabled"
        checked={!!enabled}
        onChange={(checked) => setValue('enabled', checked)}
        error={errors.enabled?.message}
      />
      {submitError && <p className="text-body-sm text-danger">{submitError}</p>}
      <div className="flex justify-end gap-3">
        {onCancel && (
          <Button type="button" variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
        )}
        <Button type="submit" disabled={busy}>
          {busy ? 'Saving…' : mode === 'create' ? 'Create' : 'Save'}
        </Button>
      </div>
    </form>
  );
}
