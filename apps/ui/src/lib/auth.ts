export interface AuthUser {
  id: string;
  email: string;
  role: string;
  organization_id: string | null;
}

const AUTH_BASE = '/api/v1/auth';

export async function login(email: string, password: string): Promise<AuthUser> {
  const res = await fetch(`${AUTH_BASE}/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) {
    throw new Error(res.status === 401 ? 'Invalid credentials' : 'Login failed');
  }
  await res.json(); // access_token also returned; not needed by JS (cookie set)
  return fetchMe();
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
