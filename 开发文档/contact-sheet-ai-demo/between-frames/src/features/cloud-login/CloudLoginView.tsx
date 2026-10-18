// 桌面端云账户：浏览器借道登录（PKCE 一次性授权码），凭据由桌面后端保存。
import { useCallback, useEffect, useState } from "react";
import type { CloudAuthRepository, CloudStatus } from "../../domain/cloud-auth";
import { useI18n } from "../../i18n";

const copy = {
  zh: { signin: "登录云端", waiting: "等待浏览器授权…", cancel: "取消", signout: "退出登录", title: "云端账户", failed: "登录失败或已取消" },
  en: { signin: "Sign in to Cloud", waiting: "Waiting for browser authorization…", cancel: "Cancel", signout: "Sign out", title: "Cloud Account", failed: "Sign-in failed or was canceled" },
  ja: { signin: "クラウドにログイン", waiting: "ブラウザ認証を待っています…", cancel: "キャンセル", signout: "ログアウト", title: "クラウドアカウント", failed: "ログインに失敗したか中止されました" },
  ko: { signin: "클라우드 로그인", waiting: "브라우저 인증 대기 중…", cancel: "취소", signout: "로그아웃", title: "클라우드 계정", failed: "로그인에 실패했거나 취소되었습니다" },
} as const;
type Copy = (typeof copy)[keyof typeof copy];

export function useCloudStatus(repository?: CloudAuthRepository): { status: CloudStatus | null } {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  useEffect(() => {
    if (!repository) return;
    let active = true;
    const refresh = () => repository.status().then(value => { if (active) setStatus(value); }).catch(() => {});
    void refresh();
    void repository.subscribe(() => void refresh());
    return () => { active = false; };
  }, [repository]);
  return { status };
}

function CloudAuthBody({ repository, copy: text, onClose }: { repository: CloudAuthRepository; copy: Copy; onClose?: () => void }) {
  const { status } = useCloudStatus(repository);
  const [busy, setBusy] = useState(false);
  const open = useCallback(() => { setBusy(true); repository.open().catch(() => {}).finally(() => setBusy(false)); }, [repository]);
  const cancel = useCallback(() => void repository.cancel(), [repository]);
  if (!status) return null;
  if (status.authenticated && status.user) {
    const name = status.user.name || status.user.email;
    return <div className="cloud-account" data-testid="cloud-account">
      <span className="cloud-account-status" title={status.error ?? undefined} aria-label={status.error ? "离线" : "已连接"} data-offline={status.error ? "true" : undefined}>●</span>
      {status.user.avatar ? (
        <img className="cloud-avatar" src={status.user.avatar} alt="" referrerPolicy="no-referrer" />
      ) : (
        <span className="cloud-avatar cloud-avatar-fallback" aria-hidden="true">{name.slice(0, 1).toUpperCase()}</span>
      )}
      <div className="cloud-account-user" title={status.user.email}>
        <div className="cloud-account-name">{name}</div>
        <div className="cloud-account-email">{status.user.email}</div>
      </div>
      <button type="button" className="fr-text" onClick={() => void repository.logout()}>{text.signout}</button>
    </div>;
  }
  if (status.waiting) {
    return <div className="cloud-account cloud-account-waiting" role="status">
      <span>{text.waiting}</span>
      <button type="button" className="fr-text" onClick={cancel}>{text.cancel}</button>
    </div>;
  }
  return <div className="cloud-account">
    {status.error && <p role="alert" className="error-message">{text.failed}</p>}
    <button type="button" className="fr-text" disabled={busy} onClick={open}>{text.signin}</button>
    {onClose && <button type="button" className="fr-text" onClick={onClose}>{text.cancel}</button>}
  </div>;
}

/** 侧栏底部账户块：未登录显示入口，已登录显示身份与退出。 */
export function CloudAccount({ repository }: { repository: CloudAuthRepository }) {
  const { locale } = useI18n();
  return <CloudAuthBody repository={repository} copy={copy[locale]} />;
}

/** 独立登录视图（弹窗用）：带标题与关闭。 */
export function CloudLoginView({ repository, onClose }: { repository: CloudAuthRepository; onClose: () => void }) {
  const { locale } = useI18n();
  const text = copy[locale];
  return <section className="info-card cloud-login" aria-label={text.title}>
    <h3>{text.title}</h3>
    <CloudAuthBody repository={repository} copy={text} onClose={onClose} />
  </section>;
}
