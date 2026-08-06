import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { RequireAuth } from './RequireAuth';
import { fetchMe } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';

vi.mock('@/lib/auth', () => ({
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
}));

const replace = vi.fn();
vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace }),
}));

const user = {
  id: 'user_1',
  email: 'admin@example.com',
  role: 'admin',
  organization_id: null,
};

describe('RequireAuth', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  it('renders children when authenticated', () => {
    useAuthStore.setState({ user, status: 'authenticated' });
    render(<RequireAuth><div>secret content</div></RequireAuth>);
    expect(screen.getByText('secret content')).toBeInTheDocument();
  });

  it('fetches the user when status is unknown, then renders children', async () => {
    vi.mocked(fetchMe).mockResolvedValue(user as never);
    render(<RequireAuth><div>secret content</div></RequireAuth>);

    expect(screen.getByText('Loading…')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('secret content')).toBeInTheDocument());
    expect(fetchMe).toHaveBeenCalled();
    expect(replace).not.toHaveBeenCalled();
  });

  it('redirects to /login when unauthenticated', () => {
    useAuthStore.setState({ user: null, status: 'unauthenticated' });
    render(<RequireAuth><div>secret content</div></RequireAuth>);
    expect(replace).toHaveBeenCalledWith('/login');
  });

  it('redirects to /login when fetchMe rejects', async () => {
    vi.mocked(fetchMe).mockRejectedValue(new Error('Not authenticated') as never);
    render(<RequireAuth><div>secret content</div></RequireAuth>);
    await waitFor(() => expect(replace).toHaveBeenCalledWith('/login'));
    expect(useAuthStore.getState().status).toBe('unauthenticated');
  });
});
