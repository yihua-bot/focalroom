/** PostgreSQL 原子固定窗口配额；多实例使用数据库时钟，不依赖各进程时钟。 */
import type { Pool } from "pg";

type Queryable = Pick<Pool, "query">;
export interface RateLimitResult {
  allowed: boolean;
  remaining: number;
  resetAt: Date;
}

export async function checkRateLimit(
  pool: Queryable,
  key: string,
  limit: number,
  windowMs = 60_000,
): Promise<RateLimitResult> {
  if (!key || !Number.isSafeInteger(limit) || limit < 1 || limit > 1_000_000 ||
      !Number.isSafeInteger(windowMs) || windowMs < 1000 || windowMs > 86_400_000)
    throw new Error("无效限流策略");
  const result = await pool.query(
    `INSERT INTO fr_rate_limits (key, window_start, count)
     VALUES ($1, to_timestamp(floor(extract(epoch FROM clock_timestamp()) * 1000 / $2) * $2 / 1000), 1)
     ON CONFLICT (key, window_start)
     DO UPDATE SET count = LEAST(fr_rate_limits.count + 1, $3 + 1)
     RETURNING count, window_start + ($2 * interval '1 millisecond') AS reset_at`,
    // 窗口长度隔离配额，部署调整策略时不混用其他窗口。
    [`${windowMs}:${key}`, windowMs, limit],
  );
  const { count, reset_at: resetAt } = result.rows[0];
  // 最长窗口为一天；不能在五分钟后清掉仍有效的小时配额。
  if (Math.random() < 0.01)
    await pool.query("DELETE FROM fr_rate_limits WHERE window_start < now() - interval '2 days'").catch(() => {});
  return { allowed: count <= limit, remaining: Math.max(0, limit - count), resetAt };
}

/** 各配额计数分别原子递增；拒绝尝试同样计数，不宣称整个批次是单一事务。 */
export async function checkRateLimits(
  pool: Queryable,
  checks: Array<{ key: string; limit: number; windowMs?: number }>,
): Promise<{ allowed: boolean; violations: string[] }> {
  const results = await Promise.all(checks.map(c => checkRateLimit(pool, c.key, c.limit, c.windowMs)));
  const violations = checks.filter((_, i) => !results[i].allowed).map(c => c.key);
  return { allowed: violations.length === 0, violations };
}
