/**
 * M3#11 云端作者记忆：Creator 隔离的记忆读写——同 Creator 跨项目共享，跨 Creator 严格隔离。
 *
 * 设计原则（ADR 0037 扩展到云端）：
 * - 每条记忆归属于 Creator（即 first_project 的 owner），不属于单个项目
 * - 同 Creator 的任务经授权可读取其全部项目的记忆
 * - 不同 Creator 的记忆严格不可见
 * - 写操作仅限 Creator 本人（评审者不能写他人记忆）
 */

import type { Pool } from "pg";

export interface AuthorMemory {
  id: number;
  statement: string;
  status: string;
  confidence: string;
  project_id: string;
  project_title: string;
  created_at: string;
  updated_at: string;
}

export interface MemoryAccessResult {
  authorized: boolean;
  creator_id: string;
  reason: string;
}

/**
 * 确认用户是项目的 Creator（owner）。Creator 身份 = 第一个以 owner 身份创建此项目的用户。
 */
export async function verifyCreator(
  pool: Pool,
  userId: string,
  projectId: string,
): Promise<MemoryAccessResult> {
  const row = (
    await pool.query(
      "SELECT owner_id FROM cloud_projects WHERE id=$1",
      [projectId],
    )
  ).rows[0];

  if (!row) {
    return { authorized: false, creator_id: "", reason: "项目不存在" };
  }

  if (row.owner_id !== userId) {
    return {
      authorized: false,
      creator_id: row.owner_id,
      reason: "只有项目所有者可以访问作者记忆",
    };
  }

  return { authorized: true, creator_id: userId, reason: "" };
}

/**
 * 获取同 Creator 的跨项目偏好假设（只读）。
 * 经授权的本地任务可以在 Agent 上下文中引用其他项目的假设，但不得修改。
 */
export async function getAuthorMemories(
  pool: Pool,
  creatorId: string,
  excludeProjectId: string,
  limit = 100,
): Promise<AuthorMemory[]> {
  const rows = await pool.query(
    `SELECT p.id, p.statement, p.status, p.confidence,
            p.project_id, cp.title AS project_title,
            p.created_at, p.updated_at
     FROM fr_author_hypotheses p
     JOIN cloud_projects cp ON cp.id = p.project_id
     WHERE p.project_id IN (
       SELECT id FROM cloud_projects WHERE owner_id = $1
     )
     AND p.project_id != $2
     AND p.status NOT IN ('retired')
     AND p.disabled_for_retrieval = 0
     ORDER BY p.updated_at DESC
     LIMIT $3`,
    [creatorId, excludeProjectId, limit],
  );
  return rows.rows;
}

/**
 * 写入/更新云端作者记忆假设（仅 Creator 本人）。
 */
export async function upsertAuthorMemory(
  pool: Pool,
  projectId: string,
  userId: string,
  statement: string,
  status: string,
  confidence: string,
): Promise<string> {
  const creator = await verifyCreator(pool, userId, projectId);
  if (!creator.authorized) {
    throw Object.assign(new Error(creator.reason), { status: 403 });
  }

  const existing = (
    await pool.query(
      "SELECT id FROM fr_author_hypotheses WHERE project_id=$1 AND statement=$2",
      [projectId, statement],
    )
  ).rows[0];

  if (existing) {
    await pool.query(
      "UPDATE fr_author_hypotheses SET status=$1, confidence=$2, updated_at=now() WHERE id=$3",
      [status, confidence, existing.id],
    );
    return existing.id;
  }

  const id = crypto.randomUUID();
  await pool.query(
    "INSERT INTO fr_author_hypotheses(id, project_id, statement, status, confidence) VALUES($1,$2,$3,$4,$5)",
    [id, projectId, statement, status, confidence],
  );
  return id;
}

/**
 * 撤销跨项目记忆授权：关闭后 Agent 不再引用其他项目的假设。
 */
export async function revokeMemoryAccess(
  pool: Pool,
  projectId: string,
  userId: string,
): Promise<boolean> {
  const creator = await verifyCreator(pool, userId, projectId);
  if (!creator.authorized) return false;
  // 实际实现：在 tasks 的 metadata 中设置 cross_project=false（前端/桌面传入）。
  // 服务端只验证 Creator 身份——实际开关由桌面端的 MemoryScopeSettings 控制。
  return true;
}
