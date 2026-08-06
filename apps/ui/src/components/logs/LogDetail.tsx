import { Modal } from '../ui/Modal';
import { Badge } from '../ui/Badge';
import type { RequestLog } from '../../lib/logs';

export interface LogDetailProps {
  open: boolean;
  log?: RequestLog;
  onClose: () => void;
}

function JsonBlock({ data }: { data: unknown }) {
  const text = JSON.stringify(data, null, 2);
  return (
    <pre className="bg-surface-container-low p-container-padding rounded-lg text-code-sm font-mono overflow-x-auto text-on-surface whitespace-pre-wrap">
      {text}
    </pre>
  );
}

function formatDate(iso?: string): string {
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
}

function maskKey(prefix?: string): string {
  if (prefix) return `${prefix}••••••••`;
  return 'sk_live_••••••••';
}

export function LogDetail({ open, log, onClose }: LogDetailProps) {
  if (!log) {
    return (
      <Modal open={open} onClose={onClose} title="Log Details">
        <p className="text-body-base text-on-surface-variant">Loading log details...</p>
      </Modal>
    );
  }

  return (
    <Modal open={open} onClose={onClose} title="Log Details" maxWidth="max-w-4xl">
      <div className="space-y-6">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-mono text-code-sm text-on-surface-variant">{log.requestId}</span>
          <Badge
            variant={
              log.status >= 200 && log.status < 300
                ? 'success'
                : log.status === 429
                ? 'warning'
                : 'error'
            }
          >
            {log.status}
          </Badge>
          <Badge variant={log.fallbackUsed ? 'info' : 'default'}>
            {log.fallbackUsed ? 'Fallback' : 'Primary'}
          </Badge>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Model</p>
            <p className="font-mono text-code-sm mt-1">{log.model}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Provider</p>
            <p className="text-body-base mt-1">{log.provider}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Finish Reason</p>
            <p className="font-mono text-code-sm mt-1">{log.finishReason || '—'}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Latency</p>
            <p className="font-mono text-code-sm mt-1">{log.latencyMs}ms</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Cost</p>
            <p className="font-mono text-code-sm mt-1">{formatCost(log.cost)}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">API Key</p>
            <p className="font-mono text-code-sm mt-1">{maskKey(log.apiKeyPrefix)}</p>
          </div>
          <div className="col-span-2">
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Timestamp</p>
            <p className="font-mono text-code-sm mt-1">{formatDate(log.timestamp)}</p>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <h3 className="text-section-sm mb-2">Request Body</h3>
            <JsonBlock data={log.requestBody} />
          </div>
          <div>
            <h3 className="text-section-sm mb-2">Response Body</h3>
            <JsonBlock data={log.responseBody} />
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Prompt Tokens</p>
            <p className="font-mono text-title-md mt-1">{log.tokensIn}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Completion Tokens</p>
            <p className="font-mono text-title-md mt-1">{log.tokensOut}</p>
          </div>
          <div>
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Total</p>
            <p className="font-mono text-title-md mt-1">{log.tokensIn + log.tokensOut}</p>
          </div>
        </div>

        {log.timeline && log.timeline.length > 0 && (
          <div>
            <h3 className="text-section-sm mb-2">Timeline</h3>
            <div className="space-y-1">
              {log.timeline.map((t, i) => (
                <div key={i} className="flex items-center gap-3 text-code-sm">
                  <span className="font-mono text-on-surface-variant">{t.time}</span>
                  <span className="text-on-surface">{t.event}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="flex flex-wrap items-center gap-2">
          <h3 className="text-section-sm mr-1">Guardrails</h3>
          <Badge variant={log.piiDetected ? 'warning' : 'success'}>
            {log.piiDetected ? 'PII detected' : 'No PII'}
          </Badge>
          <Badge
            variant={
              log.moderationStatus === 'allowed'
                ? 'success'
                : log.moderationStatus === 'blocked'
                ? 'error'
                : 'default'
            }
          >
            {log.moderationStatus === 'allowed'
              ? 'Moderation passed'
              : log.moderationStatus === 'blocked'
              ? 'Moderation blocked'
              : 'Not checked'}
          </Badge>
        </div>
      </div>
    </Modal>
  );
}
