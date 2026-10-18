import { DesktopApproval, readDesktopRequest } from "./shared/DesktopApproval";
import { MagicLogin as Login } from "./shared/MagicLogin";
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
import { WebLanguageSwitcher } from "./i18n";
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
          ) : null}
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
    </div>
  );
}
