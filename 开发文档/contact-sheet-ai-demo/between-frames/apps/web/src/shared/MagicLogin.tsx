import { useRef, useState } from "react";
import { useWebI18n } from "../i18n";
import { request, sessionSchema, type Session, errorMessage } from "../shared-api";
import { AccountRecovery } from "./AccountRecovery";

const copy = {
  zh: { title: "登录 FocalRoom", email: "邮箱", hint: "使用邮箱验证码登录。", send: "发送验证码", code: "验证码", codeTitle: "输入验证码", check: "请检查收件箱，验证码 10 分钟内有效。", verify: "验证并登录", change: "更换邮箱", busy: "处理中…", invalid: "请输入 6 位数字验证码。", resend: "重新发送", intro: "查看照片、批注和编辑提案。", review: "共享评片" },
  en: { title: "Sign in to FocalRoom", email: "Email", hint: "Sign in with an email code.", send: "Send code", code: "Verification code", codeTitle: "Enter your code", check: "Check your inbox. The code expires in 10 minutes.", verify: "Verify and sign in", change: "Change email", busy: "Working…", invalid: "Enter a 6-digit code.", resend: "Resend code", intro: "View photos, comments and editing proposals.", review: "Shared review" },
  ja: { title: "FocalRoom にログイン", email: "メールアドレス", hint: "メールの確認コードでログインします。", send: "コードを送信", code: "確認コード", codeTitle: "確認コードを入力", check: "受信箱をご確認ください。コードは10分間有効です。", verify: "確認してログイン", change: "メールアドレスを変更", busy: "処理中…", invalid: "6桁の数字を入力してください。", resend: "コードを再送信", intro: "写真、コメント、編集提案を確認できます。", review: "共同レビュー" },
  ko: { title: "FocalRoom 로그인", email: "이메일", hint: "이메일 인증 코드로 로그인하세요.", send: "코드 보내기", code: "인증 코드", codeTitle: "인증 코드 입력", check: "받은편지함을 확인하세요. 코드는 10분간 유효합니다.", verify: "인증하고 로그인", change: "이메일 변경", busy: "처리 중…", invalid: "6자리 숫자를 입력하세요.", resend: "코드 다시 보내기", intro: "사진, 댓글과 편집 제안을 확인하세요.", review: "공동 리뷰" },
} as const;

export function MagicLogin({ onLogin }: { onLogin: (session: Session) => void }) {
  const { locale, t } = useWebI18n();
  const text = copy[locale];
  const [recovering, setRecovering] = useState(false);
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
      // 桌面端借道登录：URL 带 desktop=1 时，登录成功后经 focalroom:// 深链回传
      const params = new URLSearchParams(window.location.search);
      if (params.get('desktop') === '1') {
        const stateEcho = params.get('state') ?? '';
        const banner = document.createElement('div');
        banner.style.cssText = 'position:fixed;inset:0;z-index:9999;display:flex;align-items:center;justify-content:center;background:rgba(10,10,15,.96);font-size:15px;color:#e8e8f0;text-align:center;padding:24px;';
        banner.textContent = '登录成功，正在返回 FocalRoom 桌面端…';
        document.body.appendChild(banner);
        setTimeout(() => {
          window.location.href = 'focalroom://callback?token=' + encodeURIComponent(result.csrf) + '&state=' + encodeURIComponent(stateEcho);
        }, 600);
      }
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
  </main>;
}
