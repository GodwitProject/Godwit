import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TopBar } from './TopBar';
import { useAuthStore } from '@/store/auth';

const mockFetch = vi.fn();

describe('TopBar', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    global.fetch = mockFetch;
    useAuthStore.setState({ user: null, status: 'unknown' });
  });

  it('displays user email and role', () => {
    useAuthStore.setState({
      user: { id: '1', email: 'admin@godwit.dev', role: 'super_admin', organization_id: null },
      status: 'authenticated',
    });

    render(<TopBar />);

    expect(screen.getByText('admin@godwit.dev')).toBeInTheDocument();
    expect(screen.getByText('super admin')).toBeInTheDocument();
  });

  it('calls logout when sign out is clicked', async () => {
    useAuthStore.setState({
      user: { id: '1', email: 'admin@godwit.dev', role: 'super_admin', organization_id: null },
      status: 'authenticated',
    });
    mockFetch.mockResolvedValueOnce({ ok: true } as Response);

    render(<TopBar />);

    await userEvent.click(screen.getByRole('button', { name: /sign out/i }));

    expect(mockFetch).toHaveBeenCalledWith('/api/v1/auth/logout', expect.objectContaining({ method: 'POST' }));
    expect(useAuthStore.getState().user).toBeNull();
    expect(useAuthStore.getState().status).toBe('unauthenticated');
  });
});
