'use client';
import { useState } from 'react';
import Link from 'next/link';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { LogoMark } from '@/components/icons';
import { useT } from '@/hooks/useT';
import { forgotPassword } from '@/lib/auth';

export default function ForgotPasswordPage() {
  const { t } = useT();
  const [email, setEmail] = useState('');
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    try {
      await forgotPassword(email);
      setSent(true);
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
          <h1 className="text-lg font-semibold tracking-[-0.02em]">{t('auth.forgot.title')}</h1>
        </div>
        <p className="text-[13px] text-muted mb-6">{t('auth.forgot.subtitle')}</p>
        {sent ? (
          <p className="text-[13px] text-fg mb-6">{t('auth.forgot.success')}</p>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-4">
            <Input label={t('auth.forgot.email')} type="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
            <Button type="submit" className="w-full" disabled={busy}>{busy ? t('auth.forgot.sending') : t('auth.forgot.submit')}</Button>
          </form>
        )}
        <div className="mt-5 text-center">
          <Link href="/login" className="text-[12.5px] text-accent hover:underline">{t('auth.backToLogin')}</Link>
        </div>
      </div>
    </div>
  );
}
