// 长期记忆：任务摘要生成 + 检索 + 注入
// 参考 OpenYC RAG 设计的 summary memory 层

import { Pool } from "pg";
import { z } from "zod";

export interface TaskSummary {
  id: string;
  summary: string;
  kind: string;
  preferences: string[];
  createdAt: string;
}

export class SummaryMemoryService {
  private pool: Pool;
  constructor(pool: Pool) { this.pool = pool; }

  /** 任务完成后保存摘要（幂等：同 task_id 不重复）。 */
  async save(input: {
    userId: string; projectId: string; taskId?: string;
    summary: string; kind?: string; preferences?: string[];
  }): Promise<void> {
    await this.pool.query(
      `INSERT INTO fr_task_summaries(user_id, project_id, task_id, summary, kind, preferences_extracted)
       VALUES($1, $2, $3, $4, $5, $6::jsonb)
       ON CONFLICT DO NOTHING`,
      [input.userId, input.projectId, input.taskId ?? null, input.summary, input.kind ?? "general", JSON.stringify(input.preferences ?? [])],
    );
  }

  /** 检索最近的任务摘要（同项目优先，跨项目补足）。 */
  async retrieve(userId: string, projectId: string, limit = 5): Promise<TaskSummary[]> {
    const rows = (await this.pool.query(
      `SELECT id::text, summary, kind, preferences_extracted, created_at,
              (project_id = $2) as same_project
       FROM fr_task_summaries
       WHERE user_id = $1
       ORDER BY same_project DESC, created_at DESC
       LIMIT $3`,
      [userId, projectId, limit],
    )).rows;
    return rows.map((r: Record<string, unknown>) => ({
      id: String(r.id),
      summary: String(r.summary),
      kind: String(r.kind),
      preferences: (r.preferences_extracted as string[]) ?? [],
      createdAt: String(r.created_at),
    }));
  }

  /** 生成注入 Agent 上下文的摘要文本块。 */
  async contextBlock(userId: string, projectId: string): Promise<string> {
    const summaries = await this.retrieve(userId, projectId);
    if (!summaries.length) return "";
    const lines = summaries.map(s => `- [${s.kind}] ${s.summary}`);
    const prefs = [...new Set(summaries.flatMap(s => s.preferences))];
    const prefLine = prefs.length ? `\n用户偏好（从历史任务提取）：${prefs.join("；")}` : "";
    return `## 历史任务摘要\n${lines.join("\n")}${prefLine}`;
  }
}
