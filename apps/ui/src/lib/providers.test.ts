import { describe, it, expect, vi, afterEach } from 'vitest';
import { fetchProviders, setProviderEnabled } from './providers';

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

describe('providers API', () => {
  it('fetches providers from /api/v1/provider-profiles', async () => {
    const m = mockFetch({ data: [] });
    await fetchProviders();
    const [url] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/provider-profiles');
  });

  it('patches enabled at /api/v1/provider-profiles/:id', async () => {
    const m = mockFetch({ id: 'p1', enabled: false });
    await setProviderEnabled('p1', false);
    const [url, init] = (m as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/provider-profiles/p1');
    expect(init.method).toBe('PATCH');
  });
});
