import { randomBytes, createHash, timingSafeEqual } from "node:crypto";
import type { Pool } from "pg";
import { z } from "zod";
import { AuthService, createSession, type Headers } from "./auth.ts";
import { digest, transaction } from "./database.ts";
import { assert } from "./errors.ts";
import { checkRateLimits } from "./rate-limit.ts";

const hex = z.string().regex(/^[a-f0-9]{64}$/);
const beginInput = z.strictObject({ state: hex, code_challenge: z.string().regex(/^[A-Za-z0-9_-]{43}$/) });
const authorizeInput = z.strictObject({ request_id: hex, state: hex });
const exchangeInput = authorizeInput.extend({ code: hex, code_verifier: z.string().regex(/^[A-Za-z0-9._~-]{43,128}$/) });
const equal = (a: string, b: string) => a.length === b.length && timingSafeEqual(Buffer.from(a), Buffer.from(b));

export class DesktopLoginService {
  constructor(private readonly pool: Pool, private readonly auth: AuthService) {}
  private async limit(ip: string, suffix: string) {
    const { allowed } = await checkRateLimits(this.pool, [{ key: `desktop:${suffix}:ip:${digest(ip)}`, limit: 30 }]);
    assert(allowed, 429, "RATE_LIMITED", "登录请求过多，请稍后重试");
  }
  async begin(input: unknown, ip: string) {
    const value = beginInput.parse(input);
    await this.limit(ip, "start");
    const id = randomBytes(32).toString("hex");
    await this.pool.query("INSERT INTO fr_desktop_logins(id,state_hash,challenge,expires_at) VALUES($1,$2,$3,now()+interval '15 minutes')",
      [id, digest(value.state), value.code_challenge]);
    const url = new URL("/shared", this.auth.origin);
    url.search = new URLSearchParams({ desktop: "1", request: id, state: value.state }).toString();
    return { request_id: id, login_url: url.href };
  }
  async authorize(headers: Headers, input: unknown) {
    const value = authorizeInput.parse(input);
    return transaction(this.pool, async tx => {
      const user = await this.auth.authenticate(headers, true, tx);
      const row = (await tx.query("SELECT * FROM fr_desktop_logins WHERE id=$1 AND expires_at>now() AND used_at IS NULL FOR UPDATE", [value.request_id])).rows[0];
      assert(row && equal(row.state_hash, digest(value.state)), 400, "DESKTOP_LOGIN_INVALID", "桌面登录已过期，请重新发起");
      assert(!row.browser_session || row.browser_session === user.session, 409, "DESKTOP_ACCOUNT_CHANGED", "登录账户已变化，请重新发起");
      const code = randomBytes(32).toString("hex");
      await tx.query("UPDATE fr_desktop_logins SET code_hash=$2,code_expires_at=now()+interval '60 seconds',browser_session=$3 WHERE id=$1", [value.request_id, digest(code), user.session]);
      return { code };
    });
  }
  async exchange(input: unknown, ip: string) {
    const value = exchangeInput.parse(input);
    await this.limit(ip, "exchange");
    return transaction(this.pool, async tx => {
      const row = (await tx.query("SELECT * FROM fr_desktop_logins WHERE id=$1 AND expires_at>now() AND code_expires_at>now() AND used_at IS NULL FOR UPDATE", [value.request_id])).rows[0];
      const challenge = createHash("sha256").update(value.code_verifier).digest("base64url");
      assert(row && equal(row.state_hash, digest(value.state)) && equal(row.code_hash ?? "", digest(value.code)) && equal(row.challenge, challenge),
        400, "DESKTOP_LOGIN_INVALID", "登录回传无效或已过期，请重新发起");
      // 浏览器退出、密码重置或会话过期后，不再颁发新的桌面会话。
      const user = (await tx.query("SELECT u.id,u.email,u.display_name FROM fr_sessions s JOIN fr_users u ON u.id=s.user_id WHERE s.digest=$1 AND s.expires_at>now() FOR SHARE OF s", [row.browser_session])).rows[0];
      assert(user, 401, "AUTH_REVOKED", "网页会话已失效，请重新登录");
      await tx.query("UPDATE fr_desktop_logins SET used_at=now(),code_hash=NULL WHERE id=$1", [value.request_id]);
      return createSession(tx, user);
    });
  }
}
