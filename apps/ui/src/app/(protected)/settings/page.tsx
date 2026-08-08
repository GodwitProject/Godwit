'use client';

import { Card } from '@/components/ui/Card';
import { useT } from '@/hooks/useT';
import { useAuthStore } from '@/store/auth';
import { useI18nStore } from '@/store/i18n';

export default function SettingsPage() {
  const { t } = useT();
  const user = useAuthStore((s) => s.user);
  const lang = useI18nStore((s) => s.lang);
  const setLang = useI18nStore((s) => s.setLang);

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
    </div>
  );
}
