import { dispatch, type Principal, type ProjectAuthority } from '@betweenframes/contracts';
import { Pool } from 'pg';
import {
  storeTaskResult,
  summarizeProjectComments,
  type CommentKind,
} from '../../../apps/api/src/agent-tasks.ts';

// M4 云 Agent Worker：PostgreSQL 队列 + 租约恢复。
// 只读评论 → 授权范围内调模型汇总 → 存结果；不写正式状态、不上传图片、不访问作者记忆。
// 模型 key 经 FOCALROOM_AGENT_MODEL_KEY 环境变量注入，不落库。

export function dispatchAgentProposal(input: unknown, principal: Extract<Principal, { type: 'agent' }>, project: ProjectAuthority) {
  return dispatch(input, principal, project, 'CLOUD');
}
export const workerStatus = { implemented: true, milestone: 'M4', message: '云 Agent Worker：授权队列 + 租约恢复；只读评论汇总，不写正式状态' } as const;

const MAX_ATTEMPTS = 3;
const LEASE_SECONDS = 120;

export interface ClaimedJob {
  id: string;
  project_id: string;
  consent_id: string;
  created_by: string;
  kind: CommentKind;
  attempts: number;
}

/** 领取一个可执行任务：QUEUED 优先，RUNNING 且租约过期的视为崩溃可恢复。 */
async function claim(pool: Pool): Promise<ClaimedJob | null> {
  const row = (
    await pool.query(
      `UPDATE fr_agent_jobs SET status='RUNNING', attempts=attempts+1,
              lease_expires=now()+interval '${LEASE_SECONDS} seconds', updated_at=now()
       WHERE id=(SELECT id FROM fr_agent_jobs
                 WHERE status='QUEUED' OR (status='RUNNING' AND lease_expires<now())
                 ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED)
       RETURNING id, project_id, consent_id, created_by, kind, attempts`,
    )
  ).rows[0];
  return row ?? null;
}

async function loadComments(pool: Pool, projectId: string) {
  return (
    await pool.query(
      `SELECT u.display_name AS reviewer, c.body FROM fr_comments c
       JOIN fr_users u ON u.id=c.author_id
       WHERE c.project_id=$1 AND c.state<>'RESOLVED' ORDER BY c.created_at LIMIT 200`,
      [projectId],
    )
  ).rows as Array<{ reviewer: string; body: string }>;
}

export interface WorkerRunResult {
  claimed: boolean;
  jobId?: string;
  status?: 'COMPLETED' | 'FAILED' | 'REQUEUED' | 'SKIPPED';
  detail?: string;
}

/** 执行至多一个任务；返回是否领取与结果状态。可重复调用（单步）或由循环驱动。 */
export async function runWorkerOnce(pool: Pool): Promise<WorkerRunResult> {
  const job = await claim(pool);
  if (!job) return { claimed: false };
  try {
    // 授权在执行前重新校验并扣减额度：撤权/用尽后任务标记失败，不发起模型请求。
    const consent = (
      await pool.query(
        `UPDATE fr_model_consents SET used=used+1,
                state=CASE WHEN used+1>=max_tasks THEN 'EXHAUSTED' ELSE state END
         WHERE id=$1 AND state='ACTIVE' AND used<max_tasks
         RETURNING endpoint_label, model_id`,
        [job.consent_id],
      )
    ).rows[0];
    if (!consent) {
      await pool.query(
        `UPDATE fr_agent_jobs SET status='FAILED', error='授权已撤回或额度用尽', updated_at=now() WHERE id=$1`,
        [job.id],
      );
      return { claimed: true, jobId: job.id, status: 'FAILED', detail: 'consent inactive' };
    }
    const comments = await loadComments(pool, job.project_id);
    if (!comments.length) {
      await pool.query(
        `UPDATE fr_agent_jobs SET status='FAILED', error='没有可分析的评论', updated_at=now() WHERE id=$1`,
        [job.id],
      );
      return { claimed: true, jobId: job.id, status: 'FAILED', detail: 'no comments' };
    }
    const result = await summarizeProjectComments(pool, job.project_id, comments, job.kind);
    const resultId = await storeTaskResult(pool, job.project_id, job.created_by, {
      ...result,
      comment_count: result.comment_count,
    });
    await pool.query(
      `UPDATE fr_agent_jobs SET status='COMPLETED', result_id=$2,
              checkpoint=jsonb_build_object('comment_count', $3::int, 'result_id', $2),
              error=NULL, lease_expires=NULL, updated_at=now() WHERE id=$1`,
      [job.id, resultId, comments.length],
    );
    return { claimed: true, jobId: job.id, status: 'COMPLETED', detail: resultId };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    const requeue = job.attempts < MAX_ATTEMPTS;
    await pool.query(
      `UPDATE fr_agent_jobs SET status=$2, error=$3, lease_expires=NULL, updated_at=now() WHERE id=$1`,
      [job.id, requeue ? 'QUEUED' : 'FAILED', message.slice(0, 500)],
    );
    return {
      claimed: true,
      jobId: job.id,
      status: requeue ? 'REQUEUED' : 'FAILED',
      detail: message,
    };
  }
}

/** 进程模式：固定间隔轮询；收到信号后在当前任务完成后停止。 */
export async function startWorkerLoop(pool: Pool, intervalMs = 2000) {
  let running = true;
  const stop = () => { running = false; };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  while (running) {
    try {
      const result = await runWorkerOnce(pool);
      if (!result.claimed) await new Promise((resolve) => setTimeout(resolve, intervalMs));
    } catch (error) {
      console.error('[agent-worker] 轮询异常：', error);
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
  console.log('[agent-worker] 已停止');
}

export { readScopedComments } from './comments.ts';
