import { useRef, useState } from "react";
import { useWebI18n } from "../i18n";
import { request, errorMessage, type Session } from "../shared-api";
const text = {
  zh: { title: "登录桌面端", account: "使用此账户登录 FocalRoom 桌面端", approve: "继续并返回桌面端", busy: "正在连接…", done: "已请求打开 FocalRoom", again: "再次打开桌面端", retry: "重新获取授权", invalid: "登录链接无效，请从桌面端重新发起。" },
  en: { title: "Sign in to desktop", account: "Use this account in FocalRoom desktop", approve: "Continue to desktop", busy: "Connecting…", done: "FocalRoom was requested to open", again: "Open desktop again", retry: "Get a new authorization", invalid: "Invalid sign-in link. Start again from the desktop app." },
  ja: { title: "デスクトップにログイン", account: "このアカウントをFocalRoomで使用", approve: "続けてデスクトップに戻る", busy: "接続中…", done: "FocalRoomを開くよう要求しました", again: "もう一度開く", retry: "認証をやり直す", invalid: "無効なリンクです。アプリからやり直してください。" },
  ko: { title: "데스크톱 로그인", account: "FocalRoom 데스크톱에서 이 계정 사용", approve: "계속하고 데스크톱으로 돌아가기", busy: "연결 중…", done: "FocalRoom 열기를 요청했습니다", again: "다시 열기", retry: "새 인증 받기", invalid: "잘못된 로그인 링크입니다. 앱에서 다시 시작하세요." },
};
export function readDesktopRequest(url: URL) {
  if (url.searchParams.get("desktop") !== "1") return null;
  const requestId = url.searchParams.get("request") ?? "", state = url.searchParams.get("state") ?? "";
  return { requestId, state, valid: ["desktop", "request", "state"].every(k => url.searchParams.getAll(k).length === 1) && /^[a-f0-9]{64}$/.test(requestId) && /^[a-f0-9]{64}$/.test(state) };
}
export function DesktopApproval({ session, intent, onReturn = url => window.location.assign(url) }: {
  session: Session; intent: NonNullable<ReturnType<typeof readDesktopRequest>>; onReturn?: (url: string) => void;
}) {
  const { locale } = useWebI18n(), copy = text[locale];
  const [error, setError] = useState(""), [busy, setBusy] = useState(false), [callback, setCallback] = useState("");
  const pending = useRef(false);
  const approve = async () => {
    if (pending.current || !intent.valid) return;
    pending.current = true; setBusy(true); setError(""); setCallback("");
    try {
      const value = await request<{ code: string }>("/auth/desktop/authorize", session, "POST", { request_id: intent.requestId, state: intent.state });
      if (!/^[a-f0-9]{64}$/.test(value.code)) throw new Error(copy.invalid);
      const url = new URL("focalroom://callback");
      url.search = new URLSearchParams({ code: value.code, state: intent.state }).toString();
      setCallback(url.href); onReturn(url.href);
    } catch (cause) { setError(errorMessage(cause)); }
    finally { pending.current = false; setBusy(false); }
  };
  return <main className="fr-welcome"><section><h1>{copy.title}</h1><p>{copy.account}</p></section><div className="fr-card fr-login">
    <h2>{session.name}</h2><p>{session.email}</p>
    {!intent.valid ? <p role="alert">{copy.invalid}</p> : <>
      <button className="fr-primary" disabled={busy} onClick={() => void approve()}>{busy ? copy.busy : callback ? copy.retry : copy.approve}</button>
      {callback && <><p role="status">{copy.done}</p><a href={callback}>{copy.again}</a></>}
    </>}
    {error && <p role="alert">{error}</p>}
  </div></main>;
}
