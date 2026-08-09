export interface AuthUser {
  id: string;
  email: string;
  role: string;
  organization_id: string | null;
}

const AUTH_BASE = '/api/v1/auth';

export interface LoginResult {
  user: AuthUser;
  must_change_password: boolean;
}

export async function login(email: string, password: string): Promise<LoginResult> {
  const res = await fetch(`${AUTH_BASE}/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    throw new Error(res.status === 401 ? 'Invalid credentials' : 'Login failed');
  }
  const data = await res.json(); // access_token also returned; not needed by JS (cookie set)
  const user = await fetchMe();
  return { user, must_change_password: Boolean(data.must_change_password) };
}

export async function forgotPassword(email: string): Promise<void> {
  const res = await fetch(`${AUTH_BASE}/forgot-password`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email }),
  });
  if (!res.ok) throw new Error('Failed to send reset email');
}

export async function resetPassword(token: string, newPassword: string): Promise<void> {
  const res = await fetch(`${AUTH_BASE}/reset-password`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ token, new_password: newPassword }),
  });
  if (!res.ok) throw new Error('Password reset failed');
}

export async function changePassword(current: string, next: string): Promise<void> {
  const res = await fetch(`${AUTH_BASE}/change-password`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ current_password: current, new_password: next }),
  });
  if (!res.ok) throw new Error('Password change failed');
}

export async function changeRequired(next: string): Promise<void> {
  const res = await fetch(`${AUTH_BASE}/change-required`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ new_password: next }),
  });
  if (!res.ok) throw new Error('Password change failed');
}

export async function logout(): Promise<void> {
  await fetch(`${AUTH_BASE}/logout`, { method: 'POST', credentials: 'include' });
}

export async function fetchMe(): Promise<AuthUser> {
  const res = await fetch(`${AUTH_BASE}/me`, { credentials: 'include' });
  if (!res.ok) throw new Error('Not authenticated');
  const data = await res.json();
  return data.user as AuthUser;
}
