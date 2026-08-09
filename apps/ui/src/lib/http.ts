export class UnauthorizedError extends Error {
  constructor() { super('Unauthorized'); this.name = 'UnauthorizedError'; }
}

let refreshPromise: Promise<boolean> | null = null;

async function doRefresh(): Promise<boolean> {
  try {
    const r = await fetch('/api/v1/auth/refresh', { method: 'POST', credentials: 'include' });
    return r.ok;
  } catch { return false; }
}

export { doRefresh };

export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const merged: RequestInit = { ...init, credentials: 'include' };
  const res = await fetch(path, merged);
  if (res.status !== 401) return res;

  if (!refreshPromise) {
    refreshPromise = doRefresh().finally(() => { refreshPromise = null; });
  }
  const ok = await refreshPromise;
  if (!ok) throw new UnauthorizedError();

  return fetch(path, merged); // retry original once
}
