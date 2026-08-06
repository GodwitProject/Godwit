import { create } from 'zustand';
import type { AuthUser } from '@/lib/auth';

export type AuthStatus = 'unknown' | 'authenticated' | 'unauthenticated';

interface AuthStore {
  user: AuthUser | null;
  status: AuthStatus;
  setUser: (user: AuthUser | null) => void;
}

export const useAuthStore = create<AuthStore>((set) => ({
  user: null,
  status: 'unknown',
  setUser: (user) => set({ user, status: user ? 'authenticated' : 'unauthenticated' }),
}));
