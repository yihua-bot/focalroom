// 后端错误码翻译 + 日期/数字 locale 格式化。
import { useI18n } from './index';

const LOCALE_TAGS: Record<string, string> = {
  zh: 'zh-CN', en: 'en-US', ja: 'ja-JP', ko: 'ko-KR',
};

/** 翻译后端错误：优先用错误码查语言包，回落到原始中文消息。 */
export function translateError(code: string | undefined, fallbackMessage: string | undefined, t: (key: string, fb?: string) => string): string {
  if (!code) return fallbackMessage ?? 'Unknown error';
  return t(`error.${code}`, fallbackMessage ?? code);
}

/** locale 感知的日期格式化。 */
export function formatDate(isoString: string | undefined, locale: string): string {
  if (!isoString) return '';
  const tag = LOCALE_TAGS[locale] ?? 'en-US';
  try {
    return new Intl.DateTimeFormat(tag, { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(isoString));
  } catch {
    return isoString;
  }
}

/** locale 感知的日期格式化（仅日期，无时间）。 */
export function formatDateOnly(isoString: string | undefined, locale: string): string {
  if (!isoString) return '';
  const tag = LOCALE_TAGS[locale] ?? 'en-US';
  try {
    return new Intl.DateTimeFormat(tag, { year: 'numeric', month: 'short', day: 'numeric' }).format(new Date(isoString));
  } catch {
    return isoString;
  }
}

/** locale 感知的数字格式化（千分位）。 */
export function formatNumber(n: number, locale: string): string {
  const tag = LOCALE_TAGS[locale] ?? 'en-US';
  return new Intl.NumberFormat(tag).format(n);
}

/** locale 感知的文件大小格式化。 */
export function formatFileSize(bytes: number, locale: string): string {
  const tag = LOCALE_TAGS[locale] ?? 'en-US';
  const units = locale === 'zh' ? ['B', 'KB', 'MB', 'GB'] : locale === 'ja' ? ['B', 'KB', 'MB', 'GB'] : ['B', 'KB', 'MB', 'GB'];
  let size = bytes;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++; }
  return `${new Intl.NumberFormat(tag, { maximumFractionDigits: 1 }).format(size)} ${units[unit]}`;
}

/** locale 感知的货币格式化。 */
export function formatCost(usdCents: number, locale: string): string {
  const tag = LOCALE_TAGS[locale] ?? 'en-US';
  return new Intl.NumberFormat(tag, { style: 'currency', currency: 'USD', minimumFractionDigits: 2 }).format(usdCents / 100);
}

// ===== React Hook 版本 =====

export function useFormatter() {
  const { locale, t } = useI18n();
  return {
    error: (code?: string, message?: string) => translateError(code, message, t),
    date: (iso?: string) => formatDate(iso, locale),
    dateOnly: (iso?: string) => formatDateOnly(iso, locale),
    number: (n: number) => formatNumber(n, locale),
    fileSize: (bytes: number) => formatFileSize(bytes, locale),
    cost: (cents: number) => formatCost(cents, locale),
  };
}
