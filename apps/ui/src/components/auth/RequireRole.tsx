import { Navigate, useLocation } from 'react-router-dom';
import { useAuthStore } from '@/store/auth';
import type { UserRole } from '@/types';

interface RequireRoleProps {
  allowed: UserRole[];
  fallback?: string;
  children: React.ReactNode;
}

export function RequireRole({ allowed, fallback = '/login', children }: RequireRoleProps) {
  const user = useAuthStore((state) => state.user);
  const status = useAuthStore((state) => state.status);
  const location = useLocation();

  if (status === 'unknown') {
    return (
      <div className="flex h-screen items-center justify-center text-on-surface-variant">
        Loading…
      </div>
    );
  }

  if (!user || !allowed.includes(user.role)) {
    return <Navigate to={fallback} state={{ from: location }} replace />;
  }

  return <>{children}</>;
}
