import { describe, it, expect, beforeEach } from 'vitest';
import { useAuthStore, type AuthStatus } from './auth';

const user = {
  id: 'user_1',
  email: 'admin@example.com',
  role: 'admin',
  organization_id: null,
};

describe('useAuthStore', () => {
  beforeEach(() => {
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  it('starts unknown with no user', () => {
    const s = useAuthStore.getState();
    expect(s.user).toBeNull();
    expect(s.status).toBe('unknown');
  });

  it('setUser with a user transitions to authenticated', () => {
    useAuthStore.getState().setUser(user);
    const s = useAuthStore.getState();
    expect(s.user).toEqual(user);
    expect(s.status).toBe('authenticated');
  });

  it('setUser with null transitions to unauthenticated', () => {
    useAuthStore.getState().setUser(user);
    useAuthStore.getState().setUser(null);
    const s = useAuthStore.getState();
    expect(s.user).toBeNull();
    expect(s.status).toBe('unauthenticated');
  });
});
