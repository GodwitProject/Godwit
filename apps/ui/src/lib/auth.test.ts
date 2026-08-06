import { describe, it, expect, vi, beforeEach } from 'vitest';
import { login, logout, fetchMe, type AuthUser } from './auth';

const user: AuthUser = {
  id: 'user_1',
  email: 'admin@example.com',
  role: 'admin',
  organization_id: null,
};

function mockFetchResponse(body: unknown, init: { ok?: boolean; status?: number } = {}) {
  const { ok = true, status = 200 } = init;
  return {
    ok,
    status,
    json: vi.fn().mockResolvedValue(body),
  } as unknown as Response;
}

describe('auth lib', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('login POSTs the correct path, headers, and credentials', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(mockFetchResponse({ access_token: 'token' }))
      .mockResolvedValueOnce(mockFetchResponse({ user }));
    vi.stubGlobal('fetch', fetchMock);

    const result = await login('admin@example.com', 'secret');

    const [loginCall] = fetchMock.mock.calls;
    expect(loginCall[0]).toBe('/api/v1/auth/login');
    expect(loginCall[1]).toMatchObject({
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
    });
    expect(JSON.parse(loginCall[1].body)).toEqual({
      email: 'admin@example.com',
      password: 'secret',
    });
    expect(result).toEqual(user);
  });

  it('throws Invalid credentials on 401', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({}, { ok: false, status: 401 })));

    await expect(login('a@b.com', 'x')).rejects.toThrow('Invalid credentials');
  });

  it('fetchMe returns the user on 200', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({ user })));

    const result = await fetchMe();
    expect(result).toEqual(user);
  });

  it('fetchMe throws when not authenticated', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({}, { ok: false, status: 401 })));

    await expect(fetchMe()).rejects.toThrow('Not authenticated');
  });

  it('logout POSTs to the logout endpoint with credentials', async () => {
    const fetchMock = vi.fn().mockResolvedValue(mockFetchResponse({}));
    vi.stubGlobal('fetch', fetchMock);

    await logout();

    expect(fetchMock).toHaveBeenCalledWith('/api/v1/auth/logout', {
      method: 'POST',
      credentials: 'include',
    });
  });
});
