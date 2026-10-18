/** 邮箱验证码与既有账户、工作区和 Cookie 会话共用同一认证域。 */
import { randomBytes, randomInt, randomUUID, timingSafeEqual } from "node:crypto";
import type { Pool, PoolClient } from "pg";
import { z } from "zod";
import { createAccount, createSession } from "./auth.ts";
import { digest, transaction } from "./database.ts";
import { sendMagicCodeEmail } from "./email.ts";
import { assert } from "./errors.ts";
import { hashPassword } from "./password.ts";
import { checkRateLimits } from "./rate-limit.ts";

export type MagicEmailDelivery = (email: string, code: string, origin: string) => Promise<boolean>;
const emailInput = z.string().trim().toLowerCase().pipe(z.email().max(200));
const codeHash = (id: string, code: string) => digest(`magic-v1:${id}:${code}`);
async function lockEmail(tx: PoolClient, email: string) {
  await tx.query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))", [`magic:${email}`]);
}

export async function requestMagicCode(
  pool: Pool, input: unknown, ip: string, origin: string,
  deliver: MagicEmailDelivery = sendMagicCodeEmail,
): Promise<{ sent: true }> {
  const email = emailInput.parse(input);
  const quota = await checkRateLimits(pool, [
    { key: `magic-send-ip:${digest(ip)}`, limit: 60, windowMs: 3_600_000 },
    { key: `magic-send-email:${digest(email)}`, limit: 3, windowMs: 600_000 },
  ]);
  assert(quota.allowed, 429, "RATE_LIMITED", "验证码请求过于频繁，请稍后重试");
  return transaction(pool, async tx => {
    await lockEmail(tx, email);
    const code = String(randomInt(1_000_000)).padStart(6, "0");
    const id = randomBytes(32).toString("hex");
    await tx.query("UPDATE fr_magic_login_codes SET used_at=now() WHERE email=$1 AND used_at IS NULL", [email]);
    await tx.query("INSERT INTO fr_magic_login_codes(digest,email,code_hash,expires_at,ip) VALUES($1,$2,$3,now()+interval '10 minutes',$4)",
      [id, email, codeHash(id, code), ip]);
    let sent = false;
    try { sent = await deliver(email, code, origin); } catch { /* 不记录验证码、密钥或邮件服务正文。 */ }
    assert(sent, 503, "EMAIL_UNAVAILABLE", "邮件服务暂不可用，请稍后重试");
    return { sent: true };
  });
}

export async function verifyMagicCode(pool: Pool, inputEmail: unknown, inputCode: unknown, ip: string) {
  const email = emailInput.parse(inputEmail);
  const code = z.string().regex(/^\d{6}$/).parse(inputCode);
  const quota = await checkRateLimits(pool, [
    { key: `magic-verify-ip:${digest(ip)}`, limit: 60 },
    { key: `magic-verify-email:${digest(email)}`, limit: 10 },
  ]);
  assert(quota.allowed, 429, "RATE_LIMITED", "验证尝试过多，请稍后重试");
  return transaction(pool, async tx => {
    await lockEmail(tx, email);
    const row = (await tx.query(
      "SELECT digest,code_hash,attempts FROM fr_magic_login_codes WHERE email=$1 AND used_at IS NULL AND expires_at>now() ORDER BY created_at DESC,digest DESC LIMIT 1 FOR UPDATE", [email],
    )).rows[0];
    if (!row || row.attempts >= 5) return null;
    const expected = Buffer.from(row.code_hash, "hex");
    const actual = Buffer.from(codeHash(row.digest, code), "hex");
    if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) {
      // 失败次数必须提交；在事务外返回统一错误，不能用抛错把计数回滚。
      await tx.query("UPDATE fr_magic_login_codes SET attempts=attempts+1,used_at=CASE WHEN attempts+1>=5 THEN now() ELSE used_at END WHERE digest=$1", [row.digest]);
      return null;
    }
    await tx.query("UPDATE fr_magic_login_codes SET used_at=now() WHERE email=$1 AND used_at IS NULL", [email]);
    let user = (await tx.query("SELECT id,email,display_name FROM fr_users WHERE email=$1 FOR UPDATE", [email])).rows[0];
    const isNew = !user;
    if (!user) {
      user = { id: randomUUID(), email, display_name: email.split("@")[0] };
      await createAccount(tx, user, await hashPassword(randomBytes(32).toString("hex")));
    }
    await tx.query("UPDATE fr_users SET email_verified_at=now() WHERE id=$1", [user.id]);
    return { ...await createSession(tx, user), is_new_user: isNew };
  });
}
