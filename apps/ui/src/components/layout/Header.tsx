// apps/ui/src/components/layout/Header.tsx
'use client';

import { useEffect, useRef } from 'react';
import { useRouter, usePathname } from 'next/navigation';
import { useT } from '@/hooks/useT';
import { useAuthStore } from '@/store/auth';
import { logout } from '@/lib/auth';
import { LangSwitch } from '@/components/ui/LangSwitch';
import { SearchIcon, BellIcon, KeyboardIcon, PlusIcon } from '@/components/icons';

const CRUMBS: Record<string, 'nav.overview' | 'nav.traffic' | 'nav.logs' | 'nav.keys' | 'nav.models'> = {
  '/': 'nav.overview',
  '/logs': 'nav.traffic',
  '/keys': 'nav.keys',
  '/providers': 'nav.models',
};

export function Header({ onOpenShortcuts }: { onOpenShortcuts: () => void }) {
  const router = useRouter();
  const pathname = usePathname();
  const { t } = useT();
  const user = useAuthStore((s) => s.user);
  const setUser = useAuthStore((s) => s.setUser);

  const crumbKey = CRUMBS[pathname] ?? 'nav.overview';
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey) return;
      const tag = (document.activeElement?.tagName ?? '').toUpperCase();
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      e.preventDefault();
      searchRef.current?.focus();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, []);

  async function handleNew() {
    if (pathname === '/keys') return;
    router.push('/keys');
  }

  async function handleSignOut() {
    await logout();
    setUser(null);
    router.push('/login');
  }

  return (
    <header className="h-[52px] flex-none flex items-center gap-4 px-5 bg-surface border-b border-border">
      <div className="text-[12.5px] text-muted font-medium">
        <b className="text-fg font-medium">{t(crumbKey)}</b>
      </div>

      <div className="ml-auto hidden sm:flex items-center gap-2 bg-bg border border-border rounded-lg px-2.5 py-1.5 text-muted text-[12.5px] w-[300px] lg:w-[340px]">
        <SearchIcon width={14} height={14} />
        <input
          ref={searchRef}
          className="flex-1 bg-transparent font-mono text-xs outline-none placeholder:text-muted text-fg"
          placeholder={t('top.searchPlaceholder')}
          aria-label={t('top.searchPlaceholder')}
        />
        <kbd className="font-mono text-[10px] text-muted border border-border rounded px-1.5 py-0.5 bg-surface">/</kbd>
      </div>

      <div className="flex items-center gap-2">
        <LangSwitch />
        <button
          type="button"
          className="grid place-items-center w-8 h-8 rounded-lg text-muted border border-border hover:bg-bg hover:text-fg"
          title={t('top.notifications')}
        >
          <BellIcon width={16} height={16} />
        </button>
        <button
          type="button"
          className="grid place-items-center w-8 h-8 rounded-lg text-muted border border-border hover:bg-bg hover:text-fg"
          title={t('top.shortcuts')}
          onClick={onOpenShortcuts}
        >
          <KeyboardIcon width={16} height={16} />
        </button>
        <button
          type="button"
          className="flex items-center gap-1.5 bg-fg text-bg text-[12.5px] font-medium px-3 py-[7px] rounded-lg hover:bg-[oklch(16%_0.02_240)]"
          onClick={handleNew}
        >
          <PlusIcon width={14} height={14} />
          {pathname === '/keys' ? t('top.newKey') : t('top.newRequest')}
        </button>
      </div>

      {user && (
        <span className="text-[12px] text-muted hidden sm:inline">{user.email}</span>
      )}
      {user && (
        <button
          type="button"
          onClick={handleSignOut}
          className="text-[12px] text-muted hover:text-fg font-medium border-l border-border pl-3"
        >
          {t('auth.signOut')}
        </button>
      )}
    </header>
  );
}
