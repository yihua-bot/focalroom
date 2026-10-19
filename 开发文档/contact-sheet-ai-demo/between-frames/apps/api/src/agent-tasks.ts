/**
 * M4 云 Agent Worker：评论汇总任务。
 * 只读评论 → 调授权模型汇总 → 存结果；不写正式状态、不上传图片、不访问作者记忆。
 * 模型 key 经 FOCALROOM_AGENT_MODEL_KEY 环境变量注入，不落库。
 */
import { Pool } from "pg";

export interface SummarizeResult {
  summary: string;
  comment_count: number;
  tokens_in: number;
  tokens_out: number;
  cost_estimate_minor_usd: number;
  model_id: string;
}

const MODEL_BASE_URL = () =>
  process.env.FOCALROOM_AGENT_MODEL_BASE_URL ?? "https://api.deepseek.com/v1";
const MODEL_NAME = () => process.env.FOCALROOM_AGENT_MODEL_NAME ?? "deepseek-chat";

/** 服务端配置的发送端点摘要（不含密钥），用于授权记录与回显。 */
export function modelEndpointLabel(): string {
  return `${MODEL_NAME()}@${new URL(MODEL_BASE_URL()).host}`;
}

export type CommentKind = "summarize_comments" | "summarize_disagreement";

function buildPrompt(kind: CommentKind, comments: Array<{ reviewer: string; body: string }>) {
  if (kind === "summarize_disagreement")
    return {
      system:
        "你是摄影项目的审稿助手。只分析人类评论之间的分歧并给出可审阅的折中提案要点；不添加自己的审美判断，不扩大任何评论者的主张。用中文简洁输出。",
      user: [
        "以下是摄影项目的人工审稿评论。请输出：",
        "1. 各方一致同意的要点",
        "2. 明确分歧（谁主张什么、依据是什么）",
        "3. 折中提案要点（保留双方核心诉求的可选方案，供作者决策，不代替决定）",
        "4. 需要作者澄清的问题",
        "",
        ...comments.map((c, i) => `${i + 1}. [${c.reviewer}] ${c.body}`),
      ].join("\n"),
    };
  return {
    system:
      "你是摄影项目的审稿助手。只汇总人类评论，不添加自己的审美判断。用中文简洁输出。",
    user: [
      "以下是摄影项目的人工审稿评论。请汇总：",
      "1. 整体评价倾向（正面/负面/中性）",
      "2. 主要关注点（出现 2+ 次的主题）",
      "3. 具体建议（可执行的改进项）",
      "4. 未解决争议（如有）",
      "",
      ...comments.map((c, i) => `${i + 1}. [${c.reviewer}] ${c.body}`),
    ].join("\n"),
  };
}

function costMinorUsd(tokensIn: number, tokensOut: number): number {
  // DeepSeek 峰时非缓存估算：输入 $0.30/M、输出 $1.20/M（保守整数向上取整）。
  return Math.ceil((tokensIn * 0.3 + tokensOut * 1.2) / 1000);
}

export async function summarizeProjectComments(
  _pool: Pool,
  _projectId: string,
  comments: Array<{ reviewer: string; body: string }>,
  kind: CommentKind = "summarize_comments",
): Promise<SummarizeResult> {
  const apiKey = process.env.FOCALROOM_AGENT_MODEL_KEY ?? "";
  if (!apiKey) {
    throw Object.assign(new Error("云 Agent 模型未配置"), { status: 503 });
  }
  const prompt = buildPrompt(kind, comments);

  const response = await fetch(`${MODEL_BASE_URL()}/chat/completions`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: MODEL_NAME(),
      messages: [
        { role: "system", content: prompt.system },
        { role: "user", content: prompt.user },
      ],
      max_tokens: 500,
    }),
    signal: AbortSignal.timeout(30_000),
  });

  if (!response.ok) {
    throw Object.assign(new Error(`模型调用失败: ${response.status}`), {
      status: 502,
    });
  }

  const data = (await response.json()) as {
    choices: Array<{ message: { content: string } }>;
    usage: { prompt_tokens: number; completion_tokens: number };
    model: string;
  };

  const summary = data.choices[0]?.message?.content ?? "";
  const tokensIn = data.usage?.prompt_tokens ?? 0;
  const tokensOut = data.usage?.completion_tokens ?? 0;

  return {
    summary,
    comment_count: comments.length,
    tokens_in: tokensIn,
    tokens_out: tokensOut,
    cost_estimate_minor_usd: costMinorUsd(tokensIn, tokensOut),
    model_id: data.model ?? MODEL_NAME(),
  };
}

export async function storeTaskResult(
  pool: Pool,
  projectId: string,
  createdBy: string,
  result: SummarizeResult,
  kind: CommentKind = "summarize_comments",
): Promise<string> {
  const row = (
    await pool.query(
      `INSERT INTO fr_agent_task_results
       (project_id, created_by, task_type, model_id, summary, comment_count, tokens_in, tokens_out, cost_estimate_minor_usd, provenance)
       VALUES ($1, $2, $10, $3, $4, $5, $6, $7, $8, $9::jsonb)
       RETURNING id`,
      [
        projectId,
        createdBy,
        result.model_id,
        result.summary,
        result.comment_count,
        result.tokens_in,
        result.tokens_out,
        result.cost_estimate_minor_usd,
        JSON.stringify({
          source: "human_project_comments",
          permits_model_upload: false,
          creates_author_memory: false,
        }),
        kind,
      ],
    )
  ).rows[0];
  return row.id;
}

export async function listTaskResults(
  pool: Pool,
  projectId: string,
  limit = 10,
) {
  return (
    await pool.query(
      `SELECT r.id, r.task_type, r.model_id, r.summary, r.comment_count,
              r.tokens_in, r.tokens_out, r.cost_estimate_minor_usd, r.created_at,
              u.display_name AS created_by_name
       FROM fr_agent_task_results r
       JOIN fr_users u ON u.id = r.created_by
       WHERE r.project_id = $1
       ORDER BY r.created_at DESC LIMIT $2`,
      [projectId, limit],
    )
  ).rows;
}
