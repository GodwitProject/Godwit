import { useState } from 'react';
import { Modal } from '../ui/Modal';
import { Input } from '../ui/Input';
import { Checkbox } from '../ui/Checkbox';
import { Button } from '../ui/Button';
import { useT } from '@/hooks/useT';
import type { CreateKeyRequest, CreatedKey } from '../../lib/keys';

export interface KeyFormProps {
  open: boolean;
  availableModels: string[];
  submitting?: boolean;
  onClose: () => void;
  onSubmit: (req: CreateKeyRequest) => Promise<CreatedKey | void> | CreatedKey | void;
}

const ALL_SCOPES = ['read', 'write', 'admin'];

function toNumber(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const n = Number(trimmed);
  return Number.isFinite(n) ? n : null;
}

export function KeyForm({ open, availableModels, submitting, onClose, onSubmit }: KeyFormProps) {
  const { t } = useT();
  const [name, setName] = useState('');
  const [scopes, setScopes] = useState<string[]>([]);
  const [selectedModels, setSelectedModels] = useState<string[]>([]);
  const [rateLimitRpm, setRateLimitRpm] = useState('');
  const [rateLimitTpm, setRateLimitTpm] = useState('');
  const [created, setCreated] = useState<CreatedKey | null>(null);
  const [copied, setCopied] = useState(false);

  function reset() {
    setName('');
    setScopes([]);
    setSelectedModels([]);
    setRateLimitRpm('');
    setRateLimitTpm('');
    setCreated(null);
    setCopied(false);
  }

  function toggleScope(scope: string) {
    setScopes((prev) =>
      prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope]
    );
  }

  function toggleModel(model: string) {
    setSelectedModels((prev) =>
      prev.includes(model) ? prev.filter((m) => m !== model) : [...prev, model]
    );
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const req: CreateKeyRequest = {
      name,
      scopes,
      allowed_models: selectedModels,
      rate_limit_requests_per_minute: toNumber(rateLimitRpm),
      rate_limit_tokens_per_minute: toNumber(rateLimitTpm),
    };
    const result = await onSubmit(req);
    if (result && result.key) {
      setCreated(result);
    }
  }

  function handleClose() {
    onClose();
    reset();
  }

  return (
    <Modal open={open} onClose={handleClose} title={t('keys.create.title')} maxWidth="max-w-2xl">
      {created ? (
        <div className="flex flex-col items-center text-center gap-4 py-6">
          <span className="text-5xl text-success">✓</span>
          <h3 className="text-base font-semibold">{t('keys.created.title')}</h3>
          <div className="bg-[oklch(97%_0.04_80)] text-warn rounded-lg p-3 text-[13px] w-full">
            {t('keys.created.note')}
          </div>
          <div className="w-full bg-bg border border-border rounded-lg p-3 font-mono text-[11.5px] break-all select-all">
            {created.key}
          </div>
          <Button
            variant="secondary"
            onClick={() => {
              navigator.clipboard?.writeText(created.key);
              setCopied(true);
            }}
          >
            {copied ? t('keys.created.copied') : t('keys.created.copy')}
          </Button>
          <Button onClick={handleClose}>{t('keys.created.done')}</Button>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="space-y-4">
          <Input
            label={t('keys.create.nameLabel')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Production gateway"
            required
          />

          <div className="flex flex-col gap-2">
            <span className="text-[11px] uppercase tracking-wider text-muted font-medium">{t('keys.create.scopes')}</span>
            <div className="flex flex-wrap gap-4">
              {ALL_SCOPES.map((scope) => (
                <Checkbox
                  key={scope}
                  label={scope}
                  checked={scopes.includes(scope)}
                  onChange={() => toggleScope(scope)}
                />
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <span className="text-[11px] uppercase tracking-wider text-muted font-medium">{t('keys.create.allowedModels')}</span>
            {availableModels.length === 0 ? (
              <p className="text-[11px] text-muted">—</p>
            ) : (
              <div className="flex flex-wrap gap-3">
                {availableModels.map((model) => (
                  <Checkbox
                    key={model}
                    label={model}
                    checked={selectedModels.includes(model)}
                    onChange={() => toggleModel(model)}
                  />
                ))}
              </div>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Input
              label={t('keys.create.rateLimitRpm')}
              type="number"
              min="0"
              value={rateLimitRpm}
              onChange={(e) => setRateLimitRpm(e.target.value)}
              placeholder="e.g. 1000"
            />
            <Input
              label={t('keys.create.rateLimitTpm')}
              type="number"
              min="0"
              value={rateLimitTpm}
              onChange={(e) => setRateLimitTpm(e.target.value)}
              placeholder="e.g. 100000"
            />
          </div>

          <div className="flex justify-end gap-3 pt-2">
            <Button type="button" variant="secondary" onClick={handleClose}>
              {t('keys.create.cancel')}
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? t('loading.loading') + '…' : t('keys.create.submit')}
            </Button>
          </div>
        </form>
      )}
    </Modal>
  );
}
