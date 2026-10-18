/**
 * M4 运维指标：Prometheus 文本格式的内存注册表 + 数据库派生指标。
 * - API 延迟与请求计数按路由模板聚合（UUID/长数字段归一为 :id/:n）；
 * - 任务/费用/冲突等业务指标在抓取时从 PostgreSQL 实时查询（无缓存陈旧问题）；
 * - 可选 FOCALROOM_METRICS_TOKEN 保护 /metrics；生产建议仅内网暴露。
 * 告警建议（ops/shared/README）：队列积压、任务失败率、API 错误率、预览失败。
 */
import type { Pool } from "pg";

interface HistogramState {
  counts: number[];
  sum: number;
  count: number;
}

const BUCKETS_MS = [5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000];

export class MetricsRegistry {
  private requests = new Map<string, number>();
  private durations = new Map<string, HistogramState>();
  readonly startedAt = Date.now();

  /** 路由模板：长十六进制/UUID 段 → :id，纯数字段 → :n；限制基数防标签爆炸。 */
  static template(path: string): string {
    const normalized = path
      .split("/")
      .map((segment) => {
        if (segment.length >= 8 && /^[0-9a-f-]+$/i.test(segment)) return ":id";
        if (/^\d{2,}$/.test(segment)) return ":n";
        return segment;
      })
      .join("/");
    return normalized.length > 120 ? normalized.slice(0, 120) : normalized;
  }

  observe(route: string, method: string, status: number, durationMs: number) {
    const key = `${MetricsRegistry.template(route)}|${method}|${status}`;
    this.requests.set(key, (this.requests.get(key) ?? 0) + 1);
    const histogramKey = `${MetricsRegistry.template(route)}|${method}`;
    let histogram = this.durations.get(histogramKey);
    if (!histogram) {
      histogram = { counts: new Array(BUCKETS_MS.length + 1).fill(0), sum: 0, count: 0 };
      this.durations.set(histogramKey, histogram);
    }
    histogram.sum += durationMs;
    histogram.count += 1;
    const bucket = BUCKETS_MS.findIndex((edge) => durationMs <= edge);
    histogram.counts[bucket < 0 ? BUCKETS_MS.length : bucket] += 1;
    // 基数上限：防止异常路由模板撑爆内存。
    if (this.requests.size > 5_000) this.requests.clear();
    if (this.durations.size > 2_000) this.durations.clear();
  }

  /** 数据库派生指标：任务状态、费用、预览与冲突计数。抓取时实时查询。 */
  private async derived(pool: Pool): Promise<string[]> {
    const lines: string[] = [];
    try {
      const jobs = await pool.query(
        "SELECT kind, status, count(*)::int AS n FROM fr_agent_jobs GROUP BY kind, status",
      );
      for (const row of jobs.rows) {
        lines.push(
          `focalroom_agent_jobs{kind="${row.kind}",status="${row.status}"} ${row.n}`,
        );
      }
      const costs = await pool.query(
        "SELECT coalesce(sum(cost_estimate_minor_usd),0)::int AS usd, coalesce(sum(tokens_in+tokens_out),0)::bigint AS tokens FROM fr_agent_task_results",
      );
      lines.push(
        `focalroom_agent_cost_minor_usd_total ${costs.rows[0].usd}`,
        `focalroom_agent_tokens_total ${costs.rows[0].tokens}`,
      );
      const queueDepth = await pool.query(
        "SELECT count(*)::int AS n FROM fr_agent_jobs WHERE status='QUEUED'",
      );
      lines.push(`focalroom_agent_queue_depth ${queueDepth.rows[0].n}`);
    } catch {
      lines.push("focalroom_agent_metrics_error 1");
    }
    return lines;
  }

  async render(pool: Pool): Promise<string> {
    const lines: string[] = [];
    lines.push("# HELP focalroom_api_requests_total API 请求计数（按路由模板/方法/状态）");
    lines.push("# TYPE focalroom_api_requests_total counter");
    for (const [key, value] of this.requests) {
      const [route, method, status] = key.split("|");
      lines.push(
        `focalroom_api_requests_total{route="${route}",method="${method}",status="${status}"} ${value}`,
      );
    }
    lines.push("# HELP focalroom_api_request_duration_ms API 延迟分布（毫秒桶）");
    lines.push("# TYPE focalroom_api_request_duration_ms histogram");
    for (const [key, histogram] of this.durations) {
      const [route, method] = key.split("|");
      let cumulative = 0;
      BUCKETS_MS.forEach((edge, index) => {
        cumulative += histogram.counts[index];
        lines.push(
          `focalroom_api_request_duration_ms_bucket{route="${route}",method="${method}",le="${edge}"} ${cumulative}`,
        );
      });
      cumulative += histogram.counts[BUCKETS_MS.length];
      lines.push(
        `focalroom_api_request_duration_ms_bucket{route="${route}",method="${method}",le="+Inf"} ${cumulative}`,
        `focalroom_api_request_duration_ms_sum{route="${route}",method="${method}"} ${histogram.sum}`,
        `focalroom_api_request_duration_ms_count{route="${route}",method="${method}"} ${histogram.count}`,
      );
    }
    lines.push("# HELP focalroom_agent_jobs 云 Agent 任务状态计数");
    lines.push("# TYPE focalroom_agent_jobs gauge");
    lines.push("# HELP focalroom_agent_queue_depth 等待执行的任务数");
    lines.push("# TYPE focalroom_agent_queue_depth gauge");
    lines.push("# HELP focalroom_agent_cost_minor_usd_total 模型费用累计（美分，估算）");
    lines.push("# TYPE focalroom_agent_cost_minor_usd_total counter");
    lines.push(...(await this.derived(pool)));
    lines.push(`focalroom_metrics_uptime_seconds ${Math.floor((Date.now() - this.startedAt) / 1000)}`);
    return `${lines.join("\n")}\n`;
  }
}

/** 兼容 fetch Headers 与 Node 原生头的普通对象两种形态。 */
export function metricsAuthorized(
  search: URLSearchParams,
  headers: { authorization?: string } | Headers,
): boolean {
  const token = process.env.FOCALROOM_METRICS_TOKEN;
  if (!token) return true;
  const authorization =
    typeof (headers as Headers).get === "function"
      ? (headers as Headers).get("authorization")
      : (headers as { authorization?: string }).authorization;
  return search.get("token") === token || authorization === `Bearer ${token}`;
}
