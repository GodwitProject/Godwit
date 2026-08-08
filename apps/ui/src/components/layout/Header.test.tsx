import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Header } from './Header';
import { logout } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';

vi.mock('@/lib/auth', () => ({
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
}));

const push = vi.fn();
vi.mock('next/navigation', () => ({
  useRouter: () => ({ push, replace: vi.fn() }),
  usePathname: () => '/',
}));

const user = {
  id: 'user_1',
  email: 'admin@example.com',
  role: 'admin',
  organization_id: null,
};

describe('Header', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  it('shows the signed-in user email', () => {
    useAuthStore.setState({ user, status: 'authenticated' });
    render(<Header onOpenShortcuts={() => {}} />);
    expect(screen.getByText('admin@example.com')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign out' })).toBeInTheDocument();
  });

  it('does not render identity or sign out when signed out', () => {
    useAuthStore.setState({ user: null, status: 'unauthenticated' });
    render(<Header onOpenShortcuts={() => {}} />);
    expect(screen.queryByText('admin@example.com')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Sign out' })).not.toBeInTheDocument();
  });

  it('sign out calls logout, clears the store, and navigates to /login', async () => {
    useAuthStore.setState({ user, status: 'authenticated' });
    vi.mocked(logout).mockResolvedValue(undefined as never);
    render(<Header onOpenShortcuts={() => {}} />);

    fireEvent.click(screen.getByRole('button', { name: 'Sign out' }));

    await waitFor(() => expect(logout).toHaveBeenCalled());
    expect(useAuthStore.getState().user).toBeNull();
    expect(useAuthStore.getState().status).toBe('unauthenticated');
    expect(push).toHaveBeenCalledWith('/login');
  });
});
