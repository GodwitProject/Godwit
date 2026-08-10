import type { AuthUser } from '@/types';

export interface AuthResponse {
  user: AuthUser;
}

export async function login(email: string, password: string): Promise<AuthUser> {
  const res = await fetch('/api/v1/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });

  if (!res.ok) {
    throw new Error(res.status === 401 ? 'Invalid credentials' : 'Login failed');
  }

  await res.json();
  return fetchMe();
}

export async function logout(): Promise<void> {
  await fetch('/api/v1/auth/logout', {
    method: 'POST',
    credentials: 'include',
  });
}

export async function fetchMe(): Promise<AuthUser> {
  const res = await fetch('/api/v1/auth/me', { credentials: 'include' });
  if (!res.ok) throw new Error('Not authenticated');
  const data = (await res.json()) as AuthResponse;
  return data.user;
}
