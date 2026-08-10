import { useAuthStore } from '@/store/auth';
import { useLogout } from '@/hooks/useAuth';
import { Button } from '@/components/ui/Button';

export function TopBar() {
  const user = useAuthStore((state) => state.user);
  const logout = useLogout();

  return (
    <header className="sticky top-0 z-40 flex h-16 items-center justify-between border-b hairline-border bg-surface-container-lowest px-6">
      <div className="flex items-center gap-2">
        <span className="text-headline-md font-bold text-primary">Godwit</span>
      </div>
      <div className="flex items-center gap-4">
        {user && (
          <>
            <div className="text-right hidden sm:block">
              <p className="text-label-sm font-medium text-on-surface">{user.email}</p>
              <p className="text-caption-xs text-on-surface-variant capitalize">{user.role.replace('_', ' ')}</p>
            </div>
            <Button variant="ghost" size="sm" onClick={logout}>
              Sign out
            </Button>
          </>
        )}
      </div>
    </header>
  );
}
