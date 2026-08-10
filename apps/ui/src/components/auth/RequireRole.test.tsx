import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import { RequireRole } from './RequireRole';
import { useAuthStore } from '@/store/auth';
import type { AuthUser } from '@/types';

function setUser(user: AuthUser | null) {
  useAuthStore.getState().setUser(user);
}

beforeEach(() => {
  useAuthStore.getState().setUser(null);
});

describe('RequireRole', () => {
  it('renders children when role is allowed', () => {
    setUser({ id: '1', email: 'a@b.com', role: 'super_admin', organization_id: null });
    render(
      <MemoryRouter>
        <RequireRole allowed={['super_admin']}>admin content</RequireRole>
      </MemoryRouter>
    );
    expect(screen.getByText('admin content')).toBeInTheDocument();
  });

  it('redirects when role is not allowed', () => {
    setUser({ id: '1', email: 'a@b.com', role: 'user', organization_id: 'org-1' });
    render(
      <MemoryRouter initialEntries={['/admin']}>
        <Routes>
          <Route path="/login" element={<div>login page</div>} />
          <Route
            path="/admin"
            element={
              <RequireRole allowed={['super_admin']}>
                <div>admin content</div>
              </RequireRole>
            }
          />
        </Routes>
      </MemoryRouter>
    );
    expect(screen.getByText('login page')).toBeInTheDocument();
  });
});
