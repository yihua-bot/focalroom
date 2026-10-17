/**
 * M4 集中限流：PostgreSQL 后端原子窗口计数（替代进程内存，多实例部署共享配额）。
 * 每次 checkRateLimit 原子递增并返回是否在配额内；过期窗口惰性清理。
 */
import type { Pool } from "pg";

export interface RateLimitResult {
  allowed: boolean;
  remaining: number;
  resetAt: Date;
}

/**
 * 原子限流检查：key 为唯一标识（如 digest(ip) 或 digest(email)）。
 * limitPerMinute 为每分钟最大次数；window 自动按分钟对齐。
 */
export async function checkRateLimit(
  pool: Pool,
  key: string,
  limitPerMinute: number,
): Promise<RateLimitResult> {
  const now = new Date();
  // 按分钟对齐窗口。
  const windowStart = new Date(Math.floor(now.getTime() / 60_000) * 60_000);
  const resetAt = new Date(windowStart.getTime() + 60_000);

  // 原子 upsert：不存在则插入 count=1，存在则 count+1。
  const result = await pool.query(
    `INSERT INTO fr_rate_limits (key, window_start, count)
     VALUES ($1, $2, 1)
     ON CONFLICT (key, window_start)
     DO UPDATE SET count = fr_rate_limits.count + 1
     RETURNING count`,
    [key, windowStart],
  );

  const count = result.rows[0]?.count ?? 1;
  const allowed = count <= limitPerMinute;

  // 惰性清理 5 分钟前的过期行（每次调用 ~1% 概率触发）。
  if (Math.random() < 0.01) {
    await pool.query(
      "DELETE FROM fr_rate_limits WHERE window_start < now() - interval '5 minutes'"
    ).catch(() => {});
  }

  return { allowed, remaining: Math.max(0, limitPerMinute - count), resetAt };
}

/**
 * 批量限流检查（原子）：全部通过才返回 allowed=true。
 */
export async function checkRateLimits(
  pool: Pool,
  checks: Array<{ key: string; limit: number }>,
): Promise<{ allowed: boolean; violations: string[] }> {
  const results = await Promise.all(
    checks.map(c => checkRateLimit(pool, c.key, c.limit))
  );
  const violations = checks
    .map((c, i) => ({ key: c.key, allowed: results[i].allowed }))
    .filter(r => !r.allowed)
    .map(r => r.key);
  return { allowed: violations.length === 0, violations };
}
