'use client';
import { RequireAuth } from '@/components/auth/RequireAuth';
export default function ProtectedLayout({ children }: { children: React.ReactNode }) {
  return <RequireAuth>{children}</RequireAuth>;
}
