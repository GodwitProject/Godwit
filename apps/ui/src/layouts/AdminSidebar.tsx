import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  Box,
  Server,
  Users,
  Key,
  BarChart3,
  Settings,
} from 'lucide-react';
import { clsx } from '@/lib/utils';

const navItems = [
  { to: '/admin', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/admin/models', label: 'Models', icon: Box },
  { to: '/admin/provider-profiles', label: 'Providers', icon: Server },
  { to: '/admin/users', label: 'Users', icon: Users },
  { to: '/admin/keys', label: 'API Keys', icon: Key },
  { to: '/admin/usage', label: 'Usage', icon: BarChart3 },
  { to: '/admin/settings', label: 'Settings', icon: Settings },
];

export function AdminSidebar() {
  return (
    <aside className="hidden md:flex w-sidebar-width flex-col border-r hairline-border bg-surface-container-lowest h-[calc(100vh-4rem)]">
      <nav className="flex-1 p-4 space-y-1">
        {navItems.map((item) => (
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
