'use client';
import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { fetchMe } from '@/lib/auth';

export function RequireAuth({ children }: { children: React.ReactNode }) {
  const router = useRouter();
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
    return <div className="min-h-screen flex items-center justify-center text-on-surface-variant">Loading…</div>;
  }
  return <>{children}</>;
}
