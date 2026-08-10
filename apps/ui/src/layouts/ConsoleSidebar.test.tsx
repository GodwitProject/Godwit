import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { ConsoleSidebar } from './ConsoleSidebar';
import { useAuthStore } from '@/store/auth';
import type { AuthUser } from '@/types';

function setUser(user: AuthUser | null) {
  useAuthStore.getState().setUser(user);
}

beforeEach(() => {
  useAuthStore.setState({ user: null, status: 'unknown' });
});

describe('ConsoleSidebar', () => {
  it('shows Organization link for org_admin', () => {
    setUser({ id: '1', email: 'a@b.com', role: 'org_admin', organization_id: 'org-1' });
    render(
      <MemoryRouter>
        <ConsoleSidebar />
      </MemoryRouter>
    );
    expect(screen.getByText('Organization')).toBeInTheDocument();
  });

  it('hides Organization link for user', () => {
    setUser({ id: '1', email: 'a@b.com', role: 'user', organization_id: 'org-1' });
    render(
      <MemoryRouter>
        <ConsoleSidebar />
      </MemoryRouter>
    );
    expect(screen.queryByText('Organization')).not.toBeInTheDocument();
  });
});
