'use client';
import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { useT } from '@/hooks/useT';
import { fetchMe } from '@/lib/auth';

export function RequireAuth({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const { t } = useT();
  const status = useAuthStore((s) => s.status);
  const setUser = useAuthStore((s) => s.setUser);

  useEffect(() => {
    if (status === 'unknown') {
      fetchMe().then(setUser).catch(() => { setUser(null); router.replace('/login'); });
    } else if (status === 'unauthenticated') {
      router.replace('/login');
    }
  }, [status, setUser, router]);

  if (status !== 'authenticated') {
    return (
      <div className="flex items-center justify-center h-full text-muted">
        <span className="animate-spin mr-3">◌</span>
        {t('login.loading')}
      </div>
    );
  }
  return <>{children}</>;
}
