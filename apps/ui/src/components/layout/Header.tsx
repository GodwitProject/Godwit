// apps/ui/src/components/layout/Header.tsx
'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { Button } from '@/components/ui/Button';
import { logout } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';

export function Header() {
  const router = useRouter();
  const user = useAuthStore((s) => s.user);
  const setUser = useAuthStore((s) => s.setUser);

  async function handleSignOut() {
    await logout();
    setUser(null);
    router.push('/login');
  }

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
        {user && (
          <span className="text-label-sm text-on-surface-variant hidden sm:inline">
            {user.email}
          </span>
        )}
        {user && (
          <Button variant="ghost" size="sm" onClick={handleSignOut}>
            Sign out
          </Button>
        )}
      </div>
    </header>
  );
}
