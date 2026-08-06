import { Modal } from '../ui/Modal';
import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';
import type { ApiKey } from '../../lib/keys';

export interface KeyDetailsProps {
  apiKey: ApiKey;
  onClose: () => void;
  onToggleActive: () => void;
  onDelete: () => void;
}

function formatNullable(value: number | null, suffix = ''): string {
  return value != null ? `${value}${suffix}` : '—';
}

function formatExpiry(iso: string | null): string {
  if (!iso) return 'Never';
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return iso;
  }
}

export function KeyDetails({ apiKey, onClose, onToggleActive, onDelete }: KeyDetailsProps) {
  const active = !apiKey.disabled;
  return (
    <Modal open onClose={onClose} title={apiKey.name} maxWidth="max-w-3xl">
      <div className="space-y-6">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Badge variant={active ? 'success' : 'error'}>
              {active ? 'Active' : 'Revoked'}
            </Badge>
            <span className="font-mono text-code-sm text-on-surface-variant">{apiKey.key_prefix || 'sk_live_****'}</span>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="secondary" size="sm" onClick={onToggleActive}>
              {active ? 'Revoke' : 'Unblock'}
            </Button>
            <Button variant="secondary" size="sm" onClick={onDelete}>
              Delete
            </Button>
            <Button variant="secondary" size="sm" onClick={onClose}>
              Close
            </Button>
          </div>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <Card variant="filled" className="p-4">
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Spent</p>
            <p className="font-mono text-title-md mt-1">
              {apiKey.budget_spent_usd != null ? `$${apiKey.budget_spent_usd.toFixed(2)}` : '—'}
            </p>
          </Card>
          <Card variant="filled" className="p-4">
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Budget</p>
            <p className="font-mono text-title-md mt-1">
              {apiKey.budget_limit_usd != null ? `$${apiKey.budget_limit_usd.toFixed(2)}` : 'Unlimited'}
            </p>
          </Card>
          <Card variant="filled" className="p-4">
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Rate Limit (RPM)</p>
            <p className="font-mono text-title-md mt-1">{formatNullable(apiKey.rate_limit_requests_per_minute)}</p>
          </Card>
          <Card variant="filled" className="p-4">
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Rate Limit (TPM)</p>
            <p className="font-mono text-title-md mt-1">{formatNullable(apiKey.rate_limit_tokens_per_minute)}</p>
          </Card>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <span className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Scopes</span>
          {apiKey.scopes.length === 0 ? (
            <span className="text-body-base text-on-surface-variant">—</span>
          ) : (
            apiKey.scopes.map((scope) => <Badge key={scope} variant="info">{scope}</Badge>)
          )}
        </div>

        <div>
          <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider mb-2">Allowed Models</p>
          {apiKey.allowed_models.length === 0 ? (
            <p className="text-body-base text-on-surface-variant">No models restricted (all allowed).</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {apiKey.allowed_models.map((model) => (
                <Badge key={model} variant="default" className="font-mono text-code-sm">{model}</Badge>
              ))}
            </div>
          )}
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Expires</p>
            <p className="font-mono text-code-sm mt-1">{formatExpiry(apiKey.expires_at)}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Created</p>
            <p className="font-mono text-code-sm mt-1">{apiKey.created_at || '—'}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Organization</p>
            <p className="font-mono text-code-sm mt-1">{apiKey.organization_id || '—'}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">User</p>
            <p className="font-mono text-code-sm mt-1">{apiKey.user_id || '—'}</p>
          </div>
        </div>

        <div className="bg-surface-container-low p-container-padding rounded-lg">
          <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider mb-2">Spend Trend</p>
          <p className="text-body-base text-on-surface-variant">
            No live spend series yet.
          </p>
        </div>
      </div>
    </Modal>
  );
}
