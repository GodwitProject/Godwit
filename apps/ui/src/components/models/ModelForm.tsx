import { useState } from 'react';
import { useForm, type DefaultValues } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Checkbox } from '@/components/ui/Checkbox';
import {
  createModelSchema,
  updateModelSchema,
  type CreateModelInput,
  type UpdateModelInput,
} from '@/lib/models';
import { useProviderProfiles } from '@/hooks/useProviderProfiles';

const CAPABILITIES = [
  { value: 'chat', label: 'Chat' },
  { value: 'embedding', label: 'Embedding' },
  { value: 'vision', label: 'Vision' },
  { value: 'tool_calling', label: 'Tool calling' },
];

type FormValues = {
  public_id: string;
  capabilities: string[];
  provider_profile_id?: string;
  provider_model_id?: string;
  input_price_per_million?: number;
  output_price_per_million?: number;
};

interface ModelFormProps {
  mode: 'create' | 'edit';
  defaultValues?: Partial<FormValues>;
  onSubmit: (data: CreateModelInput | UpdateModelInput) => Promise<void>;
  onCancel?: () => void;
}

export function ModelForm({ mode, defaultValues, onSubmit, onCancel }: ModelFormProps) {
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { data: profiles = [] } = useProviderProfiles();
  const schema = mode === 'create' ? createModelSchema.omit({ provider: true }) : updateModelSchema;
  const {
    register,
    handleSubmit,
    setValue,
    watch,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      public_id: '',
      provider_profile_id: '',
      provider_model_id: '',
      capabilities: ['chat'],
      input_price_per_million: 0,
      output_price_per_million: 0,
      ...defaultValues,
    } as DefaultValues<FormValues>,
  });

  const profileId = watch('provider_profile_id');
  const capabilities = watch('capabilities') ?? [];

  const handleFormSubmit = handleSubmit(async (data) => {
    setBusy(true);
    setSubmitError(null);
    try {
      if (mode === 'create') {
        const profile = profiles.find((p) => p.id === data.provider_profile_id);
        await onSubmit({
          public_id: data.public_id,
          provider_profile_id: data.provider_profile_id ?? '',
          provider_model_id: data.provider_model_id ?? '',
          provider: profile?.protocol ?? '',
          capabilities: data.capabilities,
          input_price_per_million: data.input_price_per_million ?? 0,
          output_price_per_million: data.output_price_per_million ?? 0,
        } as CreateModelInput);
      } else {
        await onSubmit(data as UpdateModelInput);
      }
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : 'Submission failed');
    } finally {
      setBusy(false);
    }
  });

  return (
    <form onSubmit={handleFormSubmit} className="space-y-4">
      <Input
        label="Public ID"
        placeholder="gpt-4o"
        error={errors.public_id?.message}
        {...register('public_id')}
      />
      {mode === 'create' && (
        <Select
          label="Provider profile"
          value={profileId ?? ''}
          placeholder="Select a profile"
          options={profiles.map((p) => ({ value: p.id, label: p.name }))}
          onChange={(value) => setValue('provider_profile_id', value, { shouldValidate: true })}
          error={errors.provider_profile_id?.message}
        />
      )}
      {mode === 'create' && (
        <Input
          label="Provider model ID"
          placeholder="gpt-4o"
          error={errors.provider_model_id?.message}
          {...register('provider_model_id')}
        />
      )}
      <fieldset className="space-y-2">
        <legend className="text-body-md text-on-surface">Capabilities</legend>
        <div className="flex flex-wrap gap-4">
          {CAPABILITIES.map((cap) => (
            <Checkbox
              key={cap.value}
              label={cap.label}
              checked={capabilities.includes(cap.value)}
              onChange={(checked) => {
                const next = checked
                  ? [...capabilities, cap.value]
                  : capabilities.filter((c) => c !== cap.value);
                setValue('capabilities', next, { shouldValidate: true });
              }}
            />
          ))}
        </div>
        {errors.capabilities && <span className="text-body-sm text-danger">{errors.capabilities.message}</span>}
      </fieldset>
      {mode === 'create' && (
        <>
          <Input
            label="Input price per million tokens"
            type="number"
            step="0.01"
            error={errors.input_price_per_million?.message}
            {...register('input_price_per_million', { valueAsNumber: true })}
          />
          <Input
            label="Output price per million tokens"
            type="number"
            step="0.01"
            error={errors.output_price_per_million?.message}
            {...register('output_price_per_million', { valueAsNumber: true })}
          />
        </>
      )}
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
