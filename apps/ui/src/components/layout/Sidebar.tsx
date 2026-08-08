// apps/ui/src/components/layout/Sidebar.tsx
'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { clsx } from 'clsx';
import { useT } from '@/hooks/useT';
import { OverviewIcon, TrafficIcon, KeysIcon, ProvidersIcon, SettingsIcon, LogoMark } from '@/components/icons';
import { useAuthStore } from '@/store/auth';

const explorerItems = [
  { href: '/', label: 'nav.overview' as const, icon: OverviewIcon },
  { href: '/logs', label: 'nav.traffic' as const, icon: TrafficIcon },
  { href: '/keys', label: 'nav.keys' as const, icon: KeysIcon },
  { href: '/providers', label: 'nav.models' as const, icon: ProvidersIcon },
];

function initials(name: string | null | undefined): string {
  if (!name) return '?';
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? '')
    .join('');
}

export function Sidebar() {
  const pathname = usePathname();
  const { t } = useT();
  const user = useAuthStore((s) => s.user);

  const isActive = (href: string) =>
    href === '/' ? pathname === '/' : pathname.startsWith(href);

  return (
    <aside className="hidden md:flex flex-col h-full w-sidebar-width fixed left-0 top-0 bg-sidebar-bg border-r border-border py-4 px-3 gap-1 z-40">
      <Link href="/" className="flex items-center gap-2.5 px-2 pb-3.5">
        <span className="grid place-items-center w-[26px] h-[26px] rounded-[7px] bg-accent text-on-accent">
          <LogoMark width={15} height={15} />
        </span>
        <span>
          <span className="block font-semibold text-[14px] tracking-[-0.01em] text-fg">Godwit</span>
          <span className="block text-[11px] text-muted font-mono">{t('brand.env')}</span>
        </span>
      </Link>

      <div className="text-[11px] uppercase tracking-[0.08em] text-muted px-2 pt-3.5 pb-1.5">
        {t('nav.explorer')}
      </div>

      <nav className="flex flex-col gap-1">
        {explorerItems.map((item) => {
          const Icon = item.icon;
          return (
            <Link
              key={item.href}
              href={item.href}
              className={clsx(
                'flex items-center gap-2.5 px-2 py-[7px] rounded-lg text-[13.5px] transition-colors',
                isActive(item.href)
                  ? 'bg-surface text-fg shadow-ambient font-medium'
                  : 'text-fg hover:bg-surface-2'
              )}
            >
              <Icon className={clsx('flex-none', isActive(item.href) ? 'text-accent-strong' : 'text-muted')} />
              <span>{t(item.label)}</span>
            </Link>
          );
        })}
      </nav>

      <div className="text-[11px] uppercase tracking-[0.08em] text-muted px-2 pt-3.5 pb-1.5">
        {t('nav.system')}
      </div>
      <button
        type="button"
        className="flex items-center gap-2.5 px-2 py-[7px] rounded-lg text-[13.5px] text-fg hover:bg-surface-2 transition-colors cursor-not-allowed opacity-70"
        disabled
        title="Coming soon"
      >
        <SettingsIcon className="flex-none text-muted" />
        <span>{t('nav.settings')}</span>
      </button>

      <div className="flex-1" />

      <div className="flex items-center gap-2.5 px-2 border-t border-border pt-2 mt-2">
        <span className="grid place-items-center w-[26px] h-[26px] rounded-full bg-accent text-on-accent text-[11px] font-semibold">
          {initials(user?.email?.split('@')[0] ?? user?.email)}
        </span>
        <span>
          <span className="block text-[12.5px] font-medium text-fg leading-tight">
            {user?.email?.split('@')[0] ?? 'Guest'}
          </span>
          <span className="block text-[11px] text-muted">{t('user.role')}</span>
        </span>
      </div>
    </aside>
  );
}
