// 文案集中维护 + locale 感知代理：所有已用 zh.xxx 的组件自动支持四语言。
import zhStrings from '../i18n/locales/zh.json';
import enStrings from '../i18n/locales/en.json';
import jaStrings from '../i18n/locales/ja.json';
import koStrings from '../i18n/locales/ko.json';

const SUPPORTED = ['zh', 'en', 'ja', 'ko'];
export const STORAGE_KEY = 'focalroom-locale';

/// 语言选择：本地保存优先，其次跟随系统语言，兜底英文。
/// 文案代理与 Provider 必须用同一份判定，否则首屏会先渲染中文再翻成目标语言。
export function detectLocale(): string {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (typeof saved === 'string' && SUPPORTED.includes(saved)) return saved;
  } catch {
    // 非浏览器环境（构建期、纯 Node 夹具）没有存储，直接走系统语言。
  }
  const browser = (typeof navigator === 'undefined' ? 'en' : navigator.language).toLowerCase();
  if (browser.startsWith('ja')) return 'ja';
  if (browser.startsWith('ko')) return 'ko';
  if (browser.startsWith('zh')) return 'zh';
  return 'en';
}

export const bundles: Record<string, Record<string, unknown>> = {
  zh: zhStrings, en: enStrings, ko: koStrings, ja: jaStrings,
};

let currentLocale = detectLocale();
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

/// 两条文案通道共用的唯一查找规则：目标语言缺键时回落中文，而不是回落键名。
/// 回落成中文是"这屏还有中文"的可观测信号，回落成键名会直接变成乱码。
export function resolveCopy(locale: string, key: string): string | undefined {
  for (const source of [bundles[locale] ?? bundles.zh, bundles.zh]) {
    const value = pick(source, key);
    if (typeof value === 'string') return value;
    if (value !== undefined && typeof value !== 'object') return String(value);
  }
  return undefined;
}

// 代理：zh.xxx 读当前语言的对应键；缺键回落中文原文。
export const zh = new Proxy({} as Record<string, string>, {
  get(_, key: string) {
    return resolveCopy(currentLocale, key) ?? key;
  },
});

export function t(key: string): string {
  return String(zh[key]);
}

/** 色彩管理状态码 → 用户可读说明。界面与 IPC 层共用这一份，陌生的码原样露出，
 * 不替它翻译成一句好看的说法——猜出来的解释比难看的原码更误导。 */
export function colorStatusLabel(status: string): string {
  const key = `colorStatus_${status}`;
  return zh[key] === key ? status : zh[key];
}
