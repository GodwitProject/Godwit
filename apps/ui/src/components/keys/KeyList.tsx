import { useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import { Toggle } from '../ui/Toggle';
import type { ApiKey } from '../../lib/keys';

export interface KeyListProps {
  keys: ApiKey[];
  onSelect: (key: ApiKey) => void;
  onToggleActive: (key: ApiKey) => void;
  onDelete: (key: ApiKey) => void;
}

const SCOPE_VARIANTS: Record<string, 'default' | 'success' | 'info' | 'warning' | 'error'> = {
  'read': 'info',
  'write': 'success',
  'admin': 'warning',
};

function scopeVariant(scope: string) {
  return SCOPE_VARIANTS[scope] || 'default';
}

function prefixLabel(key: ApiKey) {
  return key.key_prefix || 'sk_live_****';
}

function formatRateLimit(value: number | null): string {
  return value != null ? String(value) : '—';
}

function formatExpiry(iso: string | null): string {
  if (!iso) return 'Never';
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return iso;
  }
}

export function KeyList({ keys, onSelect, onToggleActive, onDelete }: KeyListProps) {
  const [menuFor, setMenuFor] = useState<string | null>(null);

  if (keys.length === 0) {
    return (
      <Card>
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="material-symbols-outlined text-4xl text-on-surface-variant mb-2">vpn_key</span>
          <p className="text-body-base text-on-surface-variant">No API keys created yet.</p>
        </div>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <div className="p-container-padding border-b hairline-border">
        <h3 className="text-section-sm">API Keys</h3>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Name</TableHeadCell>
            <TableHeadCell>Prefix</TableHeadCell>
            <TableHeadCell>Scopes</TableHeadCell>
            <TableHeadCell>Spent</TableHeadCell>
            <TableHeadCell>Rate Limit</TableHeadCell>
            <TableHeadCell>Expires</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell />
          </TableRow>
        </TableHead>
        <TableBody>
          {keys.map((key) => {
            const active = !key.disabled;
            return (
              <TableRow
                key={key.id}
                className="cursor-pointer"
                onClick={() => {
                  setMenuFor(null);
                  onSelect(key);
                }}
              >
                <TableCell>
                  <div className="flex items-center gap-3">
                    <span className="material-symbols-outlined text-on-surface-variant">vpn_key</span>
                    <span className="font-medium">{key.name}</span>
                  </div>
                </TableCell>
                <TableCell className="font-mono text-code-sm">{prefixLabel(key)}</TableCell>
                <TableCell>
                  <div className="flex flex-wrap gap-1.5">
                    {key.scopes.map((scope) => (
                      <Badge key={scope} variant={scopeVariant(scope)}>{scope}</Badge>
                    ))}
                  </div>
                </TableCell>
                <TableCell className="font-mono text-code-sm">
                  {key.budget_spent_usd != null ? `$${key.budget_spent_usd.toFixed(2)}` : '—'}
                </TableCell>
                <TableCell className="font-mono text-code-sm">
                  {formatRateLimit(key.rate_limit_requests_per_minute)} RPM
                </TableCell>
                <TableCell className="text-on-surface-variant">{formatExpiry(key.expires_at)}</TableCell>
                <TableCell>
                  <Toggle
                    checked={active}
                    onChange={(e) => {
                      e.stopPropagation();
                      onToggleActive(key);
                    }}
                    label={active ? 'Active' : 'Revoked'}
                  />
                </TableCell>
                <TableCell>
                  <div className="relative" onClick={(e) => e.stopPropagation()}>
                    <button
                      type="button"
                      className="material-symbols-outlined p-1 rounded-full hover:bg-surface-container-high text-on-surface-variant"
                      onClick={() => setMenuFor(menuFor === key.id ? null : key.id)}
                      aria-label={`Actions for ${key.name}`}
                    >
                      more_vert
                    </button>
                    {menuFor === key.id && (
                      <div className="absolute right-0 mt-1 z-20 bg-surface-container-lowest rounded-xl shadow-lg hairline-border p-1 w-40">
                        <button
                          type="button"
                          className="w-full text-left px-3 py-2 rounded-lg text-body-base text-error hover:bg-error/10"
                          onClick={() => {
                            setMenuFor(null);
                            onDelete(key);
                          }}
                        >
                          Delete
                        </button>
                      </div>
                    )}
                  </div>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </Card>
  );
}
