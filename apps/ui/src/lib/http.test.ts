import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { apiFetch, UnauthorizedError } from './http';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
}

describe('apiFetch', () => {
  const refreshSpy = vi.fn();

  beforeEach(() => {
    refreshSpy.mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('adds credentials include to every request', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ ok: true }));
    vi.stubGlobal('fetch', fetchMock);

    await apiFetch('/api/v1/admin/stats');

    expect(fetchMock).toHaveBeenCalledWith('/api/v1/admin/stats', {
      credentials: 'include',
    });
  });

  it('returns a 200 response as-is', async () => {
    const body = { data: [1, 2, 3] };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(jsonResponse(body)));

    const res = await apiFetch('/api/v1/admin/stats');

    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual(body);
  });

  it('on 401 refreshes once and retries the original request', async () => {
    const refreshResponse = jsonResponse({}, 200);
    const retryResponse = jsonResponse({ data: 'ok' }, 200);

    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({}, 401))
      .mockResolvedValueOnce(refreshResponse)
      .mockResolvedValueOnce(retryResponse);
    vi.stubGlobal('fetch', fetchMock);

    const res = await apiFetch('/api/v1/admin/stats');

    expect(fetchMock).toHaveBeenNthCalledWith(1, '/api/v1/admin/stats', {
      credentials: 'include',
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, '/api/v1/auth/refresh', {
      method: 'POST',
      credentials: 'include',
    });
    expect(fetchMock).toHaveBeenNthCalledWith(3, '/api/v1/admin/stats', {
      credentials: 'include',
    });
    await expect(res.json()).resolves.toEqual({ data: 'ok' });
  });

  it('dedups concurrent 401s to a single refresh', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === '/api/v1/auth/refresh') {
        refreshSpy();
        return Promise.resolve(jsonResponse({}, 200));
      }
      if (url === '/api/v1/admin/stats') {
        return Promise.resolve(jsonResponse({}, 401));
      }
      return Promise.resolve(jsonResponse({ data: 'ok' }, 200));
    });

    const [a, b, c] = await Promise.all([
      apiFetch('/api/v1/admin/stats'),
      apiFetch('/api/v1/admin/stats'),
      apiFetch('/api/v1/admin/stats'),
    ]);

    expect(refreshSpy).toHaveBeenCalledTimes(1);
    await Promise.all([a.json(), b.json(), c.json()]);
  });

  it('throws UnauthorizedError when refresh fails', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({}, 401))
      .mockResolvedValueOnce(jsonResponse({}, 401));
    vi.stubGlobal('fetch', fetchMock);

    await expect(apiFetch('/api/v1/admin/stats')).rejects.toThrow(UnauthorizedError);
  });
});
