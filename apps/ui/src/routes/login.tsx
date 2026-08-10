import { useNavigate, useLocation } from 'react-router-dom';
import { login } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';
import { LoginForm } from '@/components/auth/LoginForm';
import { Card } from '@/components/ui/Card';

export function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const setUser = useAuthStore((state) => state.setUser);

  const from = (location.state as { from?: { pathname?: string } } | null)?.from?.pathname || '/';

  const handleLogin = async (email: string, password: string) => {
    const user = await login(email, password);
    setUser(user);
    const destination = user.role === 'super_admin' ? '/admin' : '/console';
    navigate(from !== '/' ? from : destination, { replace: true });
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-surface-container-low px-4">
      <Card className="w-full max-w-sm">
        <div className="mb-6 text-center">
          <h1 className="text-headline-md text-on-surface">Sign in to Godwit</h1>
          <p className="text-body-base text-on-surface-variant mt-1">Admin & user console</p>
        </div>
        <LoginForm onSubmit={handleLogin} />
      </Card>
    </div>
  );
}
