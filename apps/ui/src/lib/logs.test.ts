import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchLogs } from './logs';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('logs API', () => {
  it('fetches logs from /api/v1/spend/logs', async () => {
    const m = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ data: [], offset: 0, limit: 50 }),
    });
    vi.stubGlobal('fetch', m);
    await fetchLogs({ limit: 50 });
    const [url] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/spend/logs?limit=50');
  });
});
