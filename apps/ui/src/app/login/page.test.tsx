import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import LoginPage from './page';
import { login } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';

vi.mock('@/lib/auth', () => ({
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
}));

const push = vi.fn();
vi.mock('next/navigation', () => ({
  useRouter: () => ({ push, replace: vi.fn() }),
}));

const user = {
  id: 'user_1',
  email: 'admin@example.com',
  role: 'admin',
  organization_id: null,
};

describe('LoginPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  it('navigates to / after successful login', async () => {
    vi.mocked(login).mockResolvedValue({ user, must_change_password: false } as never);
    render(<LoginPage />);

    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'secret' } });
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => expect(push).toHaveBeenCalledWith('/'));
    expect(login).toHaveBeenCalledWith('admin@example.com', 'secret');
    expect(useAuthStore.getState().user).toEqual(user);
    expect(useAuthStore.getState().status).toBe('authenticated');
  });

  it('redirects to change-required when must_change_password is true', async () => {
    vi.mocked(login).mockResolvedValue({ user, must_change_password: true } as never);
    render(<LoginPage />);

    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'secret' } });
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => expect(push).toHaveBeenCalledWith('/change-required'));
  });

  it('shows an error message on failed login', async () => {
    vi.mocked(login).mockRejectedValue(new Error('Invalid credentials') as never);
    render(<LoginPage />);

    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'admin@example.com' } });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'wrong' } });
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    expect(await screen.findByText('Invalid credentials')).toBeInTheDocument();
    expect(push).not.toHaveBeenCalled();
  });
});
