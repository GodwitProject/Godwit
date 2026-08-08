import { create } from 'zustand';
import { translations, type Lang, type TranslationKey } from '@/i18n/translations';

const STORAGE_KEY = 'godwit.lang';

export function detectStoredLang(): Lang {
  if (typeof window === 'undefined') return 'en';
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === 'fr' || stored === 'en') return stored;
    const nav = window.navigator.language?.toLowerCase() ?? '';
    return nav.startsWith('fr') ? 'fr' : 'en';
  } catch {
    return 'en';
  }
}

interface I18nStore {
  lang: Lang;
  setLang: (lang: Lang) => void;
}

export const useI18nStore = create<I18nStore>((set) => ({
  lang: 'en',
  setLang: (lang) => {
    if (typeof window !== 'undefined') {
      try { window.localStorage.setItem(STORAGE_KEY, lang); } catch { /* ignore */ }
    }
    set({ lang });
  },
}));

export type { Lang, TranslationKey };

export function translate(lang: Lang, key: TranslationKey): string {
  return translations[lang][key] ?? translations.en[key] ?? key;
}
