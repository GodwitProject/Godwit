import { useEffect } from 'react';
import { useAuthStore } from '@/store/auth';
import { fetchMe, logout } from '@/lib/auth';

export function useAuthInit() {
  const setUser = useAuthStore((state) => state.setUser);

  useEffect(() => {
    fetchMe()
      .then(setUser)
      .catch(() => setUser(null));
  }, [setUser]);
}

export function useLogout() {
  const setUser = useAuthStore((state) => state.setUser);

  return async () => {
    await logout();
    setUser(null);
  };
}
