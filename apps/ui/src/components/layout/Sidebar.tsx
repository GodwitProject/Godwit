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
