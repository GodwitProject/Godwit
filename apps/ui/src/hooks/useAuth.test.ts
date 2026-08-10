import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useAuthInit, useLogout } from './useAuth';
import { useAuthStore } from '@/store/auth';
import type { AuthUser } from '@/types';

const mockFetch = vi.fn();

describe('useAuthInit', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    global.fetch = mockFetch;
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('sets the user when fetchMe succeeds', async () => {
    const user: AuthUser = {
      id: '1',
      email: 'a@b.com',
      role: 'super_admin',
      organization_id: null,
    };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ user }),
    } as Response);

    renderHook(() => useAuthInit());

    await waitFor(() => {
      expect(useAuthStore.getState().user).toEqual(user);
      expect(useAuthStore.getState().status).toBe('authenticated');
    });

    expect(mockFetch).toHaveBeenCalledWith('/api/v1/auth/me', { credentials: 'include' });
  });

  it('sets unauthenticated when fetchMe returns 401', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 } as Response);

    renderHook(() => useAuthInit());

    await waitFor(() => {
      expect(useAuthStore.getState().user).toBeNull();
      expect(useAuthStore.getState().status).toBe('unauthenticated');
    });
  });
});

describe('useLogout', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    global.fetch = mockFetch;
    useAuthStore.setState({
      user: { id: '1', email: 'a@b.com', role: 'super_admin', organization_id: null },
      status: 'authenticated',
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('calls logout endpoint and clears the user', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true } as Response);

    const { result } = renderHook(() => useLogout());
    await result.current();

    expect(mockFetch).toHaveBeenCalledWith('/api/v1/auth/logout', expect.objectContaining({ method: 'POST' }));
    expect(useAuthStore.getState().user).toBeNull();
    expect(useAuthStore.getState().status).toBe('unauthenticated');
  });
});
