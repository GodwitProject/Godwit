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
