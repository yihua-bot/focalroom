import { detectLocale, resolveCopy, setCopyLocale } from '../app/copy';
import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';

export type Locale = 'zh' | 'en' | 'ja' | 'ko';
export const LOCALES: { code: Locale; label: string; flag: string }[] = [
  { code: 'zh', label: '中文', flag: '🇨🇳' },
  { code: 'en', label: 'English', flag: '🇺🇸' },
  { code: 'ja', label: '日本語', flag: '🇯🇵' },
  { code: 'ko', label: '한국어', flag: '🇰🇷' },
];

interface I18nContextValue {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: (key: string, fallback?: string) => string;
}

const I18nContext = createContext<I18nContextValue>({
  locale: 'zh',
  setLocale: () => {},
  t: (key, fallback) => fallback ?? key,
});

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => detectLocale() as Locale);

  useEffect(() => {
    document.documentElement.lang = locale;
    if (isTauri()) {
      void invoke('set_native_menu_locale', { locale }).catch(error => {
        console.warn('Unable to update the native menu language', error);
      });
    }
  }, [locale]);

  const setLocale = useCallback((l: Locale) => {
    // 语言包已随主包静态打进来，切换是同步的：不会出现半中半英的中间态，
    // 也没有"分片加载失败就一直停在旧语言"这种静默失败。
    setCopyLocale(l);
    setLocaleState(l);
    try {
      localStorage.setItem('focalroom-locale', l);
    } catch {
      // 存储不可用只影响"记住选择"，界面当次仍然切换。
    }
  }, []);

  const t = useCallback((key: string, fallback?: string) => {
    return resolveCopy(locale, key) ?? fallback ?? key;
  }, [locale]);

  return <I18nContext.Provider value={{ locale, setLocale, t }}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}

export function LanguageSwitcher() {
  const { locale, setLocale } = useI18n();
  return <select
    aria-label="Language / 语言 / 言語 / 언어"
    className="language-switcher"
    value={locale}
    onChange={e => setLocale(e.target.value as Locale)}
    style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid var(--border)', borderRadius: 6, padding: '4px 8px', fontSize: '0.85rem', cursor: 'pointer' }}
  >
    {LOCALES.map(l => <option key={l.code} value={l.code}>{l.flag} {l.label}</option>)}
  </select>;
}
