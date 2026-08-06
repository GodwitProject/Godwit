# Godwit UI — Phase 1 (MVP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build MVP of Godwit UI with Dashboard (read-only metrics), Providers list, API Keys CRUD, and basic Logs viewer — deployable in 2 weeks.

**Architecture:** Next.js 14 App Router (mono-repo `apps/ui/`), Tailwind CSS, React Query for state, WebSocket for real-time metrics.

**Tech Stack:**
- Next.js 14 (TypeScript)
- Tailwind CSS 3.4
- React Query (TanStack Query)
- Zustand (client state)
- Recharts (charts)
- TanStack Table (data tables)
- WebSocket (native API)

## Global Constraints

- **Design System:** Use Godwit colors from DESIGN.md (Cobalt Blue `#2563eb`, Inter font, JetBrains Mono for code)
- **Layout:** Fixed sidebar (256px) + fluid main content, responsive mobile nav
- **API Base:** `http://localhost:3000/api/v1` (configurable via `NEXT_PUBLIC_API_URL`)
- **WebSocket:** `ws://localhost:3000/api/v1/ws/metrics` for real-time updates
- **Performance:** Initial load <3s, bundle <500KB
- **Testing:** Unit tests for components, integration tests for API calls
- **YAGNI:** No advanced filters, no export, no mobile app in Phase 1

---

## File Structure

**New Directories:**
- `apps/ui/` — Next.js application root
- `apps/ui/src/components/ui/` — Base components (Button, Card, Input, Table, etc.)
- `apps/ui/src/components/layout/` — Shell, Sidebar, Header, MobileNav
- `apps/ui/src/components/metrics/` — MetricCard, TimeSeriesChart
- `apps/ui/src/lib/` — API client, WebSocket manager, utils, types
- `apps/ui/src/hooks/` — React Query hooks, custom hooks

**Modified:**
- None (UI is isolated in `apps/ui/`)

---

### Task 1: Scaffold Next.js Project

**Files:**
- Create: `apps/ui/package.json`
- Create: `apps/ui/tsconfig.json`
- Create: `apps/ui/tailwind.config.ts`
- Create: `apps/ui/next.config.js`
- Create: `apps/ui/src/app/layout.tsx`
- Create: `apps/ui/src/app/page.tsx`
- Create: `apps/ui/src/styles/globals.css`

**Interfaces:**
- Consumes: None (greenfield)
- Produces: Working Next.js 14 app with Tailwind configured

- [ ] **Step 1: Create package.json**

```json
{
  "name": "godwit-ui",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "type-check": "tsc --noEmit"
  },
  "dependencies": {
    "next": "14.2.5",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "@tanstack/react-query": "^5.51.0",
    "zustand": "^4.5.4",
    "recharts": "^2.12.7",
    "@tanstack/react-table": "^8.19.3"
  },
  "devDependencies": {
    "@types/node": "^20.14.11",
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "typescript": "^5.5.3",
    "tailwindcss": "^3.4.6",
    "postcss": "^8.4.39",
    "autoprefixer": "^10.4.19",
    "eslint": "^8.57.0",
    "eslint-config-next": "^14.2.5"
  }
}
```

- [ ] **Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
```

- [ ] **Step 3: Create tailwind.config.ts**

```typescript
import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        // Surface
        surface: '#f8f9fb',
        'surface-dim': '#d9dadc',
        'surface-bright': '#f8f9fb',
        'surface-container-lowest': '#ffffff',
        'surface-container-low': '#f3f4f6',
        'surface-container': '#edeef0',
        'surface-container-high': '#e7e8ea',
        'surface-container-highest': '#e1e2e4',
        
        // On Surface
        'on-surface': '#191c1e',
        'on-surface-variant': '#434655',
        
        // Primary (Godwit Cobalt Blue)
        primary: '#004ac6',
        'on-primary': '#ffffff',
        'primary-container': '#2563eb',
        'on-primary-container': '#eeefff',
        'primary-fixed': '#dbe1ff',
        'primary-fixed-dim': '#b4c5ff',
        
        // Secondary
        secondary: '#515f74',
        'on-secondary': '#ffffff',
        'secondary-container': '#d5e3fc',
        'on-secondary-container': '#57657a',
        
        // Tertiary
        tertiary: '#005a82',
        'on-tertiary': '#ffffff',
        'tertiary-container': '#0074a6',
        'on-tertiary-container': '#e4f2ff',
        
        // Error
        error: '#ba1a1a',
        'on-error': '#ffffff',
        'error-container': '#ffdad6',
        
        // Functional
        success: '#10b981',
        warning: '#f59e0b',
        info: '#3b82f6',
        
        // Borders
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
        'gutter': '16px',
        'margin-mobile': '16px',
        'margin-desktop': '32px',
        'sidebar-width': '256px',
        'container-padding': '24px',
      },
      borderRadius: {
        'DEFAULT': '0.125rem',
        'lg': '0.25rem',
        'xl': '0.5rem',
        'full': '9999px',
      },
    },
  },
  plugins: [],
};
export default config;
```

- [ ] **Step 4: Create next.config.js**

```javascript
/** @type {import('next').NextConfig} */
const nextConfig = {
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1',
    NEXT_PUBLIC_WS_URL: process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3000/api/v1/ws',
  },
};

module.exports = nextConfig;
```

- [ ] **Step 5: Create globals.css**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400&display=swap');

body {
  @apply font-sans text-on-surface bg-surface-container-low;
}

.ambient-shadow {
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px -1px rgba(0, 0, 0, 0.1);
}

.hairline-border {
  border: 1px solid #e5e7eb;
}
```

- [ ] **Step 6: Create root layout**

```typescript
// apps/ui/src/app/layout.tsx
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "../styles/globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Godwit - LLM Proxy Admin",
  description: "Admin dashboard for Godwit LLM Proxy",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={inter.className}>{children}</body>
    </html>
  );
}
```

- [ ] **Step 7: Create placeholder homepage**

```typescript
// apps/ui/src/app/page.tsx
export default function Home() {
  return (
    <main className="min-h-screen p-8">
      <h1 className="text-display-lg">Godwit UI</h1>
      <p className="text-body-base mt-4">Coming soon...</p>
    </main>
  );
}
```

- [ ] **Step 8: Install dependencies and verify**

```bash
cd apps/ui
npm install
npm run dev
# Open http://localhost:3001
```

Expected: Next.js dev server starts, page displays "Godwit UI - Coming soon"

- [ ] **Step 9: Commit**

```bash
git add apps/ui/
git commit -m "feat(ui): scaffold Next.js 14 project

- TypeScript, Tailwind CSS, ESLint configured
- Godwit design system tokens (colors, fonts, spacing)
- Base layout and placeholder homepage
- Dependencies: React Query, Zustand, Recharts, TanStack Table
"
```

---

### Task 2: Base UI Components

**Files:**
- Create: `apps/ui/src/components/ui/Button.tsx`
- Create: `apps/ui/src/components/ui/Card.tsx`
- Create: `apps/ui/src/components/ui/Input.tsx`
- Create: `apps/ui/src/components/ui/Select.tsx`
- Create: `apps/ui/src/components/ui/Badge.tsx`
- Create: `apps/ui/src/components/ui/Table.tsx`

**Interfaces:**
- Consumes: Tailwind classes from Task 1
- Produces: Reusable components for all pages

- [ ] **Step 1: Create Button component**

```typescript
// apps/ui/src/components/ui/Button.tsx
import { ButtonHTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

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
          'inline-flex items-center justify-center font-medium rounded transition-colors',
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

- [ ] **Step 2: Create Card component**

```typescript
// apps/ui/src/components/ui/Card.tsx
import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'elevated' | 'outlined' | 'filled';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = 'elevated', ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={clsx(
          'bg-surface-container-lowest rounded-xl p-container-padding',
          {
            'ambient-shadow': variant === 'elevated',
            'hairline-border': variant === 'outlined',
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

- [ ] **Step 3: Create Input component**

```typescript
// apps/ui/src/components/ui/Input.tsx
import { InputHTMLAttributes, forwardRef, useId } from 'react';
import { clsx } from 'clsx';

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
            'bg-surface-container-lowest hairline-border rounded px-3 py-2 text-body-base',
            'focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent',
            'placeholder:text-on-surface-variant/50',
            error && 'border-error focus:ring-error',
            className
          )}
          {...props}
        />
        {error && (
          <span className="text-caption-xs text-error">{error}</span>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';
```

- [ ] **Step 4: Create Badge component**

```typescript
// apps/ui/src/components/ui/Badge.tsx
import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
}

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant = 'default', ...props }, ref) => {
    return (
      <span
        ref={ref}
        className={clsx(
          'inline-flex items-center px-2 py-1 rounded-full text-caption-xs font-medium',
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

- [ ] **Step 5: Create Table components**

```typescript
// apps/ui/src/components/ui/Table.tsx
import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

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
  ({ className, ...props }, ref) => (
    <tbody ref={ref} className={clsx(className)} {...props} />
  )
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
      className={clsx('py-3 px-6 text-caption-xs font-medium text-on-surface-variant uppercase tracking-wider', className)}
      {...props}
    />
  )
);
TableHeadCell.displayName = 'TableHeadCell';

export const TableCell = forwardRef<HTMLTableCellElement, HTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...props }, ref) => (
    <td
      ref={ref}
      className={clsx('py-3 px-6 text-body-base', className)}
      {...props}
    />
  )
);
TableCell.displayName = 'TableCell';
```

- [ ] **Step 6: Add clsx dependency**

```bash
cd apps/ui
npm install clsx
```

- [ ] **Step 7: Write component tests**

```typescript
// apps/ui/src/components/ui/Button.test.tsx
import { render, screen } from '@testing-library/react';
import { Button } from './Button';

describe('Button', () => {
  it('renders with default variant', () => {
    render(<Button>Click me</Button>);
    const button = screen.getByRole('button');
    expect(button).toHaveClass('bg-primary');
  });

  it('applies secondary variant', () => {
    render(<Button variant="secondary">Click me</Button>);
    const button = screen.getByRole('button');
    expect(button).toHaveClass('hairline-border');
  });
});
```

- [ ] **Step 8: Commit**

```bash
git add apps/ui/src/components/ui/
git commit -m "feat(ui): add base components

- Button (4 variants: primary, secondary, ghost, danger)
- Card (3 variants: elevated, outlined, filled)
- Input (with label, error states)
- Badge (status colors: success, warning, error, info)
- Table (head, body, row, cells)
- All components use Godwit design tokens
"
```

---

### Task 3: Layout Shell (Sidebar + Header)

**Files:**
- Create: `apps/ui/src/components/layout/Sidebar.tsx`
- Create: `apps/ui/src/components/layout/Header.tsx`
- Create: `apps/ui/src/components/layout/Shell.tsx`
- Create: `apps/ui/src/components/layout/MobileNav.tsx`

**Interfaces:**
- Consumes: Base components from Task 2
- Produces: Layout wrapper for all pages

- [ ] **Step 1: Create Sidebar component**

```typescript
// apps/ui/src/components/layout/Sidebar.tsx
'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { clsx } from 'clsx';

const navItems = [
  { href: '/', label: 'Overview', icon: 'insights' },
  { href: '/providers', label: 'Providers', icon: 'hub' },
  { href: '/keys', label: 'API Keys', icon: 'vpn_key' },
  { href: '/logs', label: 'Logs', icon: 'list_alt' },
  { href: '/usage', label: 'Usage', icon: 'data_usage' },
  { href: '/settings', label: 'Settings', icon: 'settings' },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="hidden md:flex flex-col h-full w-sidebar-width fixed left-0 top-16 bg-surface-container-lowest border-r hairline-border py-6 z-40">
      <nav className="flex-1 flex flex-col gap-1 px-2">
        {navItems.map((item) => (
          <Link
            key={item.href}
            href={item.href}
            className={clsx(
              'rounded-full mx-2 px-4 py-3 flex items-center gap-3 transition-all',
              pathname === item.href
                ? 'bg-secondary-container text-on-secondary-container font-medium'
                : 'text-on-surface-variant hover:bg-surface-container-high'
            )}
          >
            <span className="material-symbols-outlined text-[18px]">{item.icon}</span>
            <span className="text-label-sm">{item.label}</span>
          </Link>
        ))}
      </nav>
    </aside>
  );
}
```

- [ ] **Step 2: Create Header component**

```typescript
// apps/ui/src/components/layout/Header.tsx
'use client';

import Link from 'next/link';
import { clsx } from 'clsx';

export function Header() {
  return (
    <header className="fixed top-0 w-full z-50 bg-surface border-b hairline-border h-16 flex items-center justify-between px-margin-mobile md:px-margin-desktop">
      <Link href="/" className="flex items-center gap-2 text-primary">
        <span className="material-symbols-outlined">terminal</span>
        <span className="font-headline-md font-bold">Godwit</span>
      </Link>
      
      <nav className="hidden md:flex items-center gap-6">
        <Link href="/" className="text-primary font-medium hover:bg-surface-container-high px-3 py-2 rounded-lg transition-colors">
          Dashboard
        </Link>
        <Link href="/providers" className="text-on-surface-variant hover:bg-surface-container-high px-3 py-2 rounded-lg transition-colors">
          Providers
        </Link>
        <Link href="/keys" className="text-on-surface-variant hover:bg-surface-container-high px-3 py-2 rounded-lg transition-colors">
          API Keys
        </Link>
        <Link href="/logs" className="text-on-surface-variant hover:bg-surface-container-high px-3 py-2 rounded-lg transition-colors">
          Logs
        </Link>
      </nav>

      <div className="flex items-center gap-4">
        <button className="hover:bg-surface-container-high p-1 rounded-full hairline-border">
          <img src="/avatar.png" alt="User" className="w-8 h-8 rounded-full" />
        </button>
      </div>
    </header>
  );
}
```

- [ ] **Step 3: Create Shell component**

```typescript
// apps/ui/src/components/layout/Shell.tsx
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { MobileNav } from './MobileNav';

export function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen pb-20 md:pb-0">
      <Header />
      <Sidebar />
      <main className="pt-20 px-margin-mobile md:px-margin-desktop md:ml-[256px] max-w-7xl mx-auto flex flex-col gap-8 pb-12">
        {children}
      </main>
      <MobileNav />
    </div>
  );
}
```

- [ ] **Step 4: Create MobileNav component**

```typescript
// apps/ui/src/components/layout/MobileNav.tsx
'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { clsx } from 'clsx';

const navItems = [
  { href: '/', label: 'Dashboard', icon: 'dashboard' },
  { href: '/providers', label: 'Providers', icon: 'account_tree' },
  { href: '/keys', label: 'API Keys', icon: 'vpn_key' },
  { href: '/logs', label: 'Logs', icon: 'list_alt' },
];

export function MobileNav() {
  const pathname = usePathname();

  return (
    <nav className="md:hidden fixed bottom-0 w-full z-50 bg-surface border-t hairline-border flex justify-around items-center h-16 px-2">
      {navItems.map((item) => (
        <Link
          key={item.href}
          href={item.href}
          className={clsx(
            'flex flex-col items-center justify-center p-2 w-16 transition-transform',
            pathname === item.href
              ? 'text-primary font-bold'
              : 'text-on-surface-variant'
          )}
        >
          <span className="material-symbols-outlined">{item.icon}</span>
          <span className="text-[10px] mt-1">{item.label}</span>
        </Link>
      ))}
    </nav>
  );
}
```

- [ ] **Step 5: Update root layout to use Shell**

```typescript
// apps/ui/src/app/layout.tsx
import { Shell } from '@/components/layout/Shell';

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <Shell>{children}</Shell>
      </body>
    </html>
  );
}
```

- [ ] **Step 6: Add Material Icons font**

```typescript
// apps/ui/src/app/layout.tsx (add to head)
<link
  href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:wght,FILL@100..700,0..1&display=swap"
  rel="stylesheet"
/>
```

- [ ] **Step 7: Commit**

```bash
git add apps/ui/src/components/layout/
git commit -m "feat(ui): add layout shell

- Sidebar (fixed 256px, desktop only)
- Header (fixed top, responsive)
- MobileNav (bottom bar, mobile only)
- Shell wrapper for all pages
- Material Icons integration
"
```

---

### Task 4: Dashboard Overview Page

**Files:**
- Modify: `apps/ui/src/app/page.tsx`
- Create: `apps/ui/src/components/metrics/MetricCard.tsx`
- Create: `apps/ui/src/components/metrics/TimeSeriesChart.tsx`
- Create: `apps/ui/src/lib/api.ts`
- Create: `apps/ui/src/hooks/useMetrics.ts`

**Interfaces:**
- Consumes: Layout (Task 3), Base components (Task 2)
- Produces: Working dashboard with real-time metrics

- [ ] **Step 1: Create API client**

```typescript
// apps/ui/src/lib/api.ts
const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000/api/v1';

export async function fetchMetrics() {
  const res = await fetch(`${API_BASE}/metrics/summary`);
  if (!res.ok) throw new Error('Failed to fetch metrics');
  return res.json();
}

export async function fetchLatency() {
  const res = await fetch(`${API_BASE}/metrics/latency`);
  if (!res.ok) throw new Error('Failed to fetch latency');
  return res.json();
}

export async function fetchTokens() {
  const res = await fetch(`${API_BASE}/metrics/tokens`);
  if (!res.ok) throw new Error('Failed to fetch tokens');
  return res.json();
}

export async function fetchRecentLogs(limit = 10) {
  const res = await fetch(`${API_BASE}/logs/recent?limit=${limit}`);
  if (!res.ok) throw new Error('Failed to fetch logs');
  return res.json();
}
```

- [ ] **Step 2: Create useMetrics hook**

```typescript
// apps/ui/src/hooks/useMetrics.ts
import { useQuery } from '@tanstack/react-query';
import { fetchMetrics, fetchLatency, fetchTokens } from '@/lib/api';

export function useMetrics() {
  return useQuery({
    queryKey: ['metrics'],
    queryFn: fetchMetrics,
    refetchInterval: 5000, // 5 seconds
  });
}

export function useLatency() {
  return useQuery({
    queryKey: ['latency'],
    queryFn: fetchLatency,
  });
}

export function useTokens() {
  return useQuery({
    queryKey: ['tokens'],
    queryFn: fetchTokens,
  });
}
```

- [ ] **Step 3: Create MetricCard component**

```typescript
// apps/ui/src/components/metrics/MetricCard.tsx
import { Card } from '@/components/ui/Card';
import { clsx } from 'clsx';

interface MetricCardProps {
  title: string;
  value: string;
  trend?: {
    value: string;
    direction: 'up' | 'down' | 'flat';
  };
  icon?: string;
  trendColor?: 'primary' | 'error' | 'success';
}

export function MetricCard({ title, value, trend, icon, trendColor = 'primary' }: MetricCardProps) {
  return (
    <Card className="flex flex-col">
      <div className="flex justify-between items-start mb-4">
        <span className="text-label-sm text-on-surface-variant uppercase tracking-wider">{title}</span>
        {icon && <span className="material-symbols-outlined text-outline">{icon}</span>}
      </div>
      <div className="mt-auto">
        <span className="text-display-lg text-on-surface">{value}</span>
        {trend && (
          <div className={clsx('flex items-center gap-1 mt-2 text-label-sm', trendColor === 'error' ? 'text-error' : 'text-primary')}>
            <span className="material-symbols-outlined text-[16px]">
              {trend.direction === 'up' ? 'trending_up' : trend.direction === 'down' ? 'trending_down' : 'trending_flat'}
            </span>
            <span>{trend.value}</span>
          </div>
        )}
      </div>
    </Card>
  );
}
```

- [ ] **Step 4: Create TimeSeriesChart component**

```typescript
// apps/ui/src/components/metrics/TimeSeriesChart.tsx
'use client';

import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

interface TimeSeriesChartProps {
  data: Array<{ time: string; value: number }>;
  title: string;
}

export function TimeSeriesChart({ data, title }: TimeSeriesChartProps) {
  return (
    <Card className="p-container-padding">
      <h3 className="text-section-sm mb-6">{title}</h3>
      <div className="h-[300px]">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data}>
            <XAxis dataKey="time" tick={{ fontSize: 12 }} />
            <YAxis tick={{ fontSize: 12 }} />
            <Tooltip />
            <Line type="monotone" dataKey="value" stroke="#004ac6" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
```

- [ ] **Step 5: Create RecentLogsTable component**

```typescript
// apps/ui/src/components/logs/RecentLogsTable.tsx
import { Card } from '@/components/ui/Card';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';

interface Log {
  timestamp: string;
  requestId: string;
  model: string;
  status: number;
  latencyMs: number;
}

interface RecentLogsTableProps {
  logs: Log[];
}

export function RecentLogsTable({ logs }: RecentLogsTableProps) {
  return (
    <Card className="overflow-hidden">
      <div className="p-container-padding border-b hairline-border flex justify-between items-center">
        <h3 className="text-section-sm">Recent Proxy Events</h3>
        <a href="/logs" className="text-label-sm text-primary hover:underline">View All Logs</a>
      </div>
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Timestamp</TableHeadCell>
            <TableHeadCell>Request ID</TableHeadCell>
            <TableHeadCell>Model</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell className="text-right">Latency</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {logs.map((log) => (
            <TableRow key={log.requestId}>
              <TableCell className="text-on-surface-variant">{log.timestamp}</TableCell>
              <TableCell className="font-mono text-code-sm">{log.requestId}</TableCell>
              <TableCell>{log.model}</TableCell>
              <TableCell>
                <Badge variant={log.status === 200 ? 'success' : 'error'}>
                  {log.status} {log.status === 200 ? 'OK' : 'Error'}
                </Badge>
              </TableCell>
              <TableCell className="text-right font-mono text-code-sm">{log.latencyMs}ms</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </Card>
  );
}
```

- [ ] **Step 6: Update Dashboard page**

```typescript
// apps/ui/src/app/page.tsx
'use client';

import { MetricCard } from '@/components/metrics/MetricCard';
import { TimeSeriesChart } from '@/components/metrics/TimeSeriesChart';
import { RecentLogsTable } from '@/components/logs/RecentLogsTable';
import { useMetrics, useLatency, useTokens } from '@/hooks/useMetrics';
import { fetchRecentLogs } from '@/lib/api';
import { useQuery } from '@tanstack/react-query';

export default function Dashboard() {
  const { data: metrics, isLoading: metricsLoading } = useMetrics();
  const { data: latency } = useLatency();
  const { data: tokens } = useTokens();
  const { data: logs } = useQuery({ queryKey: ['recent-logs'], queryFn: () => fetchRecentLogs(10) });

  if (metricsLoading) return <div>Loading...</div>;

  return (
    <>
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b hairline-border pb-4">
        <div>
          <h1 className="text-display-lg">Dashboard</h1>
          <p className="text-body-base mt-1 text-on-surface-variant">Real-time LLM proxy performance metrics.</p>
        </div>
        <div className="flex gap-3">
          <button className="bg-surface-container-lowest hairline-border px-4 py-2 rounded flex items-center gap-2">
            <span className="material-symbols-outlined">calendar_month</span>
            Last 24 Hours
          </button>
          <button className="bg-primary text-on-primary px-4 py-2 rounded flex items-center gap-2">
            <span className="material-symbols-outlined">download</span>
            Export
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <MetricCard
          title="Total Requests"
          value={metrics?.totalRequests || '0'}
          trend={{ value: '+12.5% from yesterday', direction: 'up' }}
          icon="swap_vert"
        />
        <MetricCard
          title="Avg Latency"
          value={`${latency?.p95 || 0}ms`}
          trend={{ value: '+42ms from yesterday', direction: 'up' }}
          trendColor="error"
          icon="timer"
        />
        <MetricCard
          title="Token Usage"
          value={`${tokens?.total || 0}M`}
          trend={{ value: '+5.2% from yesterday', direction: 'up' }}
          icon="toll"
        />
        <MetricCard
          title="Error Rate"
          value={`${metrics?.errorRate || 0}%`}
          trend={{ value: 'Stable', direction: 'flat' }}
          icon="error"
        />
      </div>

      <TimeSeriesChart
        data={metrics?.timeseries || []}
        title="Request Volume"
      />

      <RecentLogsTable logs={logs || []} />
    </>
  );
}
```

- [ ] **Step 7: Set up React Query provider**

```typescript
// apps/ui/src/app/providers.tsx
'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState } from 'react';

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}
```

- [ ] **Step 8: Wrap layout with Providers**

```typescript
// apps/ui/src/app/layout.tsx
import { Providers } from './providers';

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <Providers>
          <Shell>{children}</Shell>
        </Providers>
      </body>
    </html>
  );
}
```

- [ ] **Step 9: Commit**

```bash
git add apps/ui/src/app/page.tsx apps/ui/src/components/metrics/ apps/ui/src/components/logs/ apps/ui/src/lib/ apps/ui/src/hooks/
git commit -m "feat(ui): implement Dashboard Overview page

- MetricCard component (4 metrics: requests, latency, tokens, error rate)
- TimeSeriesChart with Recharts (request volume over 24h)
- RecentLogsTable (last 10 events)
- React Query hooks for data fetching
- Real-time updates (5s polling interval)
- Export and date range buttons (placeholder)
"
```

---

### Task 5: Providers Page (Read-Only)

**Files:**
- Create: `apps/ui/src/app/providers/page.tsx`
- Create: `apps/ui/src/components/providers/ProviderList.tsx`
- Create: `apps/ui/src/lib/providers.ts`

**Interfaces:**
- Consumes: API client pattern from Task 4
- Produces: Providers list with health status

*(Continue with similar detailed steps for remaining tasks...)*

---

## Remaining Tasks (Summarized)

### Task 5: Providers Page (Read-Only)
- Provider list table
- Health status indicators
- Basic provider details (expandable rows)

### Task 6: API Keys Page (CRUD)
- Keys list table
- Create key modal
- Edit/delete actions
- Copy key warning (show once)

### Task 7: Logs Page (Basic Viewer)
- Logs table with sorting
- Basic filters (date range, model)
- Log detail modal

### Task 8: WebSocket Integration
- WebSocket connection manager
- Real-time metrics subscription
- Fallback to polling if WS fails

### Task 9: Docker Deployment
- Dockerfile for UI
- Docker Compose with backend
- Environment variables

### Task 10: Documentation
- README for UI setup
- Component documentation
- API integration guide

---

## Self-Review

**1. Spec coverage:**
- ✅ Dashboard (Task 4)
- ✅ Providers list (Task 5)
- ✅ API Keys CRUD (Task 6)
- ✅ Logs viewer (Task 7)
- ✅ Layout shell (Task 3)
- ✅ Base components (Task 2)
- ✅ WebSocket real-time (Task 8)

**2. Placeholder scan:**
- ✅ No TBD/TODO in tasks 1-4
- ⚠️ Tasks 5-10 summarized (need full detail before execution)

**3. Type consistency:**
- ✅ All components use Godwit tokens
- ✅ API client pattern consistent
- ✅ React Query hooks follow same pattern

---

**Plan complete and saved to `docs/superpowers/plans/2026-08-07-godwit-ui-phase1-mvp.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
