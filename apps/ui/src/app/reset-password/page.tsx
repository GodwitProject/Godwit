'use client';
import { Suspense, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { LogoMark } from '@/components/icons';
import { useT } from '@/hooks/useT';
import { resetPassword } from '@/lib/auth';

function ResetPasswordForm() {
  const router = useRouter();
  const { t } = useT();
  const token = useSearchParams().get('token');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (password !== confirm) {
      setError(t('auth.mismatch'));
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await resetPassword(token ?? '', password);
      router.push('/login');
    } catch (err) {
      setError(err instanceof Error ? err.message : t('auth.reset.submit'));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid place-items-center bg-bg px-4" style={{ minHeight: 'calc(100vh - 52px)' }}>
      <div className="w-full max-w-sm bg-surface border border-border rounded-2xl p-6 shadow-ambient">
        <div className="flex items-center gap-2.5 mb-1">
          <span className="grid place-items-center w-[26px] h-[26px] rounded-[7px] bg-accent text-on-accent">
            <LogoMark width={15} height={15} />
          </span>
          <h1 className="text-lg font-semibold tracking-[-0.02em]">{t('auth.reset.title')}</h1>
        </div>
        <p className="text-[13px] text-muted mb-6">{t('auth.reset.subtitle')}</p>
        <form onSubmit={handleSubmit} className="space-y-4">
          <Input label={t('auth.reset.newPassword')} type="password" required value={password} onChange={(e) => setPassword(e.target.value)} />
          <Input label={t('auth.reset.confirm')} type="password" required value={confirm} onChange={(e) => setConfirm(e.target.value)} />
          {error && <p className="text-[12px] text-danger">{error}</p>}
          <Button type="submit" className="w-full" disabled={busy || !token}>{busy ? t('auth.reset.submitting') : t('auth.reset.submit')}</Button>
        </form>
      </div>
    </div>
  );
}

export default function ResetPasswordPage() {
  return (
    <Suspense fallback={null}>
      <ResetPasswordForm />
    </Suspense>
  );
}
