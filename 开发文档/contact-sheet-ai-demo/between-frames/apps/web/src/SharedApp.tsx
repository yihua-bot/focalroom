import { useEffect, useState, type FormEvent } from "react";
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
            <a href="/">体验本地工作台</a>
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
  const [register, setRegister] = useState(false),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    setBusy(true);
    setError("");
    try {
      const result = await request<{
        csrf: string;
        user: { id: string; email: string; name: string };
      }>(register ? "/auth/register" : "/auth/login", null, "POST", {
        email: form.get("email"),
        password: form.get("password"),
        ...(register ? { display_name: form.get("name") } : {}),
      });
      onLogin(sessionSchema.parse({ ...result.user, csrf: result.csrf }));
    } catch (error) {
      setError(message(error));
    } finally {
      setBusy(false);
    }
  };
  if (recovering) return <AccountRecovery onBack={() => setRecovering(false)} />;
  return (
    <main className="fr-welcome">
      <section>
        <p className="fr-eyebrow">让作品在交流中成形</p>
        <h1>
          在同一间暗房里，
          <br />
          找到照片之间的联系。
        </h1>
        <p>
          私密邀请你的编辑与伙伴，留下批注、提出序列建议。每次正式改动，都由作者审阅和确认。
        </p>
        <div className="fr-promise">
          <span>只分享预览</span>
          <span>原图保留本地</span>
          <span>AI 上传单独授权</span>
        </div>
      </section>
      <form
        className="fr-card fr-login"
        onSubmit={(event) => void submit(event)}
      >
        <h2>{register ? "建立你的工作区" : "欢迎回到 FocalRoom"}</h2>
        <p>使用受邀邮箱登录，继续评片。</p>
        {register && (
          <label>
            姓名
            <input name="name" autoComplete="name" required maxLength={200} />
          </label>
        )}
        <label>
          邮箱
          <input name="email" type="email" autoComplete="email" required />
        </label>
        <label>
          密码
          <input
            name="password"
            type="password"
            autoComplete={register ? "new-password" : "current-password"}
            minLength={8}
            maxLength={1024}
            required
          />
        </label>
        {error && (
          <p role="alert" className="fr-error">
            {error}
          </p>
        )}
        <button className="fr-primary" disabled={busy}>
          {busy ? "正在连接…" : register ? "注册并进入" : "登录"}
        </button>
        <button
          type="button"
          className="fr-text"
          onClick={() => {
            setRegister(!register);
            setError("");
          }}
        >
          {register ? "已有账户，去登录" : "首次使用，创建账户"}
        </button>
        {!register && <button type="button" className="fr-text" disabled={busy} onClick={() => setRecovering(true)}>{t("web.forgotPassword")}</button>}
      </form>
    </main>
  );
}
