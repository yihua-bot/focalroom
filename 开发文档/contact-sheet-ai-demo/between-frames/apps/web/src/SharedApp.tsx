import {AccountAvatar} from "../../../src/components/shared/AccountAvatar";
import { DesktopApproval, readDesktopRequest } from "./shared/DesktopApproval";
import { MagicLogin as Login } from "./shared/MagicLogin";
import { useEffect, useRef, useState } from "react";
import {
  request,
  sessionSchema,
  type Session,
  errorMessage as message,
} from "./shared-api";
import { ProjectHome } from "./shared/ProjectHome";
import { ProjectReview } from "./shared/ProjectReview";
import { AccountRecovery, readAccountLink, VerificationRequest } from "./shared/AccountRecovery";
import { WebLanguageSwitcher,useWebI18n } from "./i18n";
import "./shared.css";
import { BillingDialog } from './shared/BillingDialog';
import { billingCopy } from '../../../src/features/subscription/copy';
import type { SubscriptionPage } from '../../../src/features/subscription/SubscriptionDialog';
function AccountMenu({ session, onLogout, onChanged, onBilling }: { session: Session; onLogout: () => void; onChanged: (s: Session) => void; onBilling: (page: SubscriptionPage) => void }) {
  const [open, setOpen] = useState(false);
  const { locale } = useWebI18n();
  const menuText={signout:{zh:"退出登录",en:"Sign out",ja:"ログアウト",ko:"로그아웃"},editName:{zh:"修改昵称",en:"Edit name",ja:"名前を変更",ko:"이름 변경"},editAvatar:{zh:"修改头像",en:"Edit avatar",ja:"アイコンを変更",ko:"프로필 사진 변경"},cancel:{zh:"取消修改",en:"Cancel",ja:"キャンセル",ko:"취소"},save:{zh:"保存",en:"Save",ja:"保存",ko:"저장"}};
  const [editing, setEditing] = useState<"name" | null>(null);
  const [name, setName] = useState(session.name);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onDoc = (e: MouseEvent) => { if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) { setOpen(false); setEditing(null); } };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);
  async function compress(file: File): Promise<string> {
    const bitmap = await createImageBitmap(file);
    const scale = Math.min(512, Math.max(bitmap.width, bitmap.height)) / Math.max(bitmap.width, bitmap.height);
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(bitmap.width * scale); canvas.height = Math.round(bitmap.height * scale);
    canvas.getContext("2d")!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    let q = 0.85, data = canvas.toDataURL("image/jpeg", q);
    while (data.length > 341_000 && q > 0.3) { q -= 0.15; data = canvas.toDataURL("image/jpeg", q); }
    return data;
  }
  const save = async (displayName?: string, avatarUrl?: string) => {
    setBusy(true); setError("");
    try {
      const result = await request<Session>("/auth/profile", session, "PATCH",
        { ...(displayName !== undefined ? { display_name: displayName } : {}), ...(avatarUrl !== undefined ? { avatar_url: avatarUrl } : {}) });
      onChanged({ ...session, ...result, csrf: session.csrf });
      setEditing(null);
    } catch (e) { setError(message(e)); }
    finally { setBusy(false); }
  };
  return <div className="fr-account-menu" ref={wrapRef}>
    <button type="button" className="fr-account-trigger" aria-label="账户菜单" aria-expanded={open} onClick={() => setOpen(v => !v)}>
      <AccountAvatar className="fr-avatar" url={session.avatar} name={session.name || session.email} allowEmbedded />
    </button>
    {open && <div className="fr-account-dropdown" role="menu">
      <div className="fr-account-head"><b>{session.name || session.email}</b><small>{session.email}</small></div>
      <button type="button" role="menuitem" onClick={() => { setOpen(false); onBilling('usage'); }}>{billingCopy[locale].usage}</button>
      <button type="button" role="menuitem" onClick={() => { setOpen(false); onBilling('billing'); }}>{billingCopy[locale].billing}</button>
      <button type="button" role="menuitem" onClick={() => { setOpen(false); onBilling('plans'); }}>{billingCopy[locale].plans}</button>
      <button type="button" role="menuitem" onClick={() => { setEditing(editing === "name" ? null : "name"); setName(session.name); }}>{editing === "name" ? menuText.cancel[locale] : menuText.editName[locale]}</button>
      <button type="button" role="menuitem" onClick={() => fileRef.current?.click()}>{menuText.editAvatar[locale]}</button>
      <button type="button" role="menuitem" className="danger" onClick={onLogout}>{menuText.signout[locale]}</button>
      {editing === "name" && <div className="fr-account-edit">
        <input value={name} maxLength={200} disabled={busy} onChange={e => setName(e.target.value)} />
        <button type="button" disabled={busy || !name.trim()} onClick={() => void save(name.trim())}>{menuText.save[locale]}</button>
      </div>}
      {error && <div className="fr-account-edit"><small style={{ color: "#b3423a" }}>{error}</small></div>}
    </div>}
    <input ref={fileRef} type="file" accept="image/jpeg,image/png" hidden onChange={e => { const f = e.target.files?.[0]; if (f) void compress(f).then(d => void save(undefined, d)); e.target.value = ""; }} />
  </div>;
}
function Brand() {
  return (
    <a className="fr-brand" href="/shared">
      <img src="/focalroom-mark.png" alt="FocalRoom 标志" />
      <span>
        Focal<span>Room</span>
      </span>
    </a>
  );
}
export default function SharedApp() {
  const [billingPage, setBillingPage] = useState<SubscriptionPage | null>(() => new URLSearchParams(location.hash.slice(1)).has('billing') ? 'billing' : null);
  const [accountLink, setAccountLink] = useState(() => readAccountLink(new URL(location.href)));
  const [session, setSession] = useState<Session | null>(null),
    [ready, setReady] = useState(false),
    [route, setRoute] = useState(() => location.hash),
    [error, setError] = useState("");
  const desktopIntent = readDesktopRequest(new URL(location.href));
  const projectId = new URLSearchParams(route.slice(1)).get("project");
  const openProject = (id: string | null) => {
    const hash = id ? `#project=${encodeURIComponent(id)}` : "";
    history.pushState(null, "", `/shared${hash}`);
    setRoute(hash);
  };
  useEffect(() => {
    const syncRoute = (event: Event) => {
      // 邮件页会 replaceState 清理令牌；hashchange 须读取事件目标，
      // 避免紧随 popstate 的事件误把刚打开的邮件页关闭。
      const url = new URL(event instanceof HashChangeEvent ? event.newURL : location.href);
      setRoute(url.hash);
      if (new URLSearchParams(url.hash.slice(1)).has('billing')) setBillingPage('billing');
      const link = readAccountLink(url);
      if (link) setAccountLink(link);
      // 清理后的空地址不丢弃内存中的令牌；通过明确的返回按钮退出。
      else if (url.hash) setAccountLink(null);
    };
    window.addEventListener("hashchange", syncRoute);
    window.addEventListener("popstate", syncRoute);
    return () => {
      window.removeEventListener("hashchange", syncRoute);
      window.removeEventListener("popstate", syncRoute);
    };
  }, []);
  useEffect(() => {
    document.title = "FocalRoom — 团队协作审稿";
    let active = true;
    request("/auth/me", null)
      .then((value) => {
        if (active) setSession(sessionSchema.parse(value));
      })
      .catch(() => {})
      .finally(() => {
        if (active) setReady(true);
      });
    return () => {
      active = false;
    };
  }, []);
  const logout = async () => {
    try {
      await request("/auth/logout", session, "POST");
      setSession(null);
      setBillingPage(null);
      setAccountLink(null);
      if (!desktopIntent) openProject(null);
    } catch (error) {
      setError(message(error));
    }
  };
  return (
    <div className="fr-shared">
      <header className="fr-top">
        <Brand />
        <span className="fr-label">共享评片</span>
        <div className="fr-top-right">
          <WebLanguageSwitcher />
          {session ? <AccountMenu session={session} onLogout={() => void logout()} onChanged={setSession} onBilling={setBillingPage} /> : null}
        </div>
      </header>
      {error && (
        <div role="alert" className="fr-error">
          {error}
        </div>
      )}
      {accountLink ? (
        <AccountRecovery key={`${accountLink.mode}:${accountLink.token}`} link={accountLink} onBack={resetCompleted => {
          if (resetCompleted) setSession(null);
          setAccountLink(null);
          openProject(null);
        }} />
      ) : !ready ? (
        <p className="fr-loading">正在连接工作区…</p>
      ) : !session ? (
        <Login onLogin={setSession} />
      ) : desktopIntent ? (
        <DesktopApproval session={session} intent={desktopIntent} />
      ) : projectId ? (
        <ProjectReview
          key={projectId}
          id={projectId}
          session={session}
          onBack={() => openProject(null)}
        />
      ) : (
        <><VerificationRequest session={session} /><ProjectHome key={route} session={session} onOpen={openProject} /></>
      )}
      {session && billingPage && <BillingDialog key={session.id} session={session} page={billingPage} onClose={() => {
        setBillingPage(null);
        if (new URLSearchParams(location.hash.slice(1)).has('billing')) { history.replaceState(null, '', '/shared'); setRoute(''); }
      }} />}
    </div>
  );
}
