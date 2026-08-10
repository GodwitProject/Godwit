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
