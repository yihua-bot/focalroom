import { useEffect, useState } from "react";
import {
  request,
  sessionSchema,
  type Session,
  errorMessage as message,
} from "./shared-api";
import { ProjectHome } from "./shared/ProjectHome";
import { ProjectReview } from "./shared/ProjectReview";
import { AccountRecovery, readAccountLink, VerificationRequest } from "./shared/AccountRecovery";
import { useWebI18n, WebLanguageSwitcher } from "./i18n";
import "./shared.css";
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
  const [accountLink, setAccountLink] = useState(() => readAccountLink(new URL(location.href)));
  const [session, setSession] = useState<Session | null>(null),
    [ready, setReady] = useState(false),
    [route, setRoute] = useState(() => location.hash),
    [error, setError] = useState("");
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
    document.title = "FocalRoom · 共享评片";
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
      setAccountLink(null);
      openProject(null);
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
          {session ? (
            <>
              <span>{session.name}</span>
              <button onClick={() => void logout()}>退出登录</button>
            </>
          ) : (
            <a href="/">查看工作台示例</a>
          )}
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
    </div>
  );
}
function Login({ onLogin }: { onLogin: (session: Session) => void }) {
  const { t } = useWebI18n();
  const [recovering, setRecovering] = useState(false);
  const [magicStep, setMagicStep] = useState<'email' | 'code' | null>(null);
  const [magicEmail, setMagicEmail] = useState('');
  const [error, setError] = useState('');
  const [busy] = useState(false);
  const requestMagic = async (email: string) => {
    try {
      await request('/auth/magic/request', null, 'POST', { email });
      setMagicEmail(email);
      setMagicStep('code');
    } catch (e) { setError(message(e)); }
  };
  const verifyMagic = async (code: string) => {
    try {
      const result = await request<{ token: string; user_id: string; display_name: string }>(
        '/auth/magic/verify', null, 'POST', { email: magicEmail, code },
      );
      onLogin({ id: result.user_id, email: magicEmail, name: result.display_name, csrf: result.token });
    } catch (e) { setError(message(e)); }
  };
  if (recovering) return <AccountRecovery onBack={() => setRecovering(false)} />;
  return (
    <main className="fr-welcome">
      <section>
        <h1>共享评片</h1>
        <p>查看照片、批注和编辑提案。</p>
      </section>
      <div className="fr-card fr-login">
        {magicStep === null && (<>
          <h2>登录 FocalRoom</h2>
          <p>输入邮箱，我们会发送验证码。</p>
          <label>
            邮箱
            <input type="email" autoComplete="email" required value={magicEmail}
              onChange={e => setMagicEmail(e.target.value)} />
          </label>
          {error && <p role="alert" className="fr-error">{error}</p>}
          <button className="fr-primary" disabled={busy || !magicEmail}
            onClick={e => { e.preventDefault(); void requestMagic(magicEmail); }}>
            {busy ? "发送中…" : "发送验证码"}
          </button>
        </>)}
        {magicStep === 'code' && (<>
          <h2>输入验证码</h2>
          <p>已发送到 {magicEmail}</p>
          <label>
            验证码
            <input type="text" inputMode="numeric" pattern="[0-9]{6}" maxLength={6}
              required placeholder="000000" autoComplete="one-time-code"
              onChange={e => { if (e.target.value.length === 6) void verifyMagic(e.target.value); }} />
          </label>
          {error && <p role="alert" className="fr-error">{error}</p>}
          <button className="fr-primary" disabled={busy}>验证中…</button>
          <button type="button" style={{marginTop:'.5rem'}} onClick={() => setMagicStep(null)}>← 重新输入邮箱</button>
        </>)}
        <p style={{marginTop:'.75rem',fontSize:'.85rem',color:'var(--text2,#888)'}}>
          没有账号？验证码登录会自动为你创建。
        </p>
        {!recovering && <button type="button" className="fr-text" disabled={busy} onClick={() => setRecovering(true)}>{t("web.forgotPassword")}</button>}
      </div>
    </main>
  );
}
