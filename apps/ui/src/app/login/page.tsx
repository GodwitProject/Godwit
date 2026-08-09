'use client';
import { useState } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { LangSwitch } from '@/components/ui/LangSwitch';
import { LogoMark } from '@/components/icons';
import { useT } from '@/hooks/useT';
import { login } from '@/lib/auth';
import { useAuthStore } from '@/store/auth';

export default function LoginPage() {
  const router = useRouter();
  const { t } = useT();
  const setUser = useAuthStore((s) => s.setUser);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true); setError(null);
    try {
      const { user, must_change_password } = await login(email, password);
      setUser(user);
      router.push(must_change_password ? '/change-required' : '/');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally { setBusy(false); }
  }

  return (
    <div className="grid place-items-center bg-bg px-4" style={{ minHeight: 'calc(100vh - 52px)' }}>
      <div className="w-full max-w-sm bg-surface border border-border rounded-2xl p-6 shadow-ambient">
        <div className="flex items-center gap-2.5 mb-1">
          <span className="grid place-items-center w-[26px] h-[26px] rounded-[7px] bg-accent text-on-accent">
            <LogoMark width={15} height={15} />
          </span>
          <h1 className="text-lg font-semibold tracking-[-0.02em]">{t('login.title')}</h1>
        </div>
        <p className="text-[13px] text-muted mb-6">{t('login.subtitle')}</p>
        <div className="flex justify-end mb-3">
          <LangSwitch />
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <Input label={t('login.email')} type="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
          <Input label={t('login.password')} type="password" required value={password} onChange={(e) => setPassword(e.target.value)} />
          {error && <p className="text-[12px] text-danger">{error}</p>}
          <Button type="submit" className="w-full" disabled={busy}>{busy ? t('login.signingIn') : t('login.submit')}</Button>
        </form>
        <div className="mt-5 text-center">
          <Link href="/forgot-password" className="text-[12.5px] text-accent hover:underline">{t('login.forgot')}</Link>
        </div>
      </div>
    </div>
  );
}
