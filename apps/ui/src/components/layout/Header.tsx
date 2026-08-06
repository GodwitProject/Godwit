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
