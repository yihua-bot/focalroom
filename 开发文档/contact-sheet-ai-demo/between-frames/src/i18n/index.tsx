import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';

export type Locale = 'zh' | 'en' | 'ja' | 'ko';
export const LOCALES: { code: Locale; label: string; flag: string }[] = [
  { code: 'zh', label: '中文', flag: '🇨🇳' },
  { code: 'en', label: 'English', flag: '🇺🇸' },
  { code: 'ja', label: '日本語', flag: '🇯🇵' },
  { code: 'ko', label: '한국어', flag: '🇰🇷' },
];

// 动态加载语言包
const bundles: Record<Locale, () => Promise<Record<string, unknown>>> = {
  zh: () => import('./locales/zh.json').then(m => m.default),
  en: () => import('./locales/en.json').then(m => m.default),
  ja: () => import('./locales/ja.json').then(m => m.default),
  ko: () => import('./locales/ko.json').then(m => m.default),
};

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
  const [locale, setLocaleState] = useState<Locale>(() => {
    const saved = localStorage.getItem('focalroom-locale');
    if (saved === 'en' || saved === 'ja' || saved === 'ko' || saved === 'zh') return saved;
    const browser = navigator.language.toLowerCase();
    if (browser.startsWith('ja')) return 'ja';
    if (browser.startsWith('ko')) return 'ko';
    if (browser.startsWith('zh')) return 'zh';
    return 'en';
  });
  const [bundle, setBundle] = useState<Record<string, unknown>>({});

  // 加载语言包 + 同步 copy locale
  useState(() => {
    bundles[locale]().then(setBundle);
    import('../app/copy').then(({ setCopyLocale }) => setCopyLocale(locale));
  });

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    localStorage.setItem('focalroom-locale', l);
    bundles[l]().then(setBundle);
    document.documentElement.lang = l;
    // 同步 copy.ts 的 locale 代理。
    import('../app/copy').then(({ setCopyLocale }) => setCopyLocale(l));
  }, []);

  const t = useCallback((key: string, fallback?: string) => {
    return String(bundle[key] ?? fallback ?? key);
  }, [bundle]);

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
