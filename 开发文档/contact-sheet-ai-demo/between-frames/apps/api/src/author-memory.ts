/**
 * 云端作者记忆（剩余清单 #9）：Creator 隔离的跨项目偏好假设（ADR 0037 的云端扩展）。
 * - 假设经 fr_projects.owner_id 归属 Creator；同 Creator 跨项目共享，跨 Creator 严格不可见
 * - 写操作仅限 Creator 本人；评审者与受邀编辑不获得记忆写权限
 * - 0010_author_memory.sql 是未登记设计稿（UUID/cloud_projects 不兼容）；实际结构由 0023 落地
 */
import type { Pool } from "pg";
import { z } from "zod";
import { assert } from "./errors.ts";

const projectIdSchema = z.string().trim().min(1).max(160);
export const authorMemoryQuerySchema = z.strictObject({ project: projectIdSchema });
export const authorMemoryUpsertSchema = z.strictObject({
  projectId: projectIdSchema,
  statement: z.string().trim().min(1).max(1000),
  status: z.enum(["unconfirmed", "confirmed", "corrected", "retired"]),
  confidence: z.enum(["tentative", "moderate"]),
});

export interface AuthorMemory {
  id: string;
  statement: string;
  status: string;
  confidence: string;
  project_id: string;
  project_title: string;
  created_at: string;
  updated_at: string;
}

async function requireCreator(pool: Pool, userId: string, projectId: string): Promise<void> {
  const row = (await pool.query("SELECT owner_id FROM fr_projects WHERE id=$1", [projectId])).rows[0];
  assert(row, 404, "NOT_FOUND", "项目不存在");
  assert(row.owner_id === userId, 403, "PERMISSION_DENIED", "只有项目所有者可以访问作者记忆");
}

/**
 * 读取同 Creator 的跨项目偏好假设（只读）：验证 excludeProject 归属后，
 * 仅返回调用者本人拥有的其他项目中未停用、未退役的假设。
 */
export async function getAuthorMemories(
  pool: Pool,
  userId: string,
  project: unknown,
): Promise<AuthorMemory[]> {
  const { project: excludeProjectId } = authorMemoryQuerySchema.parse({ project });
  await requireCreator(pool, userId, excludeProjectId);
  return (
    await pool.query(
      `SELECT h.id, h.statement, h.status, h.confidence, h.project_id,
              p.title AS project_title, h.created_at, h.updated_at
       FROM fr_author_hypotheses h
       JOIN fr_projects p ON p.id = h.project_id
       WHERE p.owner_id = $1 AND h.project_id <> $2
         AND h.status <> 'retired' AND NOT h.disabled_for_retrieval
       ORDER BY h.updated_at DESC, h.id DESC
       LIMIT 100`,
      [userId, excludeProjectId],
    )
  ).rows;
}

/**
 * 写入/更新云端作者记忆假设（仅 Creator 本人）。按 (project_id, statement) 幂等。
 */
export async function upsertAuthorMemory(
  pool: Pool,
  userId: string,
  input: unknown,
): Promise<string> {
  const value = authorMemoryUpsertSchema.parse(input);
  await requireCreator(pool, userId, value.projectId);
  const existing = (
    await pool.query(
      "SELECT id FROM fr_author_hypotheses WHERE project_id=$1 AND statement=$2",
      [value.projectId, value.statement],
    )
  ).rows[0];
  if (existing) {
    await pool.query(
      "UPDATE fr_author_hypotheses SET status=$1, confidence=$2, updated_at=now() WHERE id=$3",
      [value.status, value.confidence, existing.id],
    );
    return existing.id;
  }
  const id = crypto.randomUUID();
  await pool.query(
    "INSERT INTO fr_author_hypotheses(id, project_id, statement, status, confidence) VALUES($1,$2,$3,$4,$5)",
    [id, value.projectId, value.statement, value.status, value.confidence],
  );
  return id;
}
