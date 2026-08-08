'use client';

import { useT } from '@/hooks/useT';
import { clsx } from 'clsx';

export function LangSwitch({ compact = false }: { compact?: boolean }) {
  const { lang, setLang } = useT();

  return (
    <div className="seg" role="group" aria-label="Language">
      {(['fr', 'en'] as const).map((l) => (
        <button
          key={l}
          type="button"
          className={clsx(
            'uppercase',
            lang === l ? 'on' : ''
          )}
          onClick={() => setLang(l)}
          aria-pressed={lang === l}
          aria-label={l === 'fr' ? 'Français' : 'English'}
        >
          {compact ? l : l === 'fr' ? 'FR' : 'EN'}
        </button>
      ))}
    </div>
  );
}
