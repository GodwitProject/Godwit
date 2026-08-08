import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchKeys, blockKey, unblockKey, deleteKey } from './keys';

function mockFetch(data: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => data,
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('keys API', () => {
  it('fetches keys from /api/v1/api-keys', async () => {
    const m = mockFetch({ data: [] });
    await fetchKeys();
    const [url] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/api-keys');
  });

  it('blocks a key at /api/v1/api-keys/:id/block', async () => {
    const m = mockFetch({ data: { id: 'k1' } });
    await blockKey('k1');
    const [url, init] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/api-keys/k1/block');
    expect(init.method).toBe('POST');
  });

  it('unblocks a key at /api/v1/api-keys/:id/unblock', async () => {
    const m = mockFetch({ data: { id: 'k1' } });
    await unblockKey('k1');
    const [url] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/api-keys/k1/unblock');
  });

  it('deletes a key at /api/v1/api-keys/:id', async () => {
    const m = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
    vi.stubGlobal('fetch', m);
    await deleteKey('k1');
    const [url, init] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/api-keys/k1');
    expect(init.method).toBe('DELETE');
  });
});
