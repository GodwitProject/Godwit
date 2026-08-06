import { useState } from 'react';
import { Modal } from '../ui/Modal';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { Checkbox } from '../ui/Checkbox';
import { Button } from '../ui/Button';
import type { ApiKey, CreateKeyRequest, CreatedKey } from '../../lib/keys';

export interface KeyFormProps {
  open: boolean;
  owners: string[];
  availableModels: string[];
  initial?: ApiKey;
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

export function KeyForm({
  open,
  owners,
  availableModels,
  initial,
  submitting,
  onClose,
  onSubmit,
}: KeyFormProps) {
  const [name, setName] = useState(initial?.name || '');
  const [owner, setOwner] = useState(initial?.owner || owners[0] || '');
  const [scopes, setScopes] = useState<string[]>(initial?.scopes || []);
  const [selectedModels, setSelectedModels] = useState<string[]>(initial?.allowedModels || []);
  const [budget, setBudget] = useState(initial?.budget != null ? String(initial.budget) : '');
  const [rateLimitRpm, setRateLimitRpm] = useState(
    initial?.rateLimitRpm != null ? String(initial.rateLimitRpm) : ''
  );
  const [rateLimitTpm, setRateLimitTpm] = useState(
    initial?.rateLimitTpm != null ? String(initial.rateLimitTpm) : ''
  );
  const [expiry, setExpiry] = useState(
    initial?.expiresAt ? initial.expiresAt.slice(0, 10) : ''
  );
  const [created, setCreated] = useState<CreatedKey | null>(null);
  const [copied, setCopied] = useState(false);

  function reset() {
    setName('');
    setOwner(owners[0] || '');
    setScopes([]);
    setSelectedModels([]);
    setBudget('');
    setRateLimitRpm('');
    setRateLimitTpm('');
    setExpiry('');
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
      owner,
      scopes,
      allowedModels: selectedModels,
      budget: toNumber(budget),
      rateLimitRpm: toNumber(rateLimitRpm),
      rateLimitTpm: toNumber(rateLimitTpm),
      expiresAt: expiry ? new Date(expiry).toISOString() : null,
    };
    const result = await onSubmit(req);
    if (result && result.fullKey) {
      setCreated(result);
    }
  }

  function handleClose() {
    onClose();
    reset();
  }

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title={initial ? `Edit Key: ${initial.name}` : 'Create API Key'}
      maxWidth="max-w-2xl"
    >
      {created ? (
        <div className="flex flex-col items-center text-center gap-4 py-6">
          <span className="material-symbols-outlined text-5xl text-success">verified</span>
          <h3 className="text-title-md">API Key created</h3>
          <div className="bg-warning/10 text-warning rounded-lg p-3 text-body-base w-full">
            Copy this key now. You won&apos;t see it again.
          </div>
          <div className="w-full bg-surface-container-low rounded-lg p-3 font-mono text-code-sm break-all select-all">
            {created.fullKey}
          </div>
          <Button
            variant="secondary"
            onClick={() => {
              navigator.clipboard?.writeText(created.fullKey);
              setCopied(true);
            }}
          >
            {copied ? 'Copied' : 'Copy Key'}
          </Button>
          <Button onClick={handleClose}>Done</Button>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="space-y-4">
          <Input
            label="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Production gateway"
            required
          />
          <Select label="Owner" value={owner} onChange={(e) => setOwner(e.target.value)}>
            {owners.map((o) => (
              <option key={o} value={o}>{o}</option>
            ))}
          </Select>

          <div className="flex flex-col gap-2">
            <span className="text-label-sm font-medium text-on-surface-variant">Scopes</span>
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
            <span className="text-label-sm font-medium text-on-surface-variant">Allowed Models</span>
            {availableModels.length === 0 ? (
              <p className="text-caption-xs text-on-surface-variant">No models available.</p>
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
              label="Budget (USD, optional)"
              type="number"
              min="0"
              step="0.01"
              value={budget}
              onChange={(e) => setBudget(e.target.value)}
              placeholder="e.g. 100"
            />
            <Input
              label="Expiry (optional)"
              type="date"
              value={expiry}
              onChange={(e) => setExpiry(e.target.value)}
            />
            <Input
              label="Rate Limit RPM (optional)"
              type="number"
              min="0"
              value={rateLimitRpm}
              onChange={(e) => setRateLimitRpm(e.target.value)}
              placeholder="e.g. 1000"
            />
            <Input
              label="Rate Limit TPM (optional)"
              type="number"
              min="0"
              value={rateLimitTpm}
              onChange={(e) => setRateLimitTpm(e.target.value)}
              placeholder="e.g. 100000"
            />
          </div>

          <div className="flex justify-end gap-3 pt-2">
            <Button type="button" variant="secondary" onClick={handleClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? 'Creating...' : initial ? 'Save Changes' : 'Create Key'}
            </Button>
          </div>
        </form>
      )}
    </Modal>
  );
}
