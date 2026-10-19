// 桌面端云账户：浏览器借道登录（PKCE 一次性授权码），凭据由桌面后端保存。
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { ChevronUp, HelpCircle, LogOut, Mail, Settings2, UserRound } from "lucide-react";
import { zh } from "../../app/copy";
import type { CloudAuthRepository, CloudStatus } from "../../domain/cloud-auth";
import { AccountAvatar } from "../../components/shared/AccountAvatar";
import { useI18n } from "../../i18n";

const copy = {
  zh: { signin: "登录", waiting: "等待浏览器授权…", cancel: "取消", signout: "退出登录", title: "账户", failed: "登录失败或已取消", editName: "修改昵称", editAvatar: "修改头像", save: "保存", namePlaceholder: "输入新昵称", chooseFile: "选择图片", avatarHint: "支持 JPG/PNG，自动压缩到 256KB 内", saved: "已保存", menuLabel: "账户菜单" },
  en: { signin: "Sign in", waiting: "Waiting for browser authorization…", cancel: "Cancel", signout: "Sign out", title: "Account", failed: "Sign-in failed or was canceled", editName: "Edit name", editAvatar: "Edit avatar", save: "Save", namePlaceholder: "Enter a new name", chooseFile: "Choose image", avatarHint: "JPG/PNG, auto-compressed under 256KB", saved: "Saved", menuLabel: "Account menu" },
  ja: { signin: "ログイン", waiting: "ブラウザ認証を待っています…", cancel: "キャンセル", signout: "ログアウト", title: "アカウント", failed: "ログインに失敗したか中止されました", editName: "名前を変更", editAvatar: "アイコンを変更", save: "保存", namePlaceholder: "新しい名前を入力", chooseFile: "画像を選択", avatarHint: "JPG/PNG、256KB以下に自動圧縮", saved: "保存しました", menuLabel: "アカウントメニュー" },
  ko: { signin: "로그인", waiting: "브라우저 인증 대기 중…", cancel: "취소", signout: "로그아웃", title: "계정", failed: "로그인에 실패했거나 취소되었습니다", editName: "이름 변경", editAvatar: "프로필 사진 변경", save: "저장", namePlaceholder: "새 이름 입력", chooseFile: "이미지 선택", avatarHint: "JPG/PNG, 256KB 이하로 자동 압축", saved: "저장됨", menuLabel: "계정 메뉴" },
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
    let cleanup: (() => void) | undefined;
    void repository.subscribe(bump).then(fn => { if (active) cleanup = fn; else fn(); }).catch(() => {});
    const seqOnMount = seqRef;
    return () => { active = false; seqOnMount.current += 1; cleanup?.(); };
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

type AccountActions = { onSettings?: () => void; onHelp?: () => void; onContact?: () => void };
function CloudAuthBody({ repository, copy: text, onClose, onSettings, onHelp, onContact }: { repository?: CloudAuthRepository; copy: Copy; onClose?: () => void } & AccountActions) {
  const { status, refresh } = useCloudStatus(repository);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [menuOpen, setMenuOpen] = useState(false);
  const [editing, setEditing] = useState<"name" | "avatar" | null>(null);
  const [nameInput, setNameInput] = useState("");
  const [notice, setNotice] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuId = useId();
  const open = useCallback(() => {
    if (!repository) return;
    setBusy(true); setNotice(""); setError("");
    repository.open().catch(cause => { setError(cause instanceof Error ? cause.message : text.failed); }).finally(() => { setBusy(false); refresh(); });
  }, [repository, refresh, text.failed]);
  const cancel = () => { if (repository) void repository.cancel().then(refresh); };
  const signOut = () => {
    if (!repository) return;
    setBusy(true);
    repository.logout().then(revoked => { if (!revoked) setError("已在本机退出；云端会话未能撤销"); })
      .catch(() => {}).finally(() => { setBusy(false); refresh(); });
  };
  const choose = (action: () => void) => { setMenuOpen(false); triggerRef.current?.focus(); action(); };
  useEffect(() => {
    const onDoc = (event: MouseEvent) => { if (menuRef.current && !menuRef.current.contains(event.target as Node)) setMenuOpen(false); };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);
  const saveName = async () => {
    if (!repository || !nameInput.trim()) return;
    setBusy(true); setNotice("");
    try { await repository.updateProfile(nameInput.trim()); setEditing(null); setNotice(text.saved); refresh(); }
    catch { setNotice(text.failed); }
    finally { setBusy(false); }
  };
  const onAvatarFile = async (file: File) => {
    if (!repository) return;
    setBusy(true); setNotice("");
    try { await repository.updateProfile(undefined, await compressAvatar(file)); setEditing(null); setNotice(text.saved); refresh(); }
    catch { setNotice(text.failed); }
    finally { setBusy(false); }
  };
  const user = status?.user;
  const name = user?.name || user?.email || zh.notSignedIn;
  return <div className="cloud-account" data-testid="cloud-account">
    {onClose && !user ? <>
      {status?.waiting ? <div role="status">{text.waiting}<button type="button" onClick={cancel}>{text.cancel}</button></div>
       : <button type="button" className="fr-text" disabled={busy} onClick={open}>{text.signin}</button>}
      <button type="button" className="fr-text" onClick={onClose}>{text.cancel}</button>
    </> : <div className="cloud-account-menu" ref={menuRef}>
      <button ref={triggerRef} type="button" className="cloud-account-trigger" aria-label={text.menuLabel} aria-haspopup="menu" aria-controls={menuId} aria-expanded={menuOpen}
        onClick={() => setMenuOpen(value => !value)} onKeyDown={event => { if (event.key === "ArrowDown") { event.preventDefault(); setMenuOpen(true); requestAnimationFrame(() => menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus()); } }}>
        {user ? <AccountAvatar className="cloud-avatar" url={user.avatar} name={name} allowEmbedded /> : <span className="cloud-avatar guest-avatar"><UserRound size={19} /></span>}
        <span className="account-trigger-label"><strong>{name}</strong><small>{status?.waiting ? zh.signingIn : user ? user.email : zh.accountGuestHint}</small></span><ChevronUp size={14} />
      </button>
      {menuOpen && <div id={menuId} className="cloud-account-dropdown" role="menu" onKeyDown={event => {
        const items = [...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')];
        const index = items.indexOf(document.activeElement as HTMLButtonElement);
        if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) { event.preventDefault(); const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (index + (event.key === "ArrowUp" ? -1 : 1) + items.length) % items.length; items[next]?.focus(); }
        if (event.key === "Escape") { event.preventDefault(); setMenuOpen(false); triggerRef.current?.focus(); }
      }}>
        <div className="cloud-account-dropdown-head"><div className="cloud-account-name">{name}</div>{user && <div className="cloud-account-email">{user.email}</div>}{status?.error && <div className="cloud-account-offline" role="status">{status.error}</div>}</div>
        {!user && <button type="button" role="menuitem" disabled={busy || !repository || status?.waiting} onClick={() => choose(open)}><UserRound size={15} />{text.signin}</button>}
        {user && <><button type="button" role="menuitem" onClick={() => choose(() => { setEditing("name"); setNameInput(user.name); })}><UserRound size={15} />{text.editName}</button>
        <button type="button" role="menuitem" onClick={() => choose(() => setEditing("avatar"))}><UserRound size={15} />{text.editAvatar}</button></>}
        {onSettings && <button type="button" role="menuitem" onClick={() => choose(onSettings)}><Settings2 size={15} />{zh.settings}</button>}
        {onHelp && <button type="button" role="menuitem" onClick={() => choose(onHelp)}><HelpCircle size={15} />{zh.help}</button>}
        {onContact && <button type="button" role="menuitem" onClick={() => choose(onContact)}><Mail size={15} />{zh.contactAuthor}</button>}
        {user && <button type="button" role="menuitem" className="cloud-account-signout" disabled={busy} onClick={() => choose(signOut)}><LogOut size={15} />{text.signout}</button>}
      </div>}
    </div>}
    {!onClose && status?.waiting && <div className="cloud-account-waiting" role="status"><span>{text.waiting}</span><button type="button" onClick={cancel}>{text.cancel}</button></div>}
    {editing === "name" && <div className="cloud-account-edit" role="dialog" aria-label={text.editName}>
      <input aria-label={text.editName} value={nameInput} maxLength={200} placeholder={text.namePlaceholder} onChange={event => setNameInput(event.target.value)} disabled={busy} />
      <button type="button" className="fr-text" disabled={busy || !nameInput.trim()} onClick={() => { void saveName(); }}>{text.save}</button><button type="button" className="fr-text" onClick={() => setEditing(null)}>{text.cancel}</button>
    </div>}
    {editing === "avatar" && <div className="cloud-account-edit" role="dialog" aria-label={text.editAvatar}>
      <input ref={fileRef} type="file" accept="image/jpeg,image/png" hidden onChange={event => { const file = event.target.files?.[0]; if (file) void onAvatarFile(file); }} />
      <button type="button" className="fr-text" disabled={busy} onClick={() => fileRef.current?.click()}>{busy ? text.waiting : text.chooseFile}</button><span className="cloud-account-hint">{text.avatarHint}</span><button type="button" className="fr-text" onClick={() => setEditing(null)}>{text.cancel}</button>
    </div>}
    {notice && <span role="status" className="cloud-account-notice">{notice}</span>}
    {(error || (!menuOpen && status?.error)) && <p role="alert" className="error-message">{error || status?.error}</p>}
  </div>;
}

export function CloudAccount({ repository, ...actions }: { repository?: CloudAuthRepository } & AccountActions) {
  const { locale } = useI18n();
  return <CloudAuthBody repository={repository} copy={copy[locale]} {...actions} />;
}
export function CloudLoginView({ repository, onClose }: { repository: CloudAuthRepository; onClose: () => void }) {
  const { locale } = useI18n();
  const text = copy[locale];
  return <section className="info-card cloud-login" aria-label={text.title}><h3>{text.title}</h3><CloudAuthBody repository={repository} copy={text} onClose={onClose} /></section>;
}
