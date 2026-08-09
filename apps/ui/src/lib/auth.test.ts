import { describe, it, expect, vi, beforeEach } from 'vitest';
import { login, logout, fetchMe, forgotPassword, resetPassword, changePassword, changeRequired, type AuthUser } from './auth';

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
    expect(result).toEqual({ user, must_change_password: false });
  });

  it('login exposes must_change_password from the response', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(mockFetchResponse({ access_token: 'token', must_change_password: true }))
      .mockResolvedValueOnce(mockFetchResponse({ user }));
    vi.stubGlobal('fetch', fetchMock);

    const result = await login('admin@example.com', 'secret');
    expect(result).toEqual({ user, must_change_password: true });
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

  it('forgotPassword POSTs the email and resolves on 200', async () => {
    const fetchMock = vi.fn().mockResolvedValue(mockFetchResponse({ ok: true }));
    vi.stubGlobal('fetch', fetchMock);

    await forgotPassword('admin@example.com');

    expect(fetchMock).toHaveBeenCalledWith('/api/v1/auth/forgot-password', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ email: 'admin@example.com' }),
    });
  });

  it('forgotPassword throws on failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({}, { ok: false, status: 500 })));
    await expect(forgotPassword('a@b.com')).rejects.toThrow('Failed to send reset email');
  });

  it('resetPassword POSTs token and new_password', async () => {
    const fetchMock = vi.fn().mockResolvedValue(mockFetchResponse({ ok: true }));
    vi.stubGlobal('fetch', fetchMock);

    await resetPassword('tok_123', 'newsecret');

    expect(fetchMock).toHaveBeenCalledWith('/api/v1/auth/reset-password', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ token: 'tok_123', new_password: 'newsecret' }),
    });
  });

  it('resetPassword throws on failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({}, { ok: false, status: 400 })));
    await expect(resetPassword('tok', 'x')).rejects.toThrow('Password reset failed');
  });

  it('changePassword POSTs current and new password', async () => {
    const fetchMock = vi.fn().mockResolvedValue(mockFetchResponse({ changed: true }));
    vi.stubGlobal('fetch', fetchMock);

    await changePassword('oldsecret', 'newsecret');

    expect(fetchMock).toHaveBeenCalledWith('/api/v1/auth/change-password', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ current_password: 'oldsecret', new_password: 'newsecret' }),
    });
  });

  it('changePassword throws on failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({}, { ok: false, status: 401 })));
    await expect(changePassword('a', 'b')).rejects.toThrow('Password change failed');
  });

  it('changeRequired POSTs the new password', async () => {
    const fetchMock = vi.fn().mockResolvedValue(mockFetchResponse({ changed: true }));
    vi.stubGlobal('fetch', fetchMock);

    await changeRequired('newsecret');

    expect(fetchMock).toHaveBeenCalledWith('/api/v1/auth/change-required', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ new_password: 'newsecret' }),
    });
  });

  it('changeRequired throws on failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockFetchResponse({}, { ok: false, status: 400 })));
    await expect(changeRequired('x')).rejects.toThrow('Password change failed');
  });
});
