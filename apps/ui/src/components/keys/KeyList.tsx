import { useState } from 'react';
import { Card } from '../ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '../ui/Table';
import { Badge } from '../ui/Badge';
import { Toggle } from '../ui/Toggle';
import { useT } from '@/hooks/useT';
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
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return iso;
  }
}

export function KeyList({ keys, onSelect, onToggleActive, onDelete }: KeyListProps) {
  const { t } = useT();
  const [menuFor, setMenuFor] = useState<string | null>(null);

  if (keys.length === 0) {
    return (
      <Card>
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <span className="text-3xl text-muted mb-2">🔑</span>
          <p className="text-[13px] text-muted">{t('keys.noKeys')}</p>
        </div>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <div className="px-4 py-3 border-b border-border">
        <h3 className="text-[13px] font-semibold">{t('keys.active')}</h3>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>{t('keys.name')}</TableHeadCell>
            <TableHeadCell>{t('keys.prefix')}</TableHeadCell>
            <TableHeadCell>{t('keys.scopes')}</TableHeadCell>
            <TableHeadCell>{t('keys.spent')}</TableHeadCell>
            <TableHeadCell>{t('keys.rateLimit')}</TableHeadCell>
            <TableHeadCell>{t('keys.expires')}</TableHeadCell>
            <TableHeadCell>{t('keys.state')}</TableHeadCell>
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
                <TableCell className="font-medium">{key.name}</TableCell>
                <TableCell className="font-mono text-[11.5px] text-muted">{prefixLabel(key)}</TableCell>
                <TableCell>
                  <div className="flex flex-wrap gap-1.5">
                    {key.scopes.map((scope) => (
                      <Badge key={scope} variant={scopeVariant(scope)}>{scope}</Badge>
                    ))}
                  </div>
                </TableCell>
                <TableCell className="font-mono text-[11.5px]">
                  {key.budget_spent_usd != null ? `$${key.budget_spent_usd.toFixed(2)}` : '—'}
                </TableCell>
                <TableCell className="font-mono text-[11.5px]">
                  {formatRateLimit(key.rate_limit_requests_per_minute)} RPM
                </TableCell>
                <TableCell className="text-muted">{formatExpiry(key.expires_at)}</TableCell>
                <TableCell>
                  <Toggle
                    checked={active}
                    onChange={(e) => {
                      e.stopPropagation();
                      onToggleActive(key);
                    }}
                    label={active ? t('keys.status.active') : t('keys.status.revoked')}
                  />
                </TableCell>
                <TableCell>
                  <div className="relative" onClick={(e) => e.stopPropagation()}>
                    <button
                      type="button"
                      className="text-muted hover:text-fg px-1 rounded hover:bg-surface-2 leading-none"
                      onClick={() => setMenuFor(menuFor === key.id ? null : key.id)}
                      aria-label={`Actions for ${key.name}`}
                    >
                      ⋯
                    </button>
                    {menuFor === key.id && (
                      <div className="absolute right-0 mt-1 z-20 bg-surface rounded-xl shadow-drawer border border-border p-1 w-36">
                        <button
                          type="button"
                          className="w-full text-left px-3 py-2 rounded-lg text-[12.5px] text-danger hover:bg-surface-2"
                          onClick={() => {
                            setMenuFor(null);
                            onDelete(key);
                          }}
                        >
                          {t('keys.delete')}
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
