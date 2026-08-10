import { describe, it, expect, beforeEach } from 'vitest';
import { useAuthStore } from './auth';
import type { AuthUser } from '@/types';

const sampleUser: AuthUser = {
  id: '1',
  email: 'a@b.com',
  role: 'super_admin',
  organization_id: null,
};

describe('auth store', () => {
  beforeEach(() => {
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  it('has unknown initial state', () => {
    expect(useAuthStore.getState().user).toBeNull();
    expect(useAuthStore.getState().status).toBe('unknown');
  });

  it('setUser with a user marks authenticated', () => {
    useAuthStore.getState().setUser(sampleUser);
    expect(useAuthStore.getState().user).toEqual(sampleUser);
    expect(useAuthStore.getState().status).toBe('authenticated');
  });

  it('setUser with null marks unauthenticated', () => {
    useAuthStore.getState().setUser(sampleUser);
    useAuthStore.getState().setUser(null);
    expect(useAuthStore.getState().user).toBeNull();
    expect(useAuthStore.getState().status).toBe('unauthenticated');
  });
});
