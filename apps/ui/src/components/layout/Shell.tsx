'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { MobileNav } from './MobileNav';
import { ShortcutsModal } from '@/components/ui/ShortcutsModal';
import { useI18nStore, detectStoredLang } from '@/store/i18n';

const NAV: Record<string, string> = {
  d: '/',
  t: '/logs',
  k: '/keys',
  r: '/providers',
  p: '/providers',
};

export function Shell({ children }: { children: React.ReactNode }) {
  const [showShortcuts, setShowShortcuts] = useState(false);
  const setLang = useI18nStore((s) => s.setLang);
  const router = useRouter();

  useEffect(() => {
    setLang(detectStoredLang());
  }, [setLang]);

  useEffect(() => {
    let buf = '';
    let timer: ReturnType<typeof setTimeout> | null = null;
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const tag = (document.activeElement?.tagName ?? '').toUpperCase();
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if (e.key === '?') { e.preventDefault(); setShowShortcuts((v) => !v); return; }
      if (e.key.toLowerCase() === 'g') {
        buf = 'g';
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => { buf = ''; }, 700);
        return;
      }
      if (buf === 'g') {
        const dest = NAV[e.key.toLowerCase()];
        if (dest) router.push(dest);
        buf = '';
        if (timer) clearTimeout(timer);
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [router]);

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <Header onOpenShortcuts={() => setShowShortcuts(true)} />
        <main className="flex-1 overflow-y-auto p-5 pb-24 md:pb-16">{children}</main>
      </div>
      <MobileNav />
      <ShortcutsModal open={showShortcuts} onClose={() => setShowShortcuts(false)} />
    </div>
  );
}
