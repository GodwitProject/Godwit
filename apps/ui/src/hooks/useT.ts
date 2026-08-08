'use client';

import { useI18nStore, translate, type TranslationKey } from '@/store/i18n';

export function useT() {
  const lang = useI18nStore((s) => s.lang);
  const setLang = useI18nStore((s) => s.setLang);
  return {
    lang,
    setLang,
    t: (key: TranslationKey) => translate(lang, key),
  };
}
