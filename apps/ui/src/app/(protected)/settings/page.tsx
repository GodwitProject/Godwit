'use client';

import { useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useT } from '@/hooks/useT';
import { useAuthStore } from '@/store/auth';
import { useI18nStore } from '@/store/i18n';
import { changePassword } from '@/lib/auth';

export default function SettingsPage() {
  const { t } = useT();
  const user = useAuthStore((s) => s.user);
  const lang = useI18nStore((s) => s.lang);
  const setLang = useI18nStore((s) => s.setLang);
  const [current, setCurrent] = useState('');
  const [next, setNext] = useState('');
  const [confirm, setConfirm] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (next !== confirm) {
      setError(t('auth.mismatch'));
      return;
    }
    setError(null);
    setSuccess(false);
    setBusy(true);
    try {
      await changePassword(current, next);
      setSuccess(true);
      setCurrent(''); setNext(''); setConfirm('');
    } catch (err) {
      setError(err instanceof Error ? err.message : t('auth.change.submit'));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="view-fade space-y-4">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-4 border-b border-border pb-4">
        <div>
          <h1 className="text-display-lg">{t('nav.settings')}</h1>
          <p className="text-[13px] text-muted mt-1 max-w-[62ch]">{t('settings.subtitle')}</p>
        </div>
      </div>

      <Card className="overflow-hidden">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-[13px] font-semibold">{t('settings.locale')}</h2>
        </div>
        <div className="px-4 py-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <div className="text-[13px] font-medium">{t('settings.localeLanguage')}</div>
              <div className="text-[12px] text-muted">{t('settings.localeHint')}</div>
            </div>
            <select
              className="bg-surface border border-border rounded-lg px-3 py-2 text-[12.5px] text-fg font-mono focus:outline-none focus:border-accent"
              value={lang}
              onChange={(e) => setLang(e.target.value as 'fr' | 'en')}
              aria-label={t('settings.localeLanguage')}
            >
              <option value="en">English</option>
              <option value="fr">Français</option>
            </select>
          </div>
        </div>
      </Card>

      <Card className="overflow-hidden">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-[13px] font-semibold">{t('settings.session')}</h2>
        </div>
        <div className="divide-y divide-bg">
          <div className="flex items-center justify-between gap-4 px-4 py-3">
            <span className="text-[13px] text-fg">{t('settings.sessionEmail')}</span>
            <span className="font-mono text-[12.5px] text-muted">{user?.email ?? '—'}</span>
          </div>
          <div className="flex items-center justify-between gap-4 px-4 py-3">
            <span className="text-[13px] text-fg">{t('settings.sessionRole')}</span>
            <span className="font-mono text-[12.5px] text-muted">{user?.role ?? '—'}</span>
          </div>
        </div>
      </Card>

      <Card className="overflow-hidden">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-[13px] font-semibold">{t('auth.change.submit')}</h2>
        </div>
        <form onSubmit={handleSubmit} className="px-4 py-4 space-y-4">
          <Input label={t('auth.change.currentPassword')} type="password" required value={current} onChange={(e) => setCurrent(e.target.value)} />
          <Input label={t('auth.change.newPassword')} type="password" required value={next} onChange={(e) => setNext(e.target.value)} />
          <Input label={t('auth.change.confirm')} type="password" required value={confirm} onChange={(e) => setConfirm(e.target.value)} />
          {error && <p className="text-[12px] text-danger">{error}</p>}
          {success && <p className="text-[12px] text-muted">{t('auth.change.success')}</p>}
          <Button type="submit" disabled={busy}>{busy ? t('auth.change.submitting') : t('auth.change.submit')}</Button>
        </form>
      </Card>
    </div>
  );
}
