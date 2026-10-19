import { useEffect, useRef, useState, type FormEvent } from "react";
import { useWebI18n } from "../i18n";
import { CloudError, request, type Session } from "../shared-api";
import { accountCopy } from "./account-copy";

export type AccountLink = { mode: "reset" | "verify"; token: string };
export function readAccountLink(url: URL): AccountLink | null {
  const hash = new URLSearchParams(url.hash.slice(1));
  if (hash.has("reset-password")) return { mode: "reset", token: hash.get("reset-password") ?? "" };
  if (hash.has("verify-email")) return { mode: "verify", token: hash.get("verify-email") ?? "" };
  // 兼容已寄出的旧版路径；新邮件仅使用 /shared 的 fragment。
  if (url.pathname === "/reset-password" || url.pathname === "/verify-email")
    return { mode: url.pathname === "/reset-password" ? "reset" : "verify", token: url.searchParams.get("token") ?? "" };
  return null;
}

type MessageKey = keyof typeof accountCopy.zh;
function useAccountMessage() {
  const { locale } = useWebI18n();
  const copy = accountCopy[locale];
  return { copy, errorKey: (error: unknown): MessageKey => {
    if (!(error instanceof CloudError)) return "failed";
    if (error.code === "EMAIL_UNAVAILABLE") return "unavailable";
    if (error.code === "RATE_LIMITED" || error.code === "AUTH_BUSY") return "limited";
    if (error.code === "VALIDATION_ERROR") return "invalid";
    if (error.code === "UNAUTHORIZED" || error.code === "AUTH_REVOKED") return "signIn";
    return "failed";
  } };
}

export function AccountRecovery({ link, onBack }: { link?: AccountLink; onBack: (resetCompleted: boolean) => void }) {
  const { copy, errorKey } = useAccountMessage();
  const [busy, setBusy] = useState(false), [done, setDone] = useState(false);
  const [error, setError] = useState<MessageKey | "">("");
  const inFlight = useRef(false);
  const mode = link?.mode ?? "forgot";
  const valid = !link || /^[a-f0-9]{64}$/.test(link.token);
  useEffect(() => {
    if (!link) return;
    const url = new URL(location.href);
    const hash = new URLSearchParams(url.hash.slice(1));
    hash.delete("reset-password"); hash.delete("verify-email");
    url.searchParams.delete("token");
    // 令牌只保留在当前组件内存，不放表单字段、存储或后续 URL。
    history.replaceState(null, "", `/shared${url.search}${hash.size ? `#${hash}` : ""}`);
  }, [link]);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (inFlight.current || done || !valid) return;
    const form = event.currentTarget;
    const data = new FormData(form);
    const password = String(data.get("password") ?? "");
    if (mode === "reset") {
      if (password.length < 8 || new TextEncoder().encode(password).length > 1024) { setError("passwordInvalid"); return; }
      if (password !== data.get("confirm")) { setError("mismatch"); return; }
    }
    inFlight.current = true; setBusy(true); setError("");
    try {
      await request(mode === "forgot" ? "/auth/forgot-password" : mode === "reset" ? "/auth/reset-password" : "/auth/verify-email", null, "POST",
        mode === "forgot" ? { email: data.get("email") } : mode === "reset" ? { token: link?.token, password } : { token: link?.token });
      form.reset(); setDone(true);
    } catch (error) { setError(errorKey(error)); }
    finally { inFlight.current = false; setBusy(false); }
  };
  return <main className="fr-account-access"><form className="fr-card fr-login" onSubmit={event => void submit(event)} aria-busy={busy}>
    <p className="fr-eyebrow">FOCALROOM / ACCOUNT</p><h1>{copy[mode]}</h1>
    <p>{mode === "forgot" ? copy.hint : mode === "reset" ? copy.resetHint : copy.verifyHint}</p>
    {!valid && <p className="fr-error" role="alert">{copy.invalid}</p>}
    {done ? <p role="status">{mode === "forgot" ? copy.accepted : mode === "reset" ? copy.resetDone : copy.verified}</p> : valid && <>
      {mode === "forgot" && <label>{copy.email}<input name="email" type="email" autoComplete="email" required maxLength={254} disabled={busy} /></label>}
      {mode === "reset" && <>
        <label>{copy.password}<input name="password" type="password" autoComplete="new-password" minLength={8} maxLength={1024} required disabled={busy} /></label>
        <label>{copy.confirm}<input name="confirm" type="password" autoComplete="new-password" minLength={8} maxLength={1024} required disabled={busy} /></label>
      </>}
      {error && <p className="fr-error" role="alert">{copy[error]}</p>}
      <button className="fr-primary" disabled={busy}>{busy ? copy.busy : mode === "forgot" ? copy.send : mode === "reset" ? copy.save : copy.verifyAction}</button>
    </>}
    <button type="button" className="fr-text" onClick={() => onBack(mode === "reset" && done)} disabled={busy}>{copy.back}</button>
  </form></main>;
}

export function VerificationRequest({ session }: { session: Session }) {
  const { copy, errorKey } = useAccountMessage();
  const [busy, setBusy] = useState(false), [notice, setNotice] = useState<MessageKey | "">("");
  const inFlight = useRef(false);
  const send = async () => {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true); setNotice("");
    try {
      const result = await request<{ already_verified?: boolean }>("/auth/resend-verification", session, "POST", {});
      setNotice(result.already_verified ? "alreadyVerified" : "sent");
    } catch (error) { setNotice(errorKey(error)); }
    finally { inFlight.current = false; setBusy(false); }
  };
  if (session.emailVerified) return null;
  return <div className="fr-account-verification"><button type="button" disabled={busy} onClick={() => void send()}>{busy ? copy.busy : copy.resend}</button>{notice && <span role="status">{copy[notice]}</span>}</div>;
}
