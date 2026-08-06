import { useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import { Toggle } from '../ui/Toggle';
import type { ApiKey } from '../../lib/keys';

export interface KeyListProps {
  keys: ApiKey[];
  onSelect: (key: ApiKey) => void;
  onEdit: (key: ApiKey) => void;
  onRevoke: (key: ApiKey) => void;
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
  if (key.prefix) return key.prefix;
  return 'sk_live_****';
}

function formatDate(iso: string | null) {
  if (!iso) return 'Never';
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return iso;
  }
}

export function KeyList({ keys, onSelect, onEdit, onRevoke, onDelete }: KeyListProps) {
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
            <TableHeadCell>Owner</TableHeadCell>
            <TableHeadCell>Scopes</TableHeadCell>
            <TableHeadCell>Spend (30d)</TableHeadCell>
            <TableHeadCell>Requests (24h)</TableHeadCell>
            <TableHeadCell>Last Used</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell />
          </TableRow>
        </TableHead>
        <TableBody>
          {keys.map((key) => (
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
              <TableCell>{key.owner}</TableCell>
              <TableCell>
                <div className="flex flex-wrap gap-1.5">
                  {key.scopes.map((scope) => (
                    <Badge key={scope} variant={scopeVariant(scope)}>{scope}</Badge>
                  ))}
                </div>
              </TableCell>
              <TableCell className="font-mono text-code-sm">
                ${(key.spend30d || 0).toFixed(2)}
              </TableCell>
              <TableCell className="font-mono text-code-sm">{key.requests24h || 0}</TableCell>
              <TableCell className="text-on-surface-variant">{formatDate(key.lastUsedAt)}</TableCell>
              <TableCell>
                <Toggle
                  checked={key.status === 'active'}
                  onChange={(e) => {
                    e.stopPropagation();
                    if (e.target.checked) {
                      onEdit(key);
                    } else {
                      onRevoke(key);
                    }
                  }}
                  label={key.status === 'active' ? 'Active' : 'Revoked'}
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
                    <div className="absolute right-0 mt-1 z-20 bg-white rounded-xl shadow-lg hairline-border p-1 w-40">
                      <button
                        type="button"
                        className="w-full text-left px-3 py-2 rounded-lg text-body-base hover:bg-surface-container-low"
                        onClick={() => {
                          setMenuFor(null);
                          onEdit(key);
                        }}
                      >
                        Edit
                      </button>
                      <button
                        type="button"
                        className="w-full text-left px-3 py-2 rounded-lg text-body-base hover:bg-surface-container-low"
                        onClick={() => {
                          setMenuFor(null);
                          onRevoke(key);
                        }}
                      >
                        Revoke
                      </button>
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
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
