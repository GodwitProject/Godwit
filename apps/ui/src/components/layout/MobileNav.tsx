// apps/ui/src/components/layout/MobileNav.tsx
'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { clsx } from 'clsx';
import { useT } from '@/hooks/useT';
import { OverviewIcon, TrafficIcon, KeysIcon, ProvidersIcon } from '@/components/icons';

const items = [
  { href: '/', label: 'nav.overview' as const, icon: OverviewIcon },
  { href: '/logs', label: 'nav.traffic' as const, icon: TrafficIcon },
  { href: '/keys', label: 'nav.keys' as const, icon: KeysIcon },
  { href: '/providers', label: 'nav.models' as const, icon: ProvidersIcon },
];

export function MobileNav() {
  const pathname = usePathname();
  const { t } = useT();

  const isActive = (href: string) =>
    href === '/' ? pathname === '/' : pathname.startsWith(href);

  return (
    <nav className="md:hidden fixed bottom-0 left-0 right-0 z-30 bg-surface border-t border-border h-[60px] flex items-center px-1.5 gap-0.5">
      {items.map((item) => {
        const Icon = item.icon;
        return (
          <Link
            key={item.href}
            href={item.href}
            className={clsx(
              'flex-1 flex flex-col items-center justify-center gap-0.5 h-[56px] rounded-lg text-[11px]',
              isActive(item.href) ? 'text-accent-strong font-medium' : 'text-muted'
            )}
          >
            <Icon width={18} height={18} />
            <span>{t(item.label)}</span>
          </Link>
        );
      })}
    </nav>
  );
}
