import { randomBytes } from "node:crypto";
import type { Pool, PoolClient } from "pg";
import { z } from "zod";
import { AuthService, type Headers } from "./auth.ts";
import { digest, transaction } from "./database.ts";
import { sendPasswordResetEmail, sendVerificationEmail } from "./email.ts";
import { ApiError, assert } from "./errors.ts";
import { hashPassword, validPassword } from "./password.ts";
import { checkRateLimits } from "./rate-limit.ts";

export interface AccountEmailDelivery {
  reset(email: string, token: string, origin: string): Promise<boolean>;
  verification(email: string, token: string, origin: string): Promise<boolean>;
}
const emailInput = z.string().trim().toLowerCase().pipe(z.email().max(254));
const tokenInput = z.string().regex(/^[a-f0-9]{64}$/);
const accepted = () => ({ sent: true }); // 仅表示请求受理，不确认账户存在或邮件已交付。

export class AccountRecoveryService {
  private resetting = 0;
  private readonly pool: Pool;
  private readonly auth: AuthService;
  private readonly origin: string;
  private readonly email: AccountEmailDelivery;
  constructor(
    pool: Pool,
    auth: AuthService,
    origin: string,
    email: AccountEmailDelivery = {
      reset: sendPasswordResetEmail,
      verification: sendVerificationEmail,
    },
  ) {
    this.pool = pool;
    this.auth = auth;
    this.origin = origin;
    this.email = email;
  }

  private async deliver(kind: "reset" | "verification", email: string, token: string) {
    let sent = false;
    try { sent = await this.email[kind](email, token, this.origin); } catch { /* 不记录令牌或第三方正文。 */ }
    assert(sent, 503, "EMAIL_UNAVAILABLE", "邮件服务暂不可用，请稍后重试");
  }

  // 不同令牌也锁同一用户，随后重新核验令牌；并发恢复只能成功一次。
  private async lockToken(tx: PoolClient, table: "fr_password_reset_tokens" | "fr_email_verify_tokens", token: string) {
    const id = digest(token);
    const user = (await tx.query(
      `SELECT u.id FROM fr_users u JOIN ${table} t ON t.user_id=u.id
       WHERE t.digest=$1 AND t.used_at IS NULL AND t.expires_at>now() FOR UPDATE OF u`, [id],
    )).rows[0];
    const row = user && (await tx.query(
      `SELECT user_id FROM ${table} WHERE digest=$1 AND used_at IS NULL AND expires_at>now() FOR UPDATE`, [id],
    )).rows[0];
    assert(row, 400, "VALIDATION_ERROR", "令牌无效或已过期");
    return row.user_id as string;
  }

  async verifyEmail(input: unknown) {
    const { token } = z.strictObject({ token: tokenInput }).parse(input);
    return transaction(this.pool, async tx => {
      const userId = await this.lockToken(tx, "fr_email_verify_tokens", token);
      await tx.query("UPDATE fr_users SET email_verified_at=now() WHERE id=$1", [userId]);
      await tx.query("UPDATE fr_email_verify_tokens SET used_at=now() WHERE user_id=$1 AND used_at IS NULL", [userId]);
      return { verified: true };
    });
  }

  async resendVerification(headers: Headers) {
    const user = await this.auth.authenticate(headers, true);
    const verified = (await this.pool.query("SELECT email_verified_at FROM fr_users WHERE id=$1", [user.id])).rows[0];
    if (verified?.email_verified_at) return { already_verified: true };
    const { allowed } = await checkRateLimits(this.pool, [{ key: `verify:${user.id}`, limit: 3, windowMs: 600_000 }]);
    assert(allowed, 429, "RATE_LIMITED", "重发过于频繁，请 10 分钟后再试");
    return transaction(this.pool, async tx => {
      // 与验证操作共用用户锁，验证成功后不再寄出新令牌。
      const row = (await tx.query("SELECT email_verified_at FROM fr_users WHERE id=$1 FOR UPDATE", [user.id])).rows[0];
      if (row?.email_verified_at) return { already_verified: true };
      const token = randomBytes(32).toString("hex");
      await tx.query("INSERT INTO fr_email_verify_tokens(digest,user_id,expires_at) VALUES($1,$2,now()+interval '48 hours')", [digest(token), user.id]);
      await this.deliver("verification", user.email, token);
      return accepted();
    });
  }

  async forgotPassword(input: unknown, ip: string) {
    const { allowed } = await checkRateLimits(this.pool, [{ key: `pwd-reset-ip:${digest(ip)}`, limit: 60, windowMs: 3_600_000 }]);
    assert(allowed, 429, "RATE_LIMITED", "重置请求过于频繁，请稍后再试");
    const parsed = emailInput.safeParse((input as Record<string, unknown>)?.email);
    if (!parsed.success) return accepted();
    const email = parsed.data;
    // 对存在与不存在的邮箱先消费相同配额；邮件配额耗尽仍返回相同受理响应。
    const quota = await checkRateLimits(this.pool, [{ key: `pwd-reset:${digest(email)}`, limit: 3, windowMs: 3_600_000 }]);
    if (!quota.allowed) return accepted();
    try {
      return await transaction(this.pool, async tx => {
        const user = (await tx.query("SELECT id FROM fr_users WHERE email=$1 FOR UPDATE", [email])).rows[0];
        if (!user) return accepted();
        const token = randomBytes(32).toString("hex");
        await tx.query("INSERT INTO fr_password_reset_tokens(digest,user_id,expires_at,ip) VALUES($1,$2,now()+interval '30 minutes',$3)", [digest(token), user.id, ip]);
        await this.deliver("reset", email, token);
        return accepted();
      });
    } catch (error) {
      // 邮件失败回滚令牌，但 HTTP 响应不泄露已注册邮箱。
      if (error instanceof ApiError && error.code === "EMAIL_UNAVAILABLE") return accepted();
      throw error;
    }
  }

  async resetPassword(input: unknown) {
    const { token, password } = z.strictObject({ token: tokenInput, password: z.string().refine(validPassword) }).parse(input);
    assert(this.resetting < 4, 429, "AUTH_BUSY", "认证繁忙，请稍后重试");
    this.resetting++;
    try {
      return await transaction(this.pool, async tx => {
        const userId = await this.lockToken(tx, "fr_password_reset_tokens", token);
        const passwordHash = await hashPassword(password);
        await tx.query("UPDATE fr_users SET password_hash=$1 WHERE id=$2", [passwordHash, userId]);
        await tx.query("UPDATE fr_password_reset_tokens SET used_at=now() WHERE user_id=$1 AND used_at IS NULL", [userId]);
        await tx.query("UPDATE fr_sessions SET expires_at=now() WHERE user_id=$1", [userId]);
        return { reset: true };
      });
    } finally { this.resetting--; }
  }
}
