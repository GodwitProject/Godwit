import { describe, it, expect, vi, beforeEach } from 'vitest';
import { apiFetch, UnauthorizedError } from './http';

const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('apiFetch', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('returns non-401 responses directly', async () => {
    mockFetch.mockResolvedValueOnce({ status: 200, ok: true } as Response);
    const res = await apiFetch('/api/v1/models');
    expect(res.status).toBe(200);
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });

  it('refreshes on 401 and retries once', async () => {
    mockFetch
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response);

    const res = await apiFetch('/api/v1/models');
    expect(res.status).toBe(200);
    expect(mockFetch).toHaveBeenCalledTimes(3);
    expect(mockFetch).toHaveBeenNthCalledWith(2, '/api/v1/auth/refresh', expect.objectContaining({ method: 'POST' }));
  });

  it('throws UnauthorizedError when refresh fails', async () => {
    mockFetch
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 401, ok: false } as Response);

    await expect(apiFetch('/api/v1/models')).rejects.toBeInstanceOf(UnauthorizedError);
  });

  it('dedups concurrent refresh calls', async () => {
    mockFetch
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response);

    const [a, b] = await Promise.all([apiFetch('/a'), apiFetch('/b')]);
    expect(a.status).toBe(200);
    expect(b.status).toBe(200);
    expect(mockFetch).toHaveBeenCalledTimes(5);
  });
});
