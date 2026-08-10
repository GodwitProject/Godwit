import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { login, fetchMe, logout } from './auth';

const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('auth', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('login posts credentials and returns user from /auth/me', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ access_token: 'at', refresh_token: 'rt' }),
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          user: { id: '1', email: 'a@b.com', role: 'super_admin', organization_id: null },
        }),
      } as Response);

    const user = await login('a@b.com', 'pw');

    expect(mockFetch).toHaveBeenNthCalledWith(
      1,
      '/api/v1/auth/login',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        body: JSON.stringify({ email: 'a@b.com', password: 'pw' }),
      })
    );
    expect(user.email).toBe('a@b.com');
  });

  it('login throws on 401', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 } as Response);
    await expect(login('a@b.com', 'pw')).rejects.toThrow('Invalid credentials');
  });

  it('fetchMe returns user on success', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        user: { id: '1', email: 'a@b.com', role: 'super_admin', organization_id: null },
      }),
    } as Response);

    const user = await fetchMe();

    expect(mockFetch).toHaveBeenCalledWith('/api/v1/auth/me', { credentials: 'include' });
    expect(user.email).toBe('a@b.com');
  });

  it('fetchMe throws on 401', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 } as Response);
    await expect(fetchMe()).rejects.toThrow('Not authenticated');
  });

  it('logout posts to logout endpoint', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true } as Response);

    await logout();

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/auth/logout',
      expect.objectContaining({ method: 'POST', credentials: 'include' })
    );
  });
});
