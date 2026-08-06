import { Modal } from '../ui/Modal';
import type { RequestLog } from '../../lib/logs';

export interface LogDetailProps {
  open: boolean;
  log?: RequestLog;
  onClose: () => void;
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

function formatLatency(durationMs: number | null): string {
  return durationMs != null ? `${durationMs}ms` : '—';
}

export function LogDetail({ open, log, onClose }: LogDetailProps) {
  return (
    <Modal open={open} onClose={onClose} title="Log Details" maxWidth="max-w-4xl">
      {!log ? (
        <p className="text-body-base text-on-surface-variant">No log selected.</p>
      ) : (
        <div className="space-y-6">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-mono text-code-sm text-on-surface-variant">{log.id}</span>
          </div>

          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Model</p>
              <p className="font-mono text-code-sm mt-1">{log.model || '—'}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Provider</p>
              <p className="text-body-base mt-1">{log.provider || '—'}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Capability</p>
              <p className="font-mono text-code-sm mt-1">{log.capability || '—'}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Streamed</p>
              <p className="font-mono text-code-sm mt-1">{log.streamed ? 'Yes' : 'No'}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Latency</p>
              <p className="font-mono text-code-sm mt-1">{formatLatency(log.duration_ms)}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Cost</p>
              <p className="font-mono text-code-sm mt-1">{formatCost(log.cost_usd)}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">API Key</p>
              <p className="font-mono text-code-sm mt-1">{log.api_key_id || '—'}</p>
            </div>
            <div>
              <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider">Timestamp</p>
              <p className="font-mono text-code-sm mt-1">{formatDate(log.created_at)}</p>
            </div>
          </div>

          <div className="bg-surface-container-low p-container-padding rounded-lg">
            <p className="text-caption-xs text-on-surface-variant uppercase tracking-wider mb-2">Details</p>
            <p className="text-body-base text-on-surface-variant">
              Request/response payloads and guardrail details are not available from the spend logs endpoint yet.
            </p>
          </div>
        </div>
      )}
    </Modal>
  );
}
