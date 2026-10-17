// Web 审稿界面 i18n + locale 格式化（与桌面共用语言包）。
import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';

export type Locale = 'zh' | 'en' | 'ja' | 'ko';

export const LOCALES: { code: Locale; label: string; flag: string }[] = [
  { code: 'zh', label: '中文', flag: '🇨🇳' },
  { code: 'en', label: 'English', flag: '🇺🇸' },
  { code: 'ja', label: '日本語', flag: '🇯🇵' },
  { code: 'ko', label: '한국어', flag: '🇰🇷' },
];

import zh from './locales/zh.json';
import en from './locales/en.json';
import ja from './locales/ja.json';
import ko from './locales/ko.json';

const bundles: Record<Locale, Record<string, unknown>> = { zh, en, ja, ko };
const LOCALE_TAGS: Record<Locale, string> = { zh: 'zh-CN', en: 'en-US', ja: 'ja-JP', ko: 'ko-KR' };

// Web 专属文案
const webStrings: Record<Locale, Record<string, string>> = {
  zh: {
    'web.login': '登录', 'web.register': '注册', 'web.logout': '退出登录',
    'web.email': '邮箱', 'web.password': '密码', 'web.displayName': '显示名称',
    'web.projects': '项目', 'web.members': '成员', 'web.invite': '邀请',
    'web.review': '审稿', 'web.comments': '评论', 'web.proposals': '提案',
    'web.approve': '批准', 'web.reject': '驳回', 'web.sequence': '序列',
    'web.versions': '版本历史', 'web.noProject': '还没有共享项目',
    'web.noProjectHint': '等待项目所有者邀请你加入',
    'web.commentPlaceholder': '写下你的评论…', 'web.submit': '提交',
    'web.resolve': '标记已解决', 'web.resolved': '已解决',
    'web.reviewer': '审稿者', 'web.owner': '所有者', 'web.editor': '编辑',
    'web.viewer': '查看者', 'web.online': '在线', 'web.offline': '离线',
    'web.connecting': '连接中…', 'web.reconnected': '已重新连接',
    'web.forgotPassword': '忘记密码？', 'web.resetPassword': '重置密码',
    'web.backToLogin': '返回登录',
  },
  en: {
    'web.login': 'Log In', 'web.register': 'Sign Up', 'web.logout': 'Log Out',
    'web.email': 'Email', 'web.password': 'Password', 'web.displayName': 'Display Name',
    'web.projects': 'Projects', 'web.members': 'Members', 'web.invite': 'Invite',
    'web.review': 'Review', 'web.comments': 'Comments', 'web.proposals': 'Proposals',
    'web.approve': 'Approve', 'web.reject': 'Reject', 'web.sequence': 'Sequence',
    'web.versions': 'Version History', 'web.noProject': 'No shared projects yet',
    'web.noProjectHint': 'Waiting for a project owner to invite you',
    'web.commentPlaceholder': 'Write your comment…', 'web.submit': 'Submit',
    'web.resolve': 'Mark Resolved', 'web.resolved': 'Resolved',
    'web.reviewer': 'Reviewer', 'web.owner': 'Owner', 'web.editor': 'Editor',
    'web.viewer': 'Viewer', 'web.online': 'Online', 'web.offline': 'Offline',
    'web.connecting': 'Connecting…', 'web.reconnected': 'Reconnected',
    'web.forgotPassword': 'Forgot Password?', 'web.resetPassword': 'Reset Password',
    'web.backToLogin': 'Back to Login',
  },
  ja: {
    'web.login': 'ログイン', 'web.register': '新規登録', 'web.logout': 'ログアウト',
    'web.email': 'メールアドレス', 'web.password': 'パスワード', 'web.displayName': '表示名',
    'web.projects': 'プロジェクト', 'web.members': 'メンバー', 'web.invite': '招待',
    'web.review': 'レビュー', 'web.comments': 'コメント', 'web.proposals': '提案',
    'web.approve': '承認', 'web.reject': '却下', 'web.sequence': 'シーケンス',
    'web.versions': 'バージョン履歴', 'web.noProject': '共有プロジェクトはまだありません',
    'web.noProjectHint': 'プロジェクトオーナーの招待を待っています',
    'web.commentPlaceholder': 'コメントを書いてください…', 'web.submit': '送信',
    'web.resolve': '解決済みにする', 'web.resolved': '解決済み',
    'web.reviewer': 'レビュアー', 'web.owner': 'オーナー', 'web.editor': 'エディター',
    'web.viewer': '閲覧者', 'web.online': 'オンライン', 'web.offline': 'オフライン',
    'web.connecting': '接続中…', 'web.reconnected': '再接続されました',
    'web.forgotPassword': 'パスワードを忘れた？', 'web.resetPassword': 'パスワードリセット',
    'web.backToLogin': 'ログインに戻る',
  },
  ko: {
    'web.login': '로그인', 'web.register': '회원가입', 'web.logout': '로그아웃',
    'web.email': '이메일', 'web.password': '비밀번호', 'web.displayName': '표시 이름',
    'web.projects': '프로젝트', 'web.members': '멤버', 'web.invite': '초대',
    'web.review': '리뷰', 'web.comments': '댓글', 'web.proposals': '제안',
    'web.approve': '승인', 'web.reject': '거부', 'web.sequence': '시퀀스',
    'web.versions': '버전 기록', 'web.noProject': '공유 프로젝트가 아직 없습니다',
    'web.noProjectHint': '프로젝트 소유자의 초대를 기다리고 있습니다',
    'web.commentPlaceholder': '댓글을 작성해 주세요…', 'web.submit': '보내기',
    'web.resolve': '해결됨으로 표시', 'web.resolved': '해결됨',
    'web.reviewer': '리뷰어', 'web.owner': '소유자', 'web.editor': '편집자',
    'web.viewer': '뷰어', 'web.online': '온라인', 'web.offline': '오프라인',
    'web.connecting': '연결 중…', 'web.reconnected': '다시 연결되었습니다',
    'web.forgotPassword': '비밀번호를 잊으셨나요?', 'web.resetPassword': '비밀번호 재설정',
    'web.backToLogin': '로그인으로 돌아가기',
  },
};

function lookup(locale: Locale, key: string): string {
  const web = webStrings[locale]?.[key];
  if (web) return web;
  const bundle = bundles[locale] as Record<string, unknown>;
  const v = bundle[key];
  if (typeof v === 'string') return v;
  const zhBundle = bundles.zh as Record<string, unknown>;
  return typeof zhBundle[key] === 'string' ? zhBundle[key] : key;
}

interface WebI18nValue {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: (key: string, fallback?: string) => string;
  fmtDate: (iso?: string) => string;
  fmtNumber: (n: number) => string;
  fmtFileSize: (bytes: number) => string;
  fmtCost: (cents: number) => string;
}

const WebI18nContext = createContext<WebI18nValue>({
  locale: 'zh', setLocale: () => {}, t: (k, fb) => fb ?? k,
  fmtDate: (iso) => iso ?? '', fmtNumber: String, fmtFileSize: String, fmtCost: String,
});

export function WebI18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => {
    const saved = localStorage.getItem('focalroom-locale');
    if (saved === 'en' || saved === 'ja' || saved === 'ko' || saved === 'zh') return saved;
    const b = navigator.language.toLowerCase();
    if (b.startsWith('ja')) return 'ja';
    if (b.startsWith('ko')) return 'ko';
    if (b.startsWith('zh')) return 'zh';
    return 'en';
  });

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    localStorage.setItem('focalroom-locale', l);
    document.documentElement.lang = l;
  }, []);

  const t = useCallback((key: string, fallback?: string) => {
    return lookup(locale, key) ?? fallback ?? key;
  }, [locale]);

  const tag = LOCALE_TAGS[locale] ?? 'en-US';

  const value: WebI18nValue = {
    locale, setLocale, t,
    fmtDate: (iso?: string) => {
      if (!iso) return '';
      try { return new Intl.DateTimeFormat(tag, { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(iso)); }
      catch { return iso; }
    },
    fmtNumber: (n: number) => new Intl.NumberFormat(tag).format(n),
    fmtFileSize: (bytes: number) => {
      const units = ['B', 'KB', 'MB', 'GB'];
      let size = bytes, unit = 0;
      while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++; }
      return `${new Intl.NumberFormat(tag, { maximumFractionDigits: 1 }).format(size)} ${units[unit]}`;
    },
    fmtCost: (cents: number) => new Intl.NumberFormat(tag, { style: 'currency', currency: 'USD' }).format(cents / 100),
  };

  return <WebI18nContext.Provider value={value}>{children}</WebI18nContext.Provider>;
}

export function useWebI18n() { return useContext(WebI18nContext); }

export function WebLanguageSwitcher() {
  const { locale, setLocale } = useWebI18n();
  return <select
    aria-label="Language"
    value={locale}
    onChange={e => setLocale(e.target.value as Locale)}
    style={{ background: 'var(--bg-secondary,#222)', color: 'var(--text-primary,#eee)', border: '1px solid var(--border,#444)', borderRadius: 6, padding: '4px 8px', fontSize: '0.85rem', cursor: 'pointer' }}
  >
    {LOCALES.map(l => <option key={l.code} value={l.code}>{l.flag} {l.label}</option>)}
  </select>;
}
