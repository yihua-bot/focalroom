import { checkRateLimits } from "./rate-limit.ts";
import { randomBytes, randomUUID, timingSafeEqual } from "node:crypto";
import type { IncomingHttpHeaders } from "node:http";
import type { Pool, PoolClient } from "pg";
import { z } from "zod";
import { digest, transaction } from "./database.ts";
import { assert, ApiError } from "./errors.ts";
import { derivePassword, hashPassword, validPassword } from "./password.ts";
export type Identity = {
  id: string;
  email: string;
  name: string;
  avatar_url?: string | null;
  email_verified_at?: string | Date | null;
  session: string;
  csrf: string;
};
export type Headers = IncomingHttpHeaders;
const account = z.strictObject({
  email: z
    .email()
    .max(254)
    .transform((v) => v.trim().toLowerCase()),
  password: z.string().min(8).max(1024),
  display_name: z.string().trim().min(1).max(200).optional(),
});
type AccountUser = { id: string; email: string; display_name: string; avatar_url?: string | null };

// 两种登录方式共用账户初始化与会话格式，避免工作区或 Cookie 状态分叉。
export async function createAccount(tx: PoolClient, user: AccountUser, passwordHash: string) {
  await tx.query("INSERT INTO fr_users(id,email,display_name,password_hash) VALUES($1,$2,$3,$4)",
    [user.id, user.email, user.display_name, passwordHash]);
  const workspace = randomUUID();
  await tx.query("INSERT INTO fr_workspaces(id,title,owner_id) VALUES($1,$2,$3)",
    [workspace, `${user.display_name} 的工作区`, user.id]);
  await tx.query("INSERT INTO fr_memberships VALUES($1,$2,'OWNER')", [workspace, user.id]);
}
export async function createSession(db: Pick<Pool, "query">, user: AccountUser) {
  const token = randomBytes(32).toString("hex"), csrf = randomBytes(24).toString("hex");
  const inserted = await db.query(`WITH eligible AS (SELECT u.id FROM fr_users u WHERE u.id=$2
    AND NOT EXISTS(SELECT 1 FROM fr_account_closures c WHERE c.user_id=u.id) FOR SHARE OF u)
    INSERT INTO fr_sessions(digest,user_id,csrf,expires_at) SELECT $1,id,$3,now()+interval '24 hours' FROM eligible RETURNING digest`,
    [digest(token), user.id, csrf]);
  assert(inserted.rowCount, 401, 'AUTH_REVOKED', '账号已关闭');
  return { token, csrf, user: { id: user.id, email: user.email, name: user.display_name, avatar: user.avatar_url ?? null } };
}

// 候选密码与 scrypt-v1 哈希的恒时比较；格式不符按不匹配处理。
export async function passwordMatches(candidate: string, hash: string | null | undefined) {
  const parts: string[] = hash?.split("$") ?? [];
  const actual = await derivePassword(candidate, parts[1] ?? "focalroom-dummy-password");
  const expected = Buffer.from(parts[2] ?? "".padEnd(64, "0"), "hex");
  return parts[0] === "scrypt-v1" && actual.length === expected.length && timingSafeEqual(actual, expected);
}
export class AuthService {
  readonly pool: Pool;
  readonly origin: string;
  readonly secure: boolean;
  private running = 0;
  constructor(pool: Pool, origin: string) {
    this.pool = pool;
    this.origin = origin;
    this.secure = new URL(origin).protocol === "https:";
  }
  async account(input: unknown, registering: boolean, ip: string) {
    const parsed = account.safeParse(input);
    assert(
      parsed.success,
      400,
      "VALIDATION_ERROR",
      "请输入有效邮箱、8–1024 字节密码与姓名",
    );
    const value = parsed.data;
    assert(
      validPassword(value.password) &&
        (!registering || value.display_name),
      400,
      "VALIDATION_ERROR",
      "账户信息不完整",
    );
    // 集中限流（PostgreSQL 后端，多实例共享配额）。
    const { allowed } = await checkRateLimits(this.pool, [
      { key: `auth:ip:${digest(ip)}`, limit: 60 },
      { key: `auth:email:${digest(value.email)}`, limit: 10 },
    ]);
    assert(allowed, 429, "RATE_LIMITED", "登录尝试过多，请稍后重试");
    assert(this.running < 4, 429, "AUTH_BUSY", "认证繁忙，请稍后重试");
    this.running++;
    try {
      let user: { id: string; email: string; display_name: string };
      if (registering) {
        const passwordHash = await hashPassword(value.password);
        user = {
          id: randomUUID(),
          email: value.email,
          display_name: value.display_name!,
        };
        await transaction(this.pool, tx => createAccount(tx, user, passwordHash));
      } else {
        const row = (
          await this.pool.query(
            "SELECT id,email,display_name,avatar_url,password_hash FROM fr_users WHERE email=$1",
            [value.email],
          )
        ).rows[0];
        const parts: string[] = row?.password_hash.split("$") ?? [];
        const actual = await derivePassword(
          value.password,
          parts[1] ?? "focalroom-dummy-password",
        );
        const expected = Buffer.from(parts[2] ?? "".padEnd(64, "0"), "hex");
        assert(
          row &&
            parts[0] === "scrypt-v1" &&
            actual.length === expected.length &&
            timingSafeEqual(actual, expected),
          401,
          "UNAUTHORIZED",
          "邮箱或密码错误",
        );
        user = row;
      }
      return createSession(this.pool, user);
    } finally {
      this.running--;
    }
  }
  cookie(token: string, logout = false) {
    return `fr_session=${token}; Path=/api/v2; HttpOnly; SameSite=Strict; Max-Age=${logout ? 0 : 86400}${this.secure ? "; Secure" : ""}`;
  }
  async authenticate(
    headers: Headers,
    mutation: boolean,
    tx: Pick<PoolClient, "query"> = this.pool,
  ): Promise<Identity> {
    const bearer = headers.authorization?.match(/^Bearer ([a-f0-9]{64})$/)?.[1];
    const cookie = headers.cookie
      ?.split(";")
      .map((v) => v.trim())
      .find((v) => v.startsWith("fr_session="))
      ?.slice(11);
    const token = bearer ?? cookie;
    assert(
      token && /^[a-f0-9]{64}$/.test(token),
      401,
      "UNAUTHORIZED",
      "请先登录",
    );
    const row = (
      await tx.query(
        "SELECT s.digest,s.csrf,u.id,u.email,u.display_name,u.avatar_url,u.email_verified_at FROM fr_sessions s JOIN fr_users u ON u.id=s.user_id WHERE s.digest=$1 AND s.expires_at>now() AND NOT EXISTS(SELECT 1 FROM fr_account_closures c WHERE c.user_id=u.id) FOR SHARE OF s,u",
        [digest(token)],
      )
    ).rows[0];
    assert(row, 401, "AUTH_REVOKED", "会话已失效，请重新登录");
    if (!bearer && mutation)
      assert(
        headers.origin === this.origin &&
          headers["x-focalroom-csrf"] === row.csrf,
        403,
        "CSRF_REJECTED",
        "请求来源或会话校验失败",
      );
    return {
      id: row.id,
      email: row.email,
      name: row.display_name,
      avatar_url: row.avatar_url,
      email_verified_at: row.email_verified_at,
      session: row.digest,
      csrf: row.csrf,
    };
  }
  checkOrigin(headers: Headers) {
    if (headers.origin && headers.origin !== this.origin)
      throw new ApiError(403, "ORIGIN_REJECTED", "请求来源未获授权");
  }
}
