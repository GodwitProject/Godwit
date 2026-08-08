'use client';

import { useState } from 'react';
import { Modal } from '../ui/Modal';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { Button } from '../ui/Button';
import { useT } from '@/hooks/useT';
import type { Provider } from '@/lib/providers';
import type { CreateModelRequest } from '@/lib/models';

export interface ModelFormProps {
  open: boolean;
  providers: Provider[];
  submitting: boolean;
  onClose: () => void;
  onSubmit: (req: CreateModelRequest) => void | Promise<void>;
}

export function ModelForm({ open, providers, submitting, onClose, onSubmit }: ModelFormProps) {
  const { t } = useT();
  const [publicId, setPublicId] = useState('');
  const [profileId, setProfileId] = useState('');
  const [providerModelId, setProviderModelId] = useState('');
  const [capabilities, setCapabilities] = useState('chat');
  const [inputPrice, setInputPrice] = useState('');
  const [outputPrice, setOutputPrice] = useState('');

  const selectedProfile = providers.find((p) => p.id === profileId);

  function reset() {
    setPublicId('');
    setProfileId('');
    setProviderModelId('');
    setCapabilities('chat');
    setInputPrice('');
    setOutputPrice('');
  }

  function handleClose() {
    reset();
    onClose();
  }

  async function handleSubmit() {
    const profile = selectedProfile;
    if (!profile || !publicId || !providerModelId) return;
    const inPrice = parseFloat(inputPrice);
    const outPrice = parseFloat(outputPrice);
    if (!Number.isFinite(inPrice) || !Number.isFinite(outPrice)) return;
    await onSubmit({
      public_id: publicId,
      provider: profile.protocol,
      provider_profile_id: profile.id,
      provider_model_id: providerModelId,
      capabilities: capabilities || 'chat',
      pricing: {
        input_price_per_million: inPrice,
        output_price_per_million: outPrice,
      },
    });
    handleClose();
  }

  return (
    <Modal open={open} onClose={handleClose} title={t('modelForm.title')}>
      <div className="space-y-4">
        <Input
          label={t('modelForm.exposed')}
          value={publicId}
          onChange={(e) => setPublicId(e.target.value)}
          placeholder="gpt-4o"
        />
        <Select
          label={t('modelForm.providerProfile')}
          value={profileId}
          onChange={(e) => setProfileId(e.target.value)}
        >
          <option value="">{t('modelForm.selectProvider')}</option>
          {providers.map((p) => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </Select>
        <Input
          label={t('modelForm.providerSideId')}
          value={providerModelId}
          onChange={(e) => setProviderModelId(e.target.value)}
          placeholder="gpt-4o-2024-11-20"
        />
        <Input
          label={t('modelForm.capabilities')}
          value={capabilities}
          onChange={(e) => setCapabilities(e.target.value)}
          placeholder="chat"
        />
        <div className="grid grid-cols-2 gap-3">
          <Input
            label={t('modelForm.inputPrice')}
            type="number"
            min="0"
            step="0.01"
            value={inputPrice}
            onChange={(e) => setInputPrice(e.target.value)}
            placeholder="0"
          />
          <Input
            label={t('modelForm.outputPrice')}
            type="number"
            min="0"
            step="0.01"
            value={outputPrice}
            onChange={(e) => setOutputPrice(e.target.value)}
            placeholder="0"
          />
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="secondary" size="sm" onClick={handleClose} disabled={submitting}>
            {t('modelForm.cancel')}
          </Button>
          <Button size="sm" onClick={handleSubmit} disabled={submitting || !profileId || !publicId || !providerModelId}>
            {submitting ? t('modelForm.submitting') : t('modelForm.submit')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
