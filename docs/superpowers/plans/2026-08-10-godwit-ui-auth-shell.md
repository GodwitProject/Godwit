# Godwit UI — Milestone 0: Auth + Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer les anciennes apps frontend (`apps/admin/` et le contenu de `apps/ui/`) par une SPA Vite + React fraîche, avec auth cookie-based, deux layouts (Admin `/admin/*` et Console `/console/*`), RBAC côté client, et tous les composants UI de base nécessaires.

**Architecture:** SPA client-first (Vite + React + React Router). Auth via cookies httpOnly posés par le backend, fetch wrapper `apiFetch` gérant le refresh auto sur 401. État auth dans Zustand ; données serveur dans TanStack Query. UI en Tailwind avec tokens Godwit.

**Tech Stack:** Vite 5, React 18, TypeScript 5, React Router 6, TanStack Query 5, Zustand 4, Tailwind CSS 3.4, React Hook Form 7 + Zod 3, Lucide React, Vitest + React Testing Library + MSW.

## Global Constraints

- Toutes les requêtes authentifiées utilisent `credentials: 'include'`.
- Le frontend ne manipule jamais le JWT directement ; il lit l'utilisateur via `GET /api/v1/auth/me`.
- `/admin/*` est réservé au rôle `super_admin`.
- `/console/*` est accessible à `super_admin`, `org_admin`, `team_admin`, `user`.
- Dans la console, l'onglet Organisation n'apparaît que pour `org_admin` ; l'onglet Équipe n'apparaît que pour `team_admin`.
- Pas de `any` ; TypeScript strict activé.
- Tests co-localisés ou dans `src/__tests__` ; au choix, mais uniforme.
- **Chaque nouveau composant/hook/lib doit avoir un test unitaire co-localisé.**
- **Chaque correction de bug doit être accompagnée d'un test de régression qui échoue avant et passe après.**
- Les tests doivent être porteurs de sens : un test qui ne ferait que parcourir le code sans assertion est inacceptable.
- Chaque task finit par un `npm run test` ou un `npm run build` propre.
- Pas de commit automatique sans accord explicite.
- Mettre à jour `AGENTS.md` avec une section `Testing policy` avant de clore le milestone.

---

### Task 1: Cleanup

**Files:**
- Delete: `apps/admin/`
- Delete: all contents of `apps/ui/`

**Interfaces:**
- Produces: empty `apps/ui/` directory.

- [ ] **Step 1: Remove old apps**

```bash
rm -rf /home/thomas/work/Godwit/apps/admin
rm -rf /home/thomas/work/Godwit/apps/ui/*
rm -rf /home/thomas/work/Godwit/apps/ui/.* 2>/dev/null || true
```

- [ ] **Step 2: Recreate empty `apps/ui/`**

```bash
mkdir -p /home/thomas/work/Godwit/apps/ui
```

- [ ] **Step 3: Verify cleanup**

```bash
ls -la /home/thomas/work/Godwit/apps/
ls -la /home/thomas/work/Godwit/apps/ui/
```

Expected: `apps/admin` does not exist ; `apps/ui` exists and is empty.

---

### Task 2: Vite Scaffold

**Files:**
- Create: `apps/ui/package.json`
- Create: `apps/ui/vite.config.ts`
- Create: `apps/ui/tsconfig.json`
- Create: `apps/ui/tsconfig.node.json`
- Create: `apps/ui/tailwind.config.ts`
- Create: `apps/ui/postcss.config.js`
- Create: `apps/ui/index.html`
- Create: `apps/ui/vitest.config.ts`
- Create: `apps/ui/src/main.tsx`
- Create: `apps/ui/src/vite-env.d.ts`

**Interfaces:**
- Produces: runnable Vite + React + TypeScript + Tailwind + Vitest project.

- [ ] **Step 1: Create `package.json`**

```json
{
  "name": "godwit-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "type-check": "tsc --noEmit"
  },
  "dependencies": {
    "@hookform/resolvers": "^3.9.0",
    "@tanstack/react-query": "^5.51.0",
    "lucide-react": "^0.400.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-hook-form": "^7.52.0",
    "react-router-dom": "^6.25.0",
    "zod": "^3.23.0",
    "zustand": "^4.5.0"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.4.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/user-event": "^14.5.0",
    "@types/node": "^20.14.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "autoprefixer": "^10.4.0",
    "jsdom": "^24.1.0",
    "msw": "^2.3.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.5.0",
    "vite": "^5.3.0",
    "vitest": "^2.0.0"
  }
}
```

- [ ] **Step 2: Create `vite.config.ts`**

```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3001,
    proxy: {
      '/api': {
        target: process.env.VITE_API_URL || 'http://localhost:3000',
        changeOrigin: true,
      },
      '/health': {
        target: process.env.VITE_API_URL || 'http://localhost:3000',
        changeOrigin: true,
      },
      '/metrics': {
        target: process.env.VITE_API_URL || 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },
});
```

- [ ] **Step 3: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 4: Create `tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 5: Create `tailwind.config.ts`**

```ts
import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        surface: '#f8f9fb',
        'surface-dim': '#d9dadc',
        'surface-bright': '#f8f9fb',
        'surface-container-lowest': '#ffffff',
        'surface-container-low': '#f3f4f6',
        'surface-container': '#edeef0',
        'surface-container-high': '#e7e8ea',
        'surface-container-highest': '#e1e2e4',
        'on-surface': '#191c1e',
        'on-surface-variant': '#434655',
        primary: '#004ac6',
        'on-primary': '#ffffff',
        'primary-container': '#2563eb',
        'on-primary-container': '#eeefff',
        'primary-fixed': '#dbe1ff',
        'primary-fixed-dim': '#b4c5ff',
        secondary: '#515f74',
        'on-secondary': '#ffffff',
        'secondary-container': '#d5e3fc',
        'on-secondary-container': '#57657a',
        tertiary: '#005a82',
        'on-tertiary': '#ffffff',
        'tertiary-container': '#0074a6',
        'on-tertiary-container': '#e4f2ff',
        error: '#ba1a1a',
        'on-error': '#ffffff',
        'error-container': '#ffdad6',
        success: '#10b981',
        warning: '#f59e0b',
        info: '#3b82f6',
        outline: '#737686',
        'outline-variant': '#c3c6d7',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      fontSize: {
        'display-lg': ['30px', { lineHeight: '36px', fontWeight: '700', letterSpacing: '-0.02em' }],
        'headline-md': ['24px', { lineHeight: '32px', fontWeight: '700', letterSpacing: '-0.01em' }],
        'title-md': ['20px', { lineHeight: '28px', fontWeight: '700' }],
        'section-sm': ['18px', { lineHeight: '28px', fontWeight: '600' }],
        'body-base': ['16px', { lineHeight: '24px', fontWeight: '400' }],
        'label-sm': ['14px', { lineHeight: '20px', fontWeight: '500' }],
        'caption-xs': ['12px', { lineHeight: '16px', fontWeight: '400' }],
        'code-sm': ['13px', { lineHeight: '20px', fontWeight: '400' }],
      },
      spacing: {
        'base-unit': '4px',
        gutter: '16px',
        'margin-mobile': '16px',
        'margin-desktop': '32px',
        'sidebar-width': '256px',
        'container-padding': '24px',
      },
      borderRadius: {
        DEFAULT: '0.25rem',
        lg: '0.5rem',
        xl: '0.75rem',
      },
    },
  },
  plugins: [],
} satisfies Config;
```

- [ ] **Step 6: Create `postcss.config.js`**

```js
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 7: Create `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Godwit</title>
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400&display=swap" rel="stylesheet" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 8: Create `vitest.config.ts`**

```ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: false,
  },
});
```

- [ ] **Step 9: Create `src/main.tsx`**

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/index.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 10: Create `src/vite-env.d.ts`**

```ts
/// <reference types="vite/client" />
```

- [ ] **Step 11: Create `src/styles/index.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  body {
    @apply font-sans text-on-surface bg-surface-container-low antialiased;
  }
}

@layer utilities {
  .ambient-shadow {
    box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px -1px rgba(0, 0, 0, 0.1);
  }

  .hairline-border {
    border: 1px solid #e5e7eb;
  }
}
```

- [ ] **Step 12: Create `src/test/setup.ts`**

```ts
import '@testing-library/jest-dom/vitest';
```

- [ ] **Step 13: Create temporary `src/App.tsx` for scaffold verification**

```tsx
// apps/ui/src/App.tsx (temporary; replaced in Task 6)
export default function App() {
  return <div className="p-8 text-headline-md text-primary">Godwit UI scaffold OK</div>;
}
```

- [ ] **Step 14: Create temporary `src/App.test.tsx` placeholder**

```tsx
// apps/ui/src/App.test.tsx (temporary; replaced in Task 9)
import { render, screen } from '@testing-library/react';
import App from './App';

describe('App scaffold', () => {
  it('renders the scaffold message', () => {
    render(<App />);
    expect(screen.getByText('Godwit UI scaffold OK')).toBeInTheDocument();
  });
});
```

- [ ] **Step 15: Install dependencies**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm install
```

- [ ] **Step 15: Verify dev server starts**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run dev &
DEV_PID=$!
sleep 5
curl -s -o /dev/null -w "%{http_code}" http://localhost:3001/
kill $DEV_PID
```

Expected: HTTP 200.

---

### Task 3: Auth Core

**Files:**
- Create: `apps/ui/src/types/index.ts`
- Create: `apps/ui/src/lib/auth.ts`
- Create: `apps/ui/src/lib/http.ts`
- Create: `apps/ui/src/store/auth.ts`
- Create: `apps/ui/src/lib/auth.test.ts`
- Create: `apps/ui/src/lib/http.test.ts`

**Interfaces:**
- Consumes: `fetch` with cookies, backend endpoints `/api/v1/auth/*`.
- Produces: `AuthUser`, `login()`, `logout()`, `fetchMe()`, `apiFetch()`, `useAuthStore`, `UnauthorizedError`.

- [ ] **Step 1: Create types**

```ts
// apps/ui/src/types/index.ts
export type UserRole = 'super_admin' | 'org_admin' | 'team_admin' | 'user';

export interface AuthUser {
  id: string;
  email: string;
  role: UserRole;
  organization_id: string | null;
}

export function isRole(user: AuthUser | null, role: UserRole): boolean {
  return user?.role === role;
}
```

- [ ] **Step 2: Implement `lib/auth.ts`**

```ts
// apps/ui/src/lib/auth.ts
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
```

- [ ] **Step 3: Implement `lib/http.ts`**

```ts
// apps/ui/src/lib/http.ts
export class UnauthorizedError extends Error {
  constructor() {
    super('Unauthorized');
    this.name = 'UnauthorizedError';
  }
}

let refreshPromise: Promise<boolean> | null = null;

async function doRefresh(): Promise<boolean> {
  try {
    const r = await fetch('/api/v1/auth/refresh', {
      method: 'POST',
      credentials: 'include',
    });
    return r.ok;
  } catch {
    return false;
  }
}

export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const merged: RequestInit = { ...init, credentials: 'include' };
  const res = await fetch(path, merged);

  if (res.status !== 401) {
    return res;
  }

  if (!refreshPromise) {
    refreshPromise = doRefresh().finally(() => {
      refreshPromise = null;
    });
  }

  const ok = await refreshPromise;
  if (!ok) {
    throw new UnauthorizedError();
  }

  return fetch(path, merged);
}
```

- [ ] **Step 4: Implement `store/auth.ts`**

```ts
// apps/ui/src/store/auth.ts
import { create } from 'zustand';
import type { AuthUser } from '@/types';

export type AuthStatus = 'unknown' | 'authenticated' | 'unauthenticated';

interface AuthState {
  user: AuthUser | null;
  status: AuthStatus;
  setUser: (user: AuthUser | null) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  status: 'unknown',
  setUser: (user) =>
    set({
      user,
      status: user ? 'authenticated' : 'unauthenticated',
    }),
}));
```

- [ ] **Step 5: Write failing tests for `login`, `fetchMe`, and `logout`**

```ts
// apps/ui/src/lib/auth.test.ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { login, logout, fetchMe } from './auth';

const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('auth', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('login posts credentials and returns user from /auth/me', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ access_token: 'at', refresh_token: 'rt' }),
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          user: { id: '1', email: 'a@b.com', role: 'super_admin', organization_id: null },
        }),
      } as Response);

    const user = await login('a@b.com', 'pw');

    expect(mockFetch).toHaveBeenNthCalledWith(
      1,
      '/api/v1/auth/login',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        body: JSON.stringify({ email: 'a@b.com', password: 'pw' }),
      })
    );
    expect(user.email).toBe('a@b.com');
  });

  it('login throws on 401', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 } as Response);
    await expect(login('a@b.com', 'pw')).rejects.toThrow('Invalid credentials');
  });

  it('fetchMe returns user on 200', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        user: { id: '1', email: 'a@b.com', role: 'super_admin', organization_id: null },
      }),
    } as Response);

    const user = await fetchMe();

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/auth/me',
      expect.objectContaining({ credentials: 'include' })
    );
    expect(user?.email).toBe('a@b.com');
  });

  it('fetchMe returns null on 401', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 } as Response);
    const user = await fetchMe();
    expect(user).toBeNull();
  });

  it('logout posts to /auth/logout with credentials', async () => {
    mockFetch.mockResolvedValueOnce({ ok: true } as Response);
    await logout();
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/auth/logout',
      expect.objectContaining({ method: 'POST', credentials: 'include' })
    );
  });
});
```

- [ ] **Step 6: Write failing test for `apiFetch` auto-refresh**

```ts
// apps/ui/src/lib/http.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { apiFetch, UnauthorizedError } from './http';

const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('apiFetch', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('returns non-401 responses directly', async () => {
    mockFetch.mockResolvedValueOnce({ status: 200, ok: true } as Response);
    const res = await apiFetch('/api/v1/models');
    expect(res.status).toBe(200);
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });

  it('refreshes on 401 and retries once', async () => {
    mockFetch
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response);

    const res = await apiFetch('/api/v1/models');
    expect(res.status).toBe(200);
    expect(mockFetch).toHaveBeenCalledTimes(3);
    expect(mockFetch).toHaveBeenNthCalledWith(2, '/api/v1/auth/refresh', expect.objectContaining({ method: 'POST' }));
  });

  it('throws UnauthorizedError when refresh fails', async () => {
    mockFetch
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 401, ok: false } as Response);

    await expect(apiFetch('/api/v1/models')).rejects.toBeInstanceOf(UnauthorizedError);
  });

  it('dedups concurrent refresh calls', async () => {
    mockFetch
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 401 } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response)
      .mockResolvedValueOnce({ status: 200, ok: true } as Response);

    const [a, b] = await Promise.all([apiFetch('/a'), apiFetch('/b')]);
    expect(a.status).toBe(200);
    expect(b.status).toBe(200);
    expect(mockFetch).toHaveBeenCalledTimes(5);
  });
});
```

- [ ] **Step 7: Run auth tests**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run test
```

Expected: all tests pass.

---

### Task 5: Auth Components & Hook

**Files:**
- Create: `apps/ui/src/hooks/useAuth.ts`
- Create: `apps/ui/src/components/auth/RequireRole.tsx`
- Create: `apps/ui/src/components/auth/LoginForm.tsx`
- Create: `apps/ui/src/components/auth/LoginForm.test.tsx`
- Create: `apps/ui/src/components/auth/RequireRole.test.tsx`

**Interfaces:**
- Consumes: `useAuthStore`, `login()`, `logout()`, `fetchMe()`, `AuthUser`, `UserRole`.
- Produces: `useAuthInit()`, `RequireRole`, `LoginForm`.

- [ ] **Step 1: Implement `useAuthInit`**

```ts
// apps/ui/src/hooks/useAuth.ts
import { useEffect } from 'react';
import { useAuthStore } from '@/store/auth';
import { fetchMe, logout } from '@/lib/auth';

export function useAuthInit() {
  const setUser = useAuthStore((state) => state.setUser);

  useEffect(() => {
    fetchMe()
      .then(setUser)
      .catch(() => setUser(null));
  }, [setUser]);
}

export function useLogout() {
  const setUser = useAuthStore((state) => state.setUser);

  return async () => {
    await logout();
    setUser(null);
  };
}
```

- [ ] **Step 2: Implement `RequireRole`**

```tsx
// apps/ui/src/components/auth/RequireRole.tsx
import { Navigate, useLocation } from 'react-router-dom';
import { useAuthStore } from '@/store/auth';
import type { UserRole } from '@/types';

interface RequireRoleProps {
  allowed: UserRole[];
  fallback?: string;
  children: React.ReactNode;
}

export function RequireRole({ allowed, fallback = '/login', children }: RequireRoleProps) {
  const user = useAuthStore((state) => state.user);
  const status = useAuthStore((state) => state.status);
  const location = useLocation();

  if (status === 'unknown') {
    return (
      <div className="flex h-screen items-center justify-center text-on-surface-variant">
        Loading…
      </div>
    );
  }

  if (!user || !allowed.includes(user.role)) {
    return <Navigate to={fallback} state={{ from: location }} replace />;
  }

  return <>{children}</>;
}
```

- [ ] **Step 3: Implement `LoginForm`**

```tsx
// apps/ui/src/components/auth/LoginForm.tsx
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';

const schema = z.object({
  email: z.string().email(),
  password: z.string().min(1, 'Password is required'),
});

type FormData = z.infer<typeof schema>;

interface LoginFormProps {
  onSubmit: (email: string, password: string) => Promise<void>;
}

export function LoginForm({ onSubmit }: LoginFormProps) {
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<FormData>({ resolver: zodResolver(schema) });

  const handleFormSubmit = handleSubmit(async (data) => {
    setBusy(true);
    setSubmitError(null);
    try {
      await onSubmit(data.email, data.password);
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setBusy(false);
    }
  });

  return (
    <form onSubmit={handleFormSubmit} className="w-full max-w-sm space-y-4">
      <Input
        label="Email"
        type="email"
        autoComplete="email"
        error={errors.email?.message}
        {...register('email')}
      />
      <Input
        label="Password"
        type="password"
        autoComplete="current-password"
        error={errors.password?.message}
        {...register('password')}
      />
      {submitError && <p className="text-label-sm text-error">{submitError}</p>}
      <Button type="submit" className="w-full" disabled={busy}>
        {busy ? 'Signing in…' : 'Sign in'}
      </Button>
    </form>
  );
}
```

- [ ] **Step 4: Test `RequireRole`**

```tsx
// apps/ui/src/components/auth/RequireRole.test.tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import { RequireRole } from './RequireRole';
import { useAuthStore } from '@/store/auth';
import type { AuthUser } from '@/types';

function setUser(user: AuthUser | null) {
  useAuthStore.getState().setUser(user);
}

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
```

- [ ] **Step 5: Test `LoginForm`**

```tsx
// apps/ui/src/components/auth/LoginForm.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LoginForm } from './LoginForm';

describe('LoginForm', () => {
  it('submits email and password', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByLabelText(/email/i), 'a@b.com');
    await userEvent.type(screen.getByLabelText(/password/i), 'password');
    await userEvent.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith('a@b.com', 'password');
    });
  });

  it('displays error on failed login', async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error('Invalid credentials'));
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByLabelText(/email/i), 'a@b.com');
    await userEvent.type(screen.getByLabelText(/password/i), 'password');
    await userEvent.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() => {
      expect(screen.getByText('Invalid credentials')).toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 6: Run tests**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run test
```

---

### Task 4: UI Primitives

**Files:**
- Create: `apps/ui/src/components/ui/Button.tsx`
- Create: `apps/ui/src/components/ui/Button.test.tsx`
- Create: `apps/ui/src/components/ui/Card.tsx`
- Create: `apps/ui/src/components/ui/Input.tsx`
- Create: `apps/ui/src/components/ui/Badge.tsx`
- Create: `apps/ui/src/components/ui/Table.tsx`
- Create: `apps/ui/src/components/ui/PageHeader.tsx`
- Create: `apps/ui/src/components/ui/EmptyState.tsx`

**Interfaces:**
- Produces: reusable base components used by layouts and pages.

- [ ] **Step 1: Button component**

```tsx
// apps/ui/src/components/ui/Button.tsx
import { forwardRef, ButtonHTMLAttributes } from 'react';
import { clsx } from '@/lib/utils';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={clsx(
          'inline-flex items-center justify-center font-medium rounded transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed',
          {
            'bg-primary text-on-primary hover:bg-primary/90': variant === 'primary',
            'bg-surface-container-lowest hairline-border text-on-surface hover:bg-surface-container-low': variant === 'secondary',
            'bg-transparent text-on-surface hover:bg-surface-container-high': variant === 'ghost',
            'bg-error text-on-error hover:bg-error/90': variant === 'danger',
            'text-label-sm px-3 py-1.5': size === 'sm',
            'text-body-base px-4 py-2': size === 'md',
            'text-title-md px-6 py-3': size === 'lg',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Button.displayName = 'Button';
```

- [ ] **Step 2: Utility `clsx`**

```ts
// apps/ui/src/lib/utils.ts
export type ClassValue = string | number | boolean | undefined | null | ClassValue[] | { [key: string]: boolean | undefined | null };

export function clsx(...inputs: ClassValue[]): string {
  const classes: string[] = [];
  for (const input of inputs) {
    if (!input) continue;
    if (typeof input === 'string' || typeof input === 'number') {
      classes.push(String(input));
    } else if (Array.isArray(input)) {
      classes.push(clsx(...input));
    } else if (typeof input === 'object') {
      for (const [key, value] of Object.entries(input)) {
        if (value) classes.push(key);
      }
    }
  }
  return classes.join(' ');
}
```

- [ ] **Step 3: Input component**

```tsx
// apps/ui/src/components/ui/Input.tsx
import { forwardRef, InputHTMLAttributes, useId } from 'react';
import { clsx } from '@/lib/utils';

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, label, error, id, ...props }, ref) => {
    const generatedId = useId();
    const inputId = id || generatedId;

    return (
      <div className="flex flex-col gap-1">
        {label && (
          <label htmlFor={inputId} className="text-label-sm font-medium text-on-surface-variant">
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={clsx(
            'bg-surface-container-lowest hairline-border rounded px-3 py-2 text-body-base text-on-surface',
            'focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent',
            'placeholder:text-on-surface-variant/50',
            error && 'border-error focus:ring-error',
            className
          )}
          {...props}
        />
        {error && <span className="text-caption-xs text-error">{error}</span>}
      </div>
    );
  }
);

Input.displayName = 'Input';
```

- [ ] **Step 4: Card component**

```tsx
// apps/ui/src/components/ui/Card.tsx
import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from '@/lib/utils';

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'elevated' | 'outlined' | 'filled';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = 'elevated', ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={clsx(
          'rounded-xl p-container-padding',
          {
            'bg-surface-container-lowest ambient-shadow': variant === 'elevated',
            'bg-surface-container-lowest hairline-border': variant === 'outlined',
            'bg-surface-container-low': variant === 'filled',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Card.displayName = 'Card';
```

- [ ] **Step 5: Badge component**

```tsx
// apps/ui/src/components/ui/Badge.tsx
import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from '@/lib/utils';

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
}

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant = 'default', ...props }, ref) => {
    return (
      <span
        ref={ref}
        className={clsx(
          'inline-flex items-center rounded-full px-2 py-1 text-caption-xs font-medium',
          {
            'bg-surface-container-high text-on-surface-variant': variant === 'default',
            'bg-success/10 text-success': variant === 'success',
            'bg-warning/10 text-warning': variant === 'warning',
            'bg-error/10 text-error': variant === 'error',
            'bg-info/10 text-info': variant === 'info',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Badge.displayName = 'Badge';
```

- [ ] **Step 6: Table components**

```tsx
// apps/ui/src/components/ui/Table.tsx
import { forwardRef, HTMLAttributes } from 'react';
import { clsx } from '@/lib/utils';

export const Table = forwardRef<HTMLTableElement, HTMLAttributes<HTMLTableElement>>(
  ({ className, ...props }, ref) => (
    <div className="overflow-x-auto">
      <table ref={ref} className={clsx('w-full text-left border-collapse', className)} {...props} />
    </div>
  )
);
Table.displayName = 'Table';

export const TableHead = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...props }, ref) => (
    <thead ref={ref} className={clsx('bg-surface-container-low', className)} {...props} />
  )
);
TableHead.displayName = 'TableHead';

export const TableBody = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...props }, ref) => <tbody ref={ref} className={clsx(className)} {...props} />
);
TableBody.displayName = 'TableBody';

export const TableRow = forwardRef<HTMLTableRowElement, HTMLAttributes<HTMLTableRowElement>>(
  ({ className, ...props }, ref) => (
    <tr
      ref={ref}
      className={clsx('border-b hairline-border hover:bg-surface-container-low transition-colors', className)}
      {...props}
    />
  )
);
TableRow.displayName = 'TableRow';

export const TableHeadCell = forwardRef<HTMLTableCellElement, HTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...props }, ref) => (
    <th
      ref={ref}
      className={clsx(
        'py-3 px-6 text-left text-caption-xs font-medium text-on-surface-variant uppercase tracking-wider',
        className
      )}
      {...props}
    />
  )
);
TableHeadCell.displayName = 'TableHeadCell';

export const TableCell = forwardRef<HTMLTableCellElement, HTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...props }, ref) => (
    <td ref={ref} className={clsx('py-3 px-6 text-body-base', className)} {...props} />
  )
);
TableCell.displayName = 'TableCell';
```

- [ ] **Step 7: PageHeader component**

```tsx
// apps/ui/src/components/ui/PageHeader.tsx
interface PageHeaderProps {
  title: string;
  description?: string;
  action?: React.ReactNode;
}

export function PageHeader({ title, description, action }: PageHeaderProps) {
  return (
    <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between border-b hairline-border pb-4 mb-6">
      <div>
        <h1 className="text-headline-md text-on-surface">{title}</h1>
        {description && <p className="text-body-base text-on-surface-variant mt-1">{description}</p>}
      </div>
      {action && <div className="mt-4 sm:mt-0">{action}</div>}
    </div>
  );
}
```

- [ ] **Step 8: EmptyState component**

```tsx
// apps/ui/src/components/ui/EmptyState.tsx
import { Card } from './Card';

interface EmptyStateProps {
  title?: string;
  message?: string;
  action?: React.ReactNode;
}

export function EmptyState({
  title = 'Nothing here',
  message = 'No items to display.',
  action,
}: EmptyStateProps) {
  return (
    <Card className="flex flex-col items-center justify-center py-16 text-center">
      <h3 className="text-section-sm text-on-surface">{title}</h3>
      <p className="text-body-base text-on-surface-variant mt-2">{message}</p>
      {action && <div className="mt-6">{action}</div>}
    </Card>
  );
}
```

- [ ] **Step 9: Test Button**

```tsx
// apps/ui/src/components/ui/Button.test.tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button } from './Button';

describe('Button', () => {
  it('renders primary variant by default', () => {
    render(<Button>Click</Button>);
    const button = screen.getByRole('button');
    expect(button).toHaveClass('bg-primary');
  });

  it('applies secondary variant', () => {
    render(<Button variant="secondary">Click</Button>);
    const button = screen.getByRole('button');
    expect(button).toHaveClass('hairline-border');
  });
});
```

- [ ] **Step 10: Run tests**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run test
```

---

### Task 6: Router + App

**Files:**
- Create: `apps/ui/src/App.tsx`
- Create: `apps/ui/src/routes/login.tsx`
- Create: `apps/ui/src/routes/admin/index.tsx`
- Create: `apps/ui/src/routes/admin/models.tsx`
- Create: `apps/ui/src/routes/admin/provider-profiles.tsx`
- Create: `apps/ui/src/routes/admin/users.tsx`
- Create: `apps/ui/src/routes/admin/keys.tsx`
- Create: `apps/ui/src/routes/admin/usage.tsx`
- Create: `apps/ui/src/routes/admin/settings.tsx`
- Create: `apps/ui/src/routes/console/index.tsx`
- Create: `apps/ui/src/routes/console/models.tsx`
- Create: `apps/ui/src/routes/console/keys.tsx`
- Create: `apps/ui/src/routes/console/usage.tsx`
- Create: `apps/ui/src/routes/console/organization.tsx`
- Create: `apps/ui/src/routes/console/team.tsx`

**Interfaces:**
- Consumes: `useAuthInit`, `RequireRole`, `LoginForm`, layouts.
- Produces: fully wired router with all routes and guards.

- [ ] **Step 1: Implement `App.tsx`**

```tsx
// apps/ui/src/App.tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAuthInit } from '@/hooks/useAuth';
import { RequireRole } from '@/components/auth/RequireRole';
import { AuthLayout } from '@/layouts/AuthLayout';
import { AdminLayout } from '@/layouts/AdminLayout';
import { ConsoleLayout } from '@/layouts/ConsoleLayout';
import { LoginPage } from '@/routes/login';
import { AdminDashboard } from '@/routes/admin';
import { AdminModels } from '@/routes/admin/models';
import { AdminProviderProfiles } from '@/routes/admin/provider-profiles';
import { AdminUsers } from '@/routes/admin/users';
import { AdminKeys } from '@/routes/admin/keys';
import { AdminUsage } from '@/routes/admin/usage';
import { AdminSettings } from '@/routes/admin/settings';
import { ConsoleDashboard } from '@/routes/console';
import { ConsoleModels } from '@/routes/console/models';
import { ConsoleKeys } from '@/routes/console/keys';
import { ConsoleUsage } from '@/routes/console/usage';
import { ConsoleOrganization } from '@/routes/console/organization';
import { ConsoleTeam } from '@/routes/console/team';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function AppRouter() {
  useAuthInit();

  return (
    <Routes>
      <Route element={<AuthLayout />}>
        <Route path="/login" element={<LoginPage />} />
      </Route>

      <Route
        element={
          <RequireRole allowed={['super_admin']} fallback="/console">
            <AdminLayout />
          </RequireRole>
        }
      >
        <Route path="/admin" element={<AdminDashboard />} />
        <Route path="/admin/models" element={<AdminModels />} />
        <Route path="/admin/provider-profiles" element={<AdminProviderProfiles />} />
        <Route path="/admin/users" element={<AdminUsers />} />
        <Route path="/admin/keys" element={<AdminKeys />} />
        <Route path="/admin/usage" element={<AdminUsage />} />
        <Route path="/admin/settings" element={<AdminSettings />} />
      </Route>

      <Route
        element={
          <RequireRole allowed={['super_admin', 'org_admin', 'team_admin', 'user']}>
            <ConsoleLayout />
          </RequireRole>
        }
      >
        <Route path="/console" element={<ConsoleDashboard />} />
        <Route path="/console/models" element={<ConsoleModels />} />
        <Route path="/console/keys" element={<ConsoleKeys />} />
        <Route path="/console/usage" element={<ConsoleUsage />} />
        <Route path="/console/organization" element={<ConsoleOrganization />} />
        <Route path="/console/team" element={<ConsoleTeam />} />
      </Route>

      <Route path="*" element={<Navigate to="/console" replace />} />
    </Routes>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppRouter />
      </BrowserRouter>
    </QueryClientProvider>
  );
}
```

- [ ] **Step 2: Implement `routes/login.tsx`**

```tsx
// apps/ui/src/routes/login.tsx
import { useNavigate, useLocation } from 'react-router-dom';
import { login } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';
import { LoginForm } from '@/components/auth/LoginForm';
import { Card } from '@/components/ui/Card';

export function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const setUser = useAuthStore((state) => state.setUser);

  const from = (location.state as { from?: { pathname?: string } } | null)?.from?.pathname || '/';

  const handleLogin = async (email: string, password: string) => {
    const user = await login(email, password);
    setUser(user);
    const destination = user.role === 'super_admin' ? '/admin' : '/console';
    navigate(from !== '/' ? from : destination, { replace: true });
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-surface-container-low px-4">
      <Card className="w-full max-w-sm">
        <div className="mb-6 text-center">
          <h1 className="text-headline-md text-on-surface">Sign in to Godwit</h1>
          <p className="text-body-base text-on-surface-variant mt-1">Admin & user console</p>
        </div>
        <LoginForm onSubmit={handleLogin} />
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Create placeholder pages**

All route files export a simple component. Example:

```tsx
// apps/ui/src/routes/admin/index.tsx
import { PageHeader } from '@/components/ui/PageHeader';

export function AdminDashboard() {
  return (
    <div>
      <PageHeader title="Admin Dashboard" description="Instance-wide overview" />
      <p className="text-body-base text-on-surface-variant">Admin dashboard content coming soon.</p>
    </div>
  );
}
```

Repeat for every route file. Each should just render `PageHeader` and a placeholder paragraph.

- [ ] **Step 4: Verify routing**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run type-check
npm run test
```

---

### Task 7: Layouts

**Files:**
- Create: `apps/ui/src/layouts/AuthLayout.tsx`
- Create: `apps/ui/src/layouts/AdminLayout.tsx`
- Create: `apps/ui/src/layouts/ConsoleLayout.tsx`
- Create: `apps/ui/src/layouts/AdminSidebar.tsx`
- Create: `apps/ui/src/layouts/ConsoleSidebar.tsx`
- Create: `apps/ui/src/layouts/TopBar.tsx`

**Interfaces:**
- Consumes: `useAuthStore`, `useLogout`, UI primitives.
- Produces: `AdminLayout`, `ConsoleLayout`, `AuthLayout`.

- [ ] **Step 1: `AuthLayout`**

```tsx
// apps/ui/src/layouts/AuthLayout.tsx
import { Outlet } from 'react-router-dom';

export function AuthLayout() {
  return (
    <div className="min-h-screen bg-surface-container-low">
      <Outlet />
    </div>
  );
}
```

- [ ] **Step 2: `TopBar`**

```tsx
// apps/ui/src/layouts/TopBar.tsx
import { useAuthStore } from '@/store/auth';
import { useLogout } from '@/hooks/useAuth';
import { Button } from '@/components/ui/Button';

export function TopBar() {
  const user = useAuthStore((state) => state.user);
  const logout = useLogout();

  return (
    <header className="sticky top-0 z-40 flex h-16 items-center justify-between border-b hairline-border bg-surface-container-lowest px-6">
      <div className="flex items-center gap-2">
        <span className="text-headline-md font-bold text-primary">Godwit</span>
      </div>
      <div className="flex items-center gap-4">
        {user && (
          <>
            <div className="text-right hidden sm:block">
              <p className="text-label-sm font-medium text-on-surface">{user.email}</p>
              <p className="text-caption-xs text-on-surface-variant capitalize">{user.role.replace('_', ' ')}</p>
            </div>
            <Button variant="ghost" size="sm" onClick={logout}>
              Sign out
            </Button>
          </>
        )}
      </div>
    </header>
  );
}
```

- [ ] **Step 3: `AdminSidebar`**

```tsx
// apps/ui/src/layouts/AdminSidebar.tsx
import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  Box,
  Server,
  Users,
  Key,
  BarChart3,
  Settings,
} from 'lucide-react';
import { clsx } from '@/lib/utils';

const navItems = [
  { to: '/admin', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/admin/models', label: 'Models', icon: Box },
  { to: '/admin/provider-profiles', label: 'Providers', icon: Server },
  { to: '/admin/users', label: 'Users', icon: Users },
  { to: '/admin/keys', label: 'API Keys', icon: Key },
  { to: '/admin/usage', label: 'Usage', icon: BarChart3 },
  { to: '/admin/settings', label: 'Settings', icon: Settings },
];

export function AdminSidebar() {
  return (
    <aside className="hidden md:flex w-sidebar-width flex-col border-r hairline-border bg-surface-container-lowest h-[calc(100vh-4rem)]">
      <nav className="flex-1 p-4 space-y-1">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              clsx(
                'flex items-center gap-3 rounded-lg px-4 py-3 text-label-sm font-medium transition-colors',
                isActive
                  ? 'bg-secondary-container text-on-secondary-container'
                  : 'text-on-surface-variant hover:bg-surface-container-high'
              )
            }
          >
            <item.icon className="h-4 w-4" />
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
```

- [ ] **Step 4: `ConsoleSidebar`**

```tsx
// apps/ui/src/layouts/ConsoleSidebar.tsx
import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  Box,
  Key,
  BarChart3,
  Building2,
  Users,
} from 'lucide-react';
import { useAuthStore } from '@/store/auth';
import { clsx } from '@/lib/utils';

const baseItems = [
  { to: '/console', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/console/models', label: 'Models', icon: Box },
  { to: '/console/keys', label: 'My API Keys', icon: Key },
  { to: '/console/usage', label: 'My Usage', icon: BarChart3 },
];

export function ConsoleSidebar() {
  const user = useAuthStore((state) => state.user);
  const role = user?.role;

  const items = [...baseItems];
  if (role === 'org_admin') {
    items.push({ to: '/console/organization', label: 'Organization', icon: Building2 });
  }
  if (role === 'team_admin') {
    items.push({ to: '/console/team', label: 'Team', icon: Users });
  }

  return (
    <aside className="hidden md:flex w-sidebar-width flex-col border-r hairline-border bg-surface-container-lowest h-[calc(100vh-4rem)]">
      <nav className="flex-1 p-4 space-y-1">
        {items.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              clsx(
                'flex items-center gap-3 rounded-lg px-4 py-3 text-label-sm font-medium transition-colors',
                isActive
                  ? 'bg-secondary-container text-on-secondary-container'
                  : 'text-on-surface-variant hover:bg-surface-container-high'
              )
            }
          >
            <item.icon className="h-4 w-4" />
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
```

- [ ] **Step 5: `AdminLayout`**

```tsx
// apps/ui/src/layouts/AdminLayout.tsx
import { Outlet } from 'react-router-dom';
import { TopBar } from './TopBar';
import { AdminSidebar } from './AdminSidebar';

export function AdminLayout() {
  return (
    <div className="min-h-screen bg-surface-container-low">
      <TopBar />
      <div className="flex">
        <AdminSidebar />
        <main className="flex-1 p-container-padding max-w-7xl">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
```

- [ ] **Step 6: `ConsoleLayout`**

```tsx
// apps/ui/src/layouts/ConsoleLayout.tsx
import { Outlet } from 'react-router-dom';
import { TopBar } from './TopBar';
import { ConsoleSidebar } from './ConsoleSidebar';

export function ConsoleLayout() {
  return (
    <div className="min-h-screen bg-surface-container-low">
      <TopBar />
      <div className="flex">
        <ConsoleSidebar />
        <main className="flex-1 p-container-padding max-w-7xl">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
```

- [ ] **Step 7: Test sidebar conditional navigation**

```tsx
// apps/ui/src/layouts/ConsoleSidebar.test.tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { ConsoleSidebar } from './ConsoleSidebar';
import { useAuthStore } from '@/store/auth';
import type { AuthUser } from '@/types';

function setUser(user: AuthUser | null) {
  useAuthStore.getState().setUser(user);
}

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
```

- [ ] **Step 8: Run tests**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run test
```

---

### Task 8: Build Verification & Final QA

**Files:**
- Create: `apps/ui/.env.example`
- Modify: `apps/ui/vite.config.ts` if proxy needs adjustment.

**Interfaces:**
- Produces: passing build, passing tests, dev server starts.

- [ ] **Step 1: Add `.env.example`**

```
# API origin for the Vite dev proxy
VITE_API_URL=http://localhost:3000
```

- [ ] **Step 2: Type check**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run type-check
```

Expected: no errors.

- [ ] **Step 3: Run all tests**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run test
```

Expected: all tests pass.

- [ ] **Step 4: Build**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run build
```

Expected: `dist/` is created with no errors.

- [ ] **Step 5: Dev server smoke test**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run dev &
DEV_PID=$!
sleep 5
curl -s http://localhost:3001/ | head -c 200
kill $DEV_PID
```

Expected: page HTML contains `<title>Godwit</title>` and React root div.

- [ ] **Step 6: Update `apps/ui/README.md`**

```markdown
# Godwit UI

Single-page application for Godwit admin and user console.

## Setup

```bash
cd apps/ui
npm install
```

## Development

```bash
npm run dev
```

The dev server proxies `/api`, `/health`, `/metrics` to the backend at `VITE_API_URL` (default: `http://localhost:3000`).

## Build

```bash
npm run build
npm run preview
```

## Tests

```bash
npm run test
npm run test:watch
```
```

- [ ] **Step 7: Manual acceptance check**

Without a running backend, verify at least:
- `/login` renders the login form.
- `/admin` redirects to `/login` when unauthenticated.
- `/console` redirects to `/login` when unauthenticated.

With a running backend and valid user:
- Login as `super_admin` → redirect to `/admin`, sidebar admin visible.
- Login as `user` → redirect to `/console`, sidebar console visible without Organization/Team links.

---

### Task 9: Comprehensive Unit Tests

**Files:**
- Create: `apps/ui/src/lib/utils.test.ts`
- Create: `apps/ui/src/store/auth.test.ts`
- Create: `apps/ui/src/hooks/useAuth.test.ts`
- Create: `apps/ui/src/components/ui/Input.test.tsx`
- Create: `apps/ui/src/components/ui/Card.test.tsx`
- Create: `apps/ui/src/components/ui/Badge.test.tsx`
- Create: `apps/ui/src/components/ui/Table.test.tsx`
- Create: `apps/ui/src/components/ui/PageHeader.test.tsx`
- Create: `apps/ui/src/components/ui/EmptyState.test.tsx`
- Create: `apps/ui/src/layouts/TopBar.test.tsx`

**Interfaces:**
- Every new component, hook, lib module, and store introduced in Tasks 2-7 has meaningful unit tests.

- [ ] **Step 1: Test `clsx` utility in `lib/utils.test.ts`**

Cover strings, arrays, objects, mixed arguments, and falsy values.

- [ ] **Step 2: Test Zustand auth store in `store/auth.test.ts`**

Cover initial state, `setUser` with authenticated user, and `setUser(null)` for logout.

- [ ] **Step 3: Test `useAuthInit` and `useLogout` in `hooks/useAuth.test.ts`**

Mock `global.fetch` and the Zustand store. Verify that `useAuthInit` populates the store on success and clears it on 401; `useLogout` posts to `/auth/logout` and clears the store.

- [ ] **Step 4: Add tests for remaining UI primitives**

For each component test behavior, not just rendering:
- `Input`: label association, value change, error message display.
- `Card`: renders children, handles variants (`default`, `outlined`, `filled`).
- `Badge`: renders text, respects variants (`default`, `primary`, `success`, `warning`, `danger`).
- `Table`: renders `Table`, `TableHead`, `TableBody`, `TableRow`, `TableCell`.
- `PageHeader`: renders title, description, and action slot.
- `EmptyState`: renders title, message, and action slot.

- [ ] **Step 5: Test `TopBar` layout in `layouts/TopBar.test.tsx`**

Verify that the bar displays the user email and role, and that the logout button calls `useLogout`.

- [ ] **Step 6: Run all tests**

```bash
cd /home/thomas/work/Godwit/apps/ui
npm run test
```

Expected: all tests pass (target 50+ tests across 17 files).

---

### Task 10: Update `AGENTS.md` Testing Policy

**Files:**
- Edit: `/home/thomas/work/Godwit/AGENTS.md`

- [ ] **Step 1: Add a `Testing policy` section**

Add a new `Testing policy` section before `Testing quirks` with the following rules:
- Tests are not optional.
- New features must ship with focused unit tests covering happy path and meaningful failure modes.
- Bug fixes must ship with a regression test that fails before the fix and passes after.
- Frontend: co-located unit tests using Vitest + React Testing Library.
- Backend: unit/integration tests using `cargo test`.
- Coverage is a side effect, not a goal.
- Tests must fail if the implementation changes in a way that breaks the feature (mutation testing mindset).

- [ ] **Step 2: Verify `AGENTS.md` formatting**

Run a markdown linter or at least confirm the file renders correctly in the repository preview.

---

## Self-Review

**1. Spec coverage:**
- ✅ Cleanup old apps (Task 1)
- ✅ Vite scaffold (Task 2)
- ✅ Design tokens (Task 2/4)
- ✅ Cookie auth via `lib/auth.ts` + `lib/http.ts` (Task 3)
- ✅ Auth store Zustand (Task 3)
- ✅ UI primitives (Task 4)
- ✅ Login form + guards (Task 5)
- ✅ Router with `/admin/*` and `/console/*` (Task 6)
- ✅ Admin & Console layouts with conditional sidebar (Task 7)
- ✅ Build verification (Task 8)
- ✅ Comprehensive tests for auth, http, guards, components, hooks, store, sidebar, lib utils, layouts (Task 9)
- ✅ `AGENTS.md` updated with testing policy (Task 10)

**2. Placeholder scan:**
- No TBD/TODO in code steps.
- Placeholder `App.test.tsx` exists for scaffold verification and is replaced by meaningful `App.test.tsx` smoke test in Task 9.
- Placeholder pages are explicitly empty and labelled "coming soon".

**3. Type consistency:**
- `AuthUser` defined in `types/index.ts` and used everywhere.
- `UserRole` union used in `RequireRole`, layouts, routes.
- `apiFetch` signature consistent across hooks.

**4. Dependencies:**
- `@hookform/resolvers` declared in Task 2 dependencies.
- `lucide-react` icons used in sidebars.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-08-10-godwit-ui-auth-shell.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using `executing-plans`, batch execution with checkpoints.

**Which approach?**
