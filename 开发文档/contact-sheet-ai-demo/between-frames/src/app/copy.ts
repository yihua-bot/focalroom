// 文案集中维护 + locale 感知代理：所有已用 zh.xxx 的组件自动支持四语言。
import zhStrings from '../i18n/locales/zh.json';
import enStrings from '../i18n/locales/en.json';
import jaStrings from '../i18n/locales/ja.json';
import koStrings from '../i18n/locales/ko.json';

const bundles: Record<string, Record<string, unknown>> = {
  zh: zhStrings, en: enStrings, ko: koStrings, ja: jaStrings,
};

let currentLocale = 'zh';
export function setCopyLocale(locale: string) { currentLocale = locale; }
export function getCopyLocale() { return currentLocale; }

function pick(obj: Record<string, unknown>, key: string): unknown {
  if (key in obj) return obj[key];
  // 支持点号嵌套（如 status.keep → copy.status.keep）。
  const parts = key.split('.');
  const nested = parts.reduce<unknown>((acc, k) =>
    acc && typeof acc === 'object' ? (acc as Record<string, unknown>)[k] : undefined, obj);
  if (nested !== undefined) return nested;
  // 查 copy.* 子对象。
  const copy = obj.copy;
  if (copy && typeof copy === 'object') {
    const c = copy as Record<string, unknown>;
    if (key in c) return c[key];
    return parts.reduce<unknown>((acc, k) =>
      acc && typeof acc === 'object' ? (acc as Record<string, unknown>)[k] : undefined, c);
  }
  return undefined;
}

// 代理：zh.xxx 读当前语言的对应键；缺键回落中文原文。
export const zh = new Proxy({} as Record<string, string>, {
  get(_, key: string) {
    const bundle = bundles[currentLocale] ?? bundles.zh;
    const value = pick(bundle, key);
    if (typeof value === 'string') return value;
    if (value !== undefined && typeof value !== 'object') return String(value);
    // 回落到中文源。
    const fallback = pick(bundles.zh, key);
    if (typeof fallback === 'string') return fallback;
    if (fallback !== undefined && typeof fallback !== 'object') return String(fallback);
    return key;
  },
});

export function t(key: string): string {
  return String(zh[key]);
}
