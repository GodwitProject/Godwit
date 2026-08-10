import { create } from 'zustand';
import type { AuthUser } from '@/types';

export type AuthStatus = 'unknown' | 'authenticated' | 'unauthenticated';

interface AuthState {
  user: AuthUser | null;
  status: AuthStatus;
  setUser: (user: AuthUser | null) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  status: 'unknown',
  setUser: (user) =>
    set({
      user,
      status: user ? 'authenticated' : 'unauthenticated',
    }),
}));
