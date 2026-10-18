/**
 * M4 验证码登录：无密码——邮箱收 6 位验证码，输码即登录。
 * 10 分钟有效，一次性，每邮箱每 10 分钟最多 3 次请求。
 * 自动注册：未注册邮箱首次验证后自动创建账户。
 */
import { randomBytes, createHash } from "node:crypto";
import type { Pool } from "pg";
import { sendMagicCodeEmail } from "./email.ts";

export interface MagicLoginResult {
  token: string;
  user_id: string;
  display_name: string;
  is_new_user: boolean;
}

function hashCode(code: string): string {
  return createHash("sha256").update(code).digest("hex");
}

export function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email) && email.length <= 200;
}

/** 第一步：请求验证码。 */
export async function requestMagicCode(
  pool: Pool,
  email: string,
  ip: string,
  webOrigin: string,
): Promise<{ sent: boolean }> {
  if (!isValidEmail(email)) {
    return { sent: true }; // 不暴露邮箱是否有效
  }
  // 限流：每邮箱每 10 分钟 3 次。
  const recent = (
    await pool.query(
      "SELECT COUNT(*) FROM fr_magic_login_codes WHERE email=$1 AND created_at>now()-interval '10 minutes'",
      [email],
    )
  ).rows[0].count;
  if (Number(recent) >= 3) {
    return { sent: true }; // 静默拒绝
  }

  // 生成 6 位数字验证码。
  const code = String(Math.floor(100000 + Math.random() * 900000));
  const codeHash = hashCode(code);
  const raw = randomBytes(32).toString("hex"); // 请求标识

  await pool.query(
    "INSERT INTO fr_magic_login_codes(digest,email,code_hash,expires_at,ip) VALUES($1,$2,$3,now()+interval '10 minutes',$4)",
    [raw, email, codeHash, ip],
  );

  // 发送验证码邮件。
  const sent = await sendMagicCodeEmail(email, code, webOrigin);
  if (!sent) {
    console.warn("[magic-login] 邮件未发送（Key 未配置或网络错误）");
  }
  return { sent: true };
}

/** 第二步：验证码登录（自动注册未注册邮箱）。 */
export async function verifyMagicCode(
  pool: Pool,
  email: string,
  code: string,
): Promise<MagicLoginResult | null> {
  if (!isValidEmail(email) || !/^\d{6}$/.test(code)) return null;

  const codeHash = hashCode(code);
  const row = (
    await pool.query(
      "SELECT digest FROM fr_magic_login_codes WHERE email=$1 AND code_hash=$2 AND used_at IS NULL AND expires_at>now() FOR UPDATE",
      [email, codeHash],
    )
  ).rows[0];
  if (!row) return null;

  // 标记已使用。
  await pool.query("UPDATE fr_magic_login_codes SET used_at=now() WHERE digest=$1", [row.digest]);

  // 查找或创建用户。
  let user = (
    await pool.query("SELECT id, display_name FROM fr_users WHERE email=$1", [email])
  ).rows[0];

  let isNew = false;
  if (!user) {
    // 自动注册。
    const userId = crypto.randomUUID();
    const name = email.split("@")[0];
    await pool.query(
      "INSERT INTO fr_users(id,email,display_name,password_hash) VALUES($1,$2,$3,'')",
      [userId, email, name],
    );
    user = { id: userId, display_name: name };
    isNew = true;
  }

  // 创建会话。
  const token = randomBytes(32).toString("hex");
  await pool.query(
    "INSERT INTO sessions(token,user_id) VALUES($1,$2)",
    [createHash("sha256").update(token).digest("hex"), user.id],
  );

  return {
    token,
    user_id: user.id,
    display_name: user.display_name,
    is_new_user: isNew,
  };
}
