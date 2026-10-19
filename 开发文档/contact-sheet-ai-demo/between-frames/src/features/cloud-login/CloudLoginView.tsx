// 桌面端云账户：浏览器借道登录（PKCE 一次性授权码），凭据由桌面后端保存。
import { useCallback, useEffect, useRef, useState } from "react";
import type { CloudAuthRepository, CloudStatus } from "../../domain/cloud-auth";
import { AccountAvatar } from "../../components/shared/AccountAvatar";
import { useI18n } from "../../i18n";

const copy = {
  zh: { signin: "登录云端", waiting: "等待浏览器授权…", cancel: "取消", signout: "退出登录", title: "云端账户", failed: "登录失败或已取消", editName: "修改昵称", editAvatar: "修改头像", save: "保存", namePlaceholder: "输入新昵称", chooseFile: "选择图片", avatarHint: "支持 JPG/PNG，自动压缩到 256KB 内", saved: "已保存", menuLabel: "账户菜单" },
  en: { signin: "Sign in to Cloud", waiting: "Waiting for browser authorization…", cancel: "Cancel", signout: "Sign out", title: "Cloud Account", failed: "Sign-in failed or was canceled", editName: "Edit name", editAvatar: "Edit avatar", save: "Save", namePlaceholder: "Enter a new name", chooseFile: "Choose image", avatarHint: "JPG/PNG, auto-compressed under 256KB", saved: "Saved", menuLabel: "Account menu" },
  ja: { signin: "クラウドにログイン", waiting: "ブラウザ認証を待っています…", cancel: "キャンセル", signout: "ログアウト", title: "クラウドアカウント", failed: "ログインに失敗したか中止されました", editName: "名前を変更", editAvatar: "アイコンを変更", save: "保存", namePlaceholder: "新しい名前を入力", chooseFile: "画像を選択", avatarHint: "JPG/PNG、256KB以下に自動圧縮", saved: "保存しました", menuLabel: "アカウントメニュー" },
  ko: { signin: "클라우드 로그인", waiting: "브라우저 인증 대기 중…", cancel: "취소", signout: "로그아웃", title: "클라우드 계정", failed: "로그인에 실패했거나 취소되었습니다", editName: "이름 변경", editAvatar: "프로필 사진 변경", save: "저장", namePlaceholder: "새 이름 입력", chooseFile: "이미지 선택", avatarHint: "JPG/PNG, 256KB 이하로 자동 압축", saved: "저장됨", menuLabel: "계정 메뉴" },
} as const;
type Copy = (typeof copy)[keyof typeof copy];

export function useCloudStatus(repository?: CloudAuthRepository): { status: CloudStatus | null; refresh: () => void } {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  const seqRef = useRef(0);
  const refresh = useCallback(() => {
    if (!repository) return;
    const ticket = ++seqRef.current;
    repository.status().then(value => { if (ticket === seqRef.current) setStatus(value); }).catch(() => {});
  }, [repository]);
  useEffect(() => {
    if (!repository) return;
    let active = true;
    const bump = () => { if (active) refresh(); };
    bump();
    void repository.subscribe(bump);
    const seqOnMount = seqRef;
    return () => { active = false; seqOnMount.current += 1; };
  }, [repository, refresh]);
  return { status, refresh };
}

/** 选本机图片 → canvas 压缩为 ≤256KB 的 JPEG data URL。 */
async function compressAvatar(file: File): Promise<string> {
  const bitmap = await createImageBitmap(file);
  const edge = Math.min(512, Math.max(bitmap.width, bitmap.height));
  const scale = edge / Math.max(bitmap.width, bitmap.height);
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(bitmap.width * scale));
  canvas.height = Math.max(1, Math.round(bitmap.height * scale));
  canvas.getContext("2d")!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  let quality = 0.85, data = canvas.toDataURL("image/jpeg", quality);
  while (data.length > 341_000 && quality > 0.3) { quality -= 0.15; data = canvas.toDataURL("image/jpeg", quality); }
  return data;
}

function CloudAuthBody({ repository, copy: text, onClose }: { repository: CloudAuthRepository; copy: Copy; onClose?: () => void }) {
  const { status, refresh } = useCloudStatus(repository);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [menuOpen, setMenuOpen] = useState(false);
  const [editing, setEditing] = useState<"name" | "avatar" | null>(null);
  const [nameInput, setNameInput] = useState("");
  const [notice, setNotice] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const open = useCallback(() => {
    setBusy(true); setNotice("");
    repository.open().catch(cause => { setError(cause instanceof Error ? cause.message : "登录失败或已取消"); }).finally(() => setBusy(false));
  }, [repository]);
  const cancel = useCallback(() => void repository.cancel(), [repository]);
  const signOut = useCallback(() => {
    setBusy(true);
    repository.logout().then(revoked => { if (!revoked) setError("已在本机退出；云端会话未能撤销"); })
      .catch(() => {})
      .finally(() => { setBusy(false); refresh(); });
  }, [repository, refresh]);

  useEffect(() => {
    const onDoc = (event: MouseEvent) => { if (menuRef.current && !menuRef.current.contains(event.target as Node)) setMenuOpen(false); };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  const saveName = async () => {
    const name = nameInput.trim();
    if (!name) return;
    setBusy(true); setNotice("");
    try { await repository.updateProfile(name); setEditing(null); setNotice(text.saved); }
    catch { setNotice(text.failed); }
    finally { setBusy(false); }
  };
  const onAvatarFile = async (file: File) => {
    setBusy(true); setNotice("");
    try { await repository.updateProfile(undefined, await compressAvatar(file)); setEditing(null); setNotice(text.saved); }
    catch { setNotice(text.failed); }
    finally { setBusy(false); }
  };

  if (!status) return null;
  if (status.user) {
    const name = status.user.name || status.user.email;
    return <div className="cloud-account" data-testid="cloud-account">
      <div className="cloud-account-menu" ref={menuRef}>
        <button type="button" className="cloud-account-trigger" aria-label={text.menuLabel} aria-expanded={menuOpen} onClick={() => setMenuOpen(v => !v)}>
          <AccountAvatar className="cloud-avatar" url={status.user.avatar} name={name} allowEmbedded />
        </button>
        {menuOpen && <div className="cloud-account-dropdown" role="menu">
          <div className="cloud-account-dropdown-head">
            <div className="cloud-account-name">{name}</div>
            <div className="cloud-account-email">{status.user.email}</div>
            {!status.authenticated && status.error && <div className="cloud-account-offline" role="status">{status.error}</div>}
          </div>
          <button type="button" role="menuitem" onClick={() => { setEditing("name"); setNameInput(status.user!.name); setMenuOpen(false); }}>{text.editName}</button>
          <button type="button" role="menuitem" onClick={() => { setEditing("avatar"); setMenuOpen(false); }}>{text.editAvatar}</button>
          <button type="button" role="menuitem" className="cloud-account-signout" onClick={() => { setMenuOpen(false); signOut(); }}>{text.signout}</button>
        </div>}
      </div>
      {editing === "name" && <div className="cloud-account-edit" role="dialog" aria-label={text.editName}>
        <input value={nameInput} maxLength={200} placeholder={text.namePlaceholder} onChange={e => setNameInput(e.target.value)} disabled={busy} />
        <button type="button" className="fr-text" disabled={busy || !nameInput.trim()} onClick={() => void saveName()}>{text.save}</button>
        <button type="button" className="fr-text" onClick={() => setEditing(null)}>{text.cancel}</button>
      </div>}
      {editing === "avatar" && <div className="cloud-account-edit" role="dialog" aria-label={text.editAvatar}>
        <input ref={fileRef} type="file" accept="image/jpeg,image/png" hidden onChange={e => { const f = e.target.files?.[0]; if (f) void onAvatarFile(f); }} />
        <button type="button" className="fr-text" disabled={busy} onClick={() => fileRef.current?.click()}>{busy ? text.waiting : text.chooseFile}</button>
        <span className="cloud-account-hint">{text.avatarHint}</span>
        <button type="button" className="fr-text" onClick={() => setEditing(null)}>{text.cancel}</button>
      </div>}
        {notice && <span role="status" className="cloud-account-notice">{notice}</span>}
        {error && <p role="alert" className="error-message">{error}</p>}
      </div>;
  }
  if (status.waiting) {
    return <div className="cloud-account cloud-account-waiting" role="status">
      <span>{text.waiting}</span>
      <button type="button" className="fr-text" onClick={cancel}>{text.cancel}</button>
    </div>;
  }
  return <div className="cloud-account">
    {(error || status.error) && <p role="alert" className="error-message">{error || status.error}</p>}
    <button type="button" className="fr-text" disabled={busy} onClick={open}>{text.signin}</button>
    {onClose && <button type="button" className="fr-text" onClick={onClose}>{text.cancel}</button>}
  </div>;
}

/** 侧栏底部账户块：未登录显示入口，已登录点头像弹菜单。 */
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
