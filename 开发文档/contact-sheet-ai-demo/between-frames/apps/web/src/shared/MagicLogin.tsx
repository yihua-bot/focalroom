import { useRef, useState } from "react";
import { GoogleOAuthProvider, useGoogleLogin } from "@react-oauth/google";
import { useWebI18n } from "../i18n";
import { request, sessionSchema, type Session, errorMessage } from "../shared-api";
import { AccountRecovery } from "./AccountRecovery";

const GOOGLE_CLIENT_ID = "397192072857-eeeo0i8gjjkaol413qek43l10artfvq4.apps.googleusercontent.com";
const googleCopy = {
  zh: { button: "使用 Google 登录", fail: "Google 登录失败，请重试" },
  en: { button: "Sign in with Google", fail: "Google sign-in failed. Please try again." },
  ja: { button: "Google でログイン", fail: "Google ログインに失敗しました。もう一度お試しください。" },
  ko: { button: "Google로 로그인", fail: "Google 로그인에 실패했습니다. 다시 시도해 주세요." },
} as const;

const copy = {
  zh: { title: "登录 FocalRoom", email: "邮箱", hint: "使用邮箱验证码登录。", send: "发送验证码", code: "验证码", codeTitle: "输入验证码", check: "请检查收件箱，验证码 10 分钟内有效。", verify: "验证并登录", change: "更换邮箱", busy: "处理中…", invalid: "请输入 6 位数字验证码。", resend: "重新发送", intro: "查看照片、批注和编辑提案。", review: "共享评片" },
  en: { title: "Sign in to FocalRoom", email: "Email", hint: "Sign in with an email code.", send: "Send code", code: "Verification code", codeTitle: "Enter your code", check: "Check your inbox. The code expires in 10 minutes.", verify: "Verify and sign in", change: "Change email", busy: "Working…", invalid: "Enter a 6-digit code.", resend: "Resend code", intro: "View photos, comments and editing proposals.", review: "Shared review" },
  ja: { title: "FocalRoom にログイン", email: "メールアドレス", hint: "メールの確認コードでログインします。", send: "コードを送信", code: "確認コード", codeTitle: "確認コードを入力", check: "受信箱をご確認ください。コードは10分間有効です。", verify: "確認してログイン", change: "メールアドレスを変更", busy: "処理中…", invalid: "6桁の数字を入力してください。", resend: "コードを再送信", intro: "写真、コメント、編集提案を確認できます。", review: "共同レビュー" },
  ko: { title: "FocalRoom 로그인", email: "이메일", hint: "이메일 인증 코드로 로그인하세요.", send: "코드 보내기", code: "인증 코드", codeTitle: "인증 코드 입력", check: "받은편지함을 확인하세요. 코드는 10분간 유효합니다.", verify: "인증하고 로그인", change: "이메일 변경", busy: "처리 중…", invalid: "6자리 숫자를 입력하세요.", resend: "코드 다시 보내기", intro: "사진, 댓글과 편집 제안을 확인하세요.", review: "공동 리뷰" },
} as const;

export function MagicLogin({ onLogin }: { onLogin: (session: Session) => void }) {
  return <GoogleOAuthProvider clientId={GOOGLE_CLIENT_ID}>
    <MagicLoginInner onLogin={onLogin} />
  </GoogleOAuthProvider>;
}

function MagicLoginInner({ onLogin }: { onLogin: (session: Session) => void }) {
  const { locale, t } = useWebI18n();
  const text = copy[locale];
  const [recovering, setRecovering] = useState(false);
  const [googleLoading, setGoogleLoading] = useState(false);
  const gText = googleCopy[locale];
  const handleGoogleLogin = useGoogleLogin({
    onSuccess: async (tokenResponse: { access_token: string }) => {
      setGoogleLoading(true);
      try {
        const res = await fetch("https://www.googleapis.com/oauth2/v3/userinfo", {
          headers: { Authorization: `Bearer ${tokenResponse.access_token}` },
        });
        const googleUser = await res.json() as { email?: string; name?: string; picture?: string };
        const gEmail = googleUser.email ?? "";
        if (!gEmail) throw new Error("No email");
        const result = await request<{ user: { id: string; email: string; name: string }; csrf: string }>(
          "/auth/google", null, "POST", { email: gEmail, name: googleUser.name ?? "", avatar: googleUser.picture ?? "" });
        onLogin(sessionSchema.parse({ ...result.user, csrf: result.csrf }));
        const params = new URLSearchParams(window.location.search);
        if (params.get("desktop") === "1") {
          const stateEcho = params.get("state") ?? "";
          setTimeout(() => {
            window.location.href = "focalroom://callback?token=" + encodeURIComponent(result.csrf) + "&state=" + encodeURIComponent(stateEcho);
          }, 600);
        }
      } catch { setError(gText.fail); }
      finally { setGoogleLoading(false); }
    },
    onError: () => setError(gText.fail),
    flow: "implicit",
  });
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [step, setStep] = useState<"email" | "code">("email");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const perform = async (action: () => Promise<void>) => {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError("");
    try { await action(); } catch (cause) { setError(errorMessage(cause)); }
    finally { pending.current = false; setBusy(false); }
  };
  const send = () => perform(async () => {
    const normalized = email.trim().toLowerCase();
    await request("/auth/magic/request", null, "POST", { email: normalized });
    setEmail(normalized); setCode(""); setStep("code");
  });
  const verify = () => {
    if (!/^\d{6}$/.test(code)) { setError(text.invalid); return; }
    return perform(async () => {
      const result = await request<{ user: { id: string; email: string; name: string }; csrf: string }>(
        "/auth/magic/verify", null, "POST", { email, code });
      onLogin(sessionSchema.parse({ ...result.user, csrf: result.csrf }));

    });
  };
  if (recovering) return <AccountRecovery onBack={() => setRecovering(false)} />;
  return <main className="fr-welcome">
    <section><h1>{text.review}</h1><p>{text.intro}</p></section>
    <div className="fr-card fr-login">
      <h2>{step === "email" ? text.title : text.codeTitle}</h2>
      {step === "email" ? <form onSubmit={event => { event.preventDefault(); void send(); }}>
        <p>{text.hint}</p>
        <label>{text.email}<input type="email" autoComplete="email" maxLength={200} required disabled={busy} value={email} onChange={event => setEmail(event.target.value)} /></label>
        <button className="fr-primary" disabled={busy || !email.trim()}>{busy ? text.busy : text.send}</button>
      </form> : <form onSubmit={event => { event.preventDefault(); void verify(); }}>
        <p>{email}</p><p role="status">{text.check}</p>
        <label>{text.code}<input type="text" inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6}" maxLength={6} required disabled={busy} value={code} onChange={event => setCode(event.target.value)} /></label>
        <button className="fr-primary" disabled={busy || !/^\d{6}$/.test(code)}>{busy ? text.busy : text.verify}</button>
        <button type="button" className="fr-text" disabled={busy} onClick={() => { void send(); }}>{text.resend}</button>
        <button type="button" className="fr-text" disabled={busy} onClick={() => { setStep("email"); setCode(""); setError(""); }}>{text.change}</button>
      </form>}
      {error && <p role="alert" className="fr-error">{error}</p>}
      <button type="button" className="fr-text" disabled={busy} onClick={() => setRecovering(true)}>{t("web.forgotPassword")}</button>
    </div>
  
      <div style={{marginTop:12,textAlign:"center"}}>
        <button type="button" onClick={() => handleGoogleLogin()} disabled={googleLoading || busy}
          style={{display:"inline-flex",alignItems:"center",gap:8,padding:"10px 20px",borderRadius:8,border:"1px solid #4285f4",background:"#fff",color:"#3c4043",fontSize:14,fontWeight:500,cursor:"pointer",opacity:googleLoading?0.6:1}}>
          <svg width="18" height="18" viewBox="0 0 24 24"><path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/><path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/><path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/><path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/></svg>
          {gText.button}
        </button>
      </div>
    </main>;
}
