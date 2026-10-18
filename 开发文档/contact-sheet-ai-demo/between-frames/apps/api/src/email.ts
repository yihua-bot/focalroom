/**
 * M4 邮箱服务：密码找回邮件（经 Resend API）。
 * Key 经 FOCALROOM_RESEND_API_KEY 环境变量注入，不落库。
 */
export async function sendPasswordResetEmail(
  toEmail: string,
  resetToken: string,
  webOrigin: string,
): Promise<boolean> {
  const apiKey = process.env.FOCALROOM_RESEND_API_KEY ?? "";
  if (!apiKey) {
    console.warn("[email] FOCALROOM_RESEND_API_KEY 未配置，邮件不发送");
    return false;
  }
  const resetUrl = accountEmailUrl(webOrigin, "reset-password", resetToken);
  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from: process.env.FOCALROOM_EMAIL_FROM ?? "FocalRoom <noreply@focalroom.app>",
      to: toEmail,
      subject: "FocalRoom 密码重置",
      html: `
        <div style="font-family:system-ui;max-width:480px;margin:0 auto;padding:2rem">
          <h2 style="color:#1a1a2e">FocalRoom 密码重置</h2>
          <p>我们收到了重置密码的请求。点击以下链接重置：</p>
          <p><a href="${resetUrl}" style="display:inline-block;background:#5865F2;color:#fff;padding:.75rem 1.5rem;border-radius:6px;text-decoration:none">重置密码</a></p>
          <p style="color:#666;font-size:.85rem">链接 30 分钟内有效。如果这不是你的操作，请忽略此邮件。</p>
        </div>
      `,
    }),
    signal: AbortSignal.timeout(10_000),
  });
  return response.ok;
}

export async function sendVerificationEmail(
  toEmail: string,
  verifyToken: string,
  webOrigin: string,
): Promise<boolean> {
  const apiKey = process.env.FOCALROOM_RESEND_API_KEY ?? "";
  if (!apiKey) {
    console.warn("[email] FOCALROOM_RESEND_API_KEY 未配置，验证邮件不发送");
    return false;
  }
  const verifyUrl = accountEmailUrl(webOrigin, "verify-email", verifyToken);
  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from: process.env.FOCALROOM_EMAIL_FROM ?? "FocalRoom <noreply@focalroom.app>",
      to: toEmail,
      subject: "验证你的 FocalRoom 账号",
      html: `
        <div style="font-family:system-ui;max-width:480px;margin:0 auto;padding:2rem">
          <h2 style="color:#1a1a2e">验证你的 FocalRoom 账号</h2>
          <p>欢迎！点击以下链接完成邮箱验证：</p>
          <p><a href="${verifyUrl}" style="display:inline-block;background:#5865F2;color:#fff;padding:.75rem 1.5rem;border-radius:6px;text-decoration:none">验证邮箱</a></p>
          <p style="color:#666;font-size:.85rem">链接 48 小时内有效。如果这不是你的操作，请忽略此邮件。</p>
        </div>
      `,
    }),
    signal: AbortSignal.timeout(10_000),
  });
  return response.ok;
}

export function accountEmailUrl(origin: string, kind: "reset-password" | "verify-email", token: string) {
  const url = new URL("/shared", origin);
  if (!/^[a-f0-9]{64}$/.test(token) || url.username || url.password ||
      (url.protocol !== "https:" && !(url.protocol === "http:" && ["127.0.0.1", "localhost", "[::1]"].includes(url.hostname))))
    throw new Error("账户邮件需要可信 HTTPS 或本地回环地址");
  url.hash = new URLSearchParams({ [kind]: token }).toString();
  return url.href;
}

export async function sendMagicCodeEmail(
  toEmail: string,
  code: string,
  _webOrigin: string,
): Promise<boolean> {
  const apiKey = process.env.FOCALROOM_RESEND_API_KEY ?? "";
  if (!apiKey) {
    console.warn("[email] FOCALROOM_RESEND_API_KEY 未配置，验证码邮件不发送");
    return false;
  }
  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from: process.env.FOCALROOM_EMAIL_FROM ?? "FocalRoom <noreply@focalroom.app>",
      to: toEmail,
      subject: `FocalRoom 验证码: ${code}`,
      html: `
        <div style="font-family:system-ui;max-width:480px;margin:0 auto;padding:2rem;text-align:center">
          <h2 style="color:#1a1a2e">FocalRoom 登录验证码</h2>
          <div style="font-size:2.5rem;font-weight:700;letter-spacing:.3em;color:#5865F2;margin:1.5rem 0">${code}</div>
          <p style="color:#666;font-size:.85rem">验证码 10 分钟内有效。如果这不是你的操作，请忽略此邮件。</p>
        </div>
      `,
    }),
    signal: AbortSignal.timeout(10_000),
  });
  return response.ok;
}
