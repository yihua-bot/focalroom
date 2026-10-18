import {safeAvatarUrl} from "../../../packages/contracts/src/avatar.ts";
/** Google 登录：服务端验证 access_token（tokeninfo 校验 aud + email_verified），与验证码登录共用账户/工作区/会话域。 */
import { randomBytes, randomUUID } from "node:crypto";
import type { Pool, PoolClient } from "pg";
import { z } from "zod";
import { createAccount, createSession } from "./auth.ts";
import { digest, transaction } from "./database.ts";
import { ApiError, assert } from "./errors.ts";
import { hashPassword } from "./password.ts";
import { checkRateLimits } from "./rate-limit.ts";

const emailInput = z.string().trim().toLowerCase().pipe(z.email().max(200));
const tokenInput = z.strictObject({access_token:z.string().min(10).max(4096)});

export function googleClientId(): string {
  return process.env.FOCALROOM_GOOGLE_CLIENT_ID ?? "";
}

interface GoogleIdentity { email: string; name: string; picture?: string }

/** 用 Google tokeninfo 验证凭据：aud 必须匹配本应用 Client ID，且邮箱已验证。 */
async function verifyGoogleToken(accessToken: string, expectedAud: string): Promise<GoogleIdentity> {
  const info = await fetch(
    `https://oauth2.googleapis.com/tokeninfo?access_token=${encodeURIComponent(accessToken)}`,
    { signal: AbortSignal.timeout(8_000),redirect:"error" },
  ).then(r => { assert(r.ok, 502, "GOOGLE_UNAVAILABLE", "Google 身份服务暂不可用"); return r.json() as Promise<Record<string, string>>; })
    .catch(cause => { throw Object.assign(new ApiError(502,"GOOGLE_UNAVAILABLE","Google 身份服务暂不可用"),{cause}); });
  assert(info.aud === expectedAud, 401, "GOOGLE_AUD_MISMATCH", "Google 凭据不属于本应用");
  assert(String(info.email_verified) === "true", 401, "GOOGLE_EMAIL_UNVERIFIED", "Google 邮箱未验证");
  const email = emailInput.parse(info.email);
  // userinfo 补充资料；失败不阻塞登录（email 已由 tokeninfo 验证）。
  let name = email.split("@")[0], picture: string | undefined;
  try {
    const profile = await fetch("https://www.googleapis.com/oauth2/v3/userinfo", {
      headers: { Authorization: `Bearer ${accessToken}` }, signal: AbortSignal.timeout(8_000),redirect:"error",
    }).then(r => r.json() as Promise<{ name?: string; picture?: string }>);
    if (typeof profile.name === "string" && profile.name.trim()) name = profile.name.trim().slice(0,200);
    picture = safeAvatarUrl(profile.picture) ?? undefined;
  } catch { /* 资料缺失不影响身份。 */ }
  return { email, name, picture };
}

async function lockEmail(tx: PoolClient, email: string) {
  await tx.query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))", [`google:${email}`]);
}

export async function loginWithGoogle(
  pool: Pool, input: unknown, ip: string,
): Promise<{ token: string; csrf: string; user: { id: string; email: string; name: string }; is_new_user: boolean }> {
  const expectedAud = googleClientId();
  assert(expectedAud, 501, "GOOGLE_DISABLED", "Google 登录未启用");
  const {access_token:accessToken} = tokenInput.parse(input);
  const quota = await checkRateLimits(pool, [
    { key: `google-ip:${digest(ip)}`, limit: 30, windowMs: 600_000 },
  ]);
  assert(quota.allowed, 429, "RATE_LIMITED", "登录尝试过多，请稍后重试");
  const identity = await verifyGoogleToken(accessToken, expectedAud);
  return transaction(pool, async tx => {
    await lockEmail(tx, identity.email);
    let user = (await tx.query("SELECT id,email,display_name,avatar_url FROM fr_users WHERE email=$1 FOR UPDATE", [identity.email])).rows[0];
    const isNew = !user;
    if (!user) {
      user = { id: randomUUID(), email: identity.email, display_name: identity.name.slice(0, 200) };
      await createAccount(tx, user, await hashPassword(randomBytes(32).toString("hex")));
    }
    user = (await tx.query("UPDATE fr_users SET email_verified_at=now(),avatar_url=coalesce($2,avatar_url) WHERE id=$1 RETURNING id,email,display_name,avatar_url", [user.id, identity.picture ?? null])).rows[0];
    return { ...await createSession(tx, user), is_new_user: isNew };
  });
}
