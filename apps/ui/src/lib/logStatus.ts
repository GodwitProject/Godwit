import type { TranslationKey } from '@/i18n/translations';

export type LogState = 'ok' | 'warn' | 'error' | 'retry';

export function mapStatus(status: string): LogState {
  const s = (status || '').toLowerCase();
  if (!s) return 'retry';
  if (s === 'success') return 'ok';
  if (s.includes('rate_limit') || s.includes('ratelimit') || s.includes('limited')) return 'warn';
  if (s.includes('timeout') || s.includes('retry') || s.includes('overloaded')) return 'retry';
  return 'error';
}

export const STATUS_KEY: Record<LogState, TranslationKey> = {
  ok: 'state.ok',
  warn: 'state.limited',
  error: 'state.error',
  retry: 'state.retry',
};
