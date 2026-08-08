import { useT } from '@/hooks/useT';
import { mapStatus, STATUS_KEY, type LogState } from '@/lib/logStatus';

const STATE_CLASS: Record<LogState, string> = {
  ok: 'ok',
  warn: 'warn',
  error: 'err',
  retry: 'retry',
};

export function StatusPill({ status }: { status: string }) {
  const { t } = useT();
  const state = mapStatus(status);
  return (
    <span className={`pill ${STATE_CLASS[state]}`}>
      <span className="dot" />
      {t(STATUS_KEY[state])}
    </span>
  );
}
