import { randomBytes, randomUUID, timingSafeEqual } from 'node:crypto';
import type { Pool } from 'pg';
import { z } from 'zod';
import type { AuthService, Headers } from './auth.ts';
import { derivePassword, hashPassword } from './password.ts';
import { transaction } from './database.ts';
import { assert } from './errors.ts';
import type { PreviewStore } from './storage.ts';

export class AccountDataService {
  private readonly pool: Pool;
  private readonly auth: AuthService;
  private readonly store: PreviewStore;
  private readonly timer: ReturnType<typeof setInterval>;
  private cleaning?: Promise<void>;
  constructor(pool: Pool, auth: AuthService, store: PreviewStore) {
    this.pool = pool; this.auth = auth; this.store = store;
    this.timer = setInterval(() => { void this.clean().catch(() => {}); }, 30000);
    this.timer.unref();
  }
  async close() { clearInterval(this.timer); await this.cleaning; }

  async export(headers: Headers, input: unknown) {
    z.strictObject({}).parse(input);
    return transaction(this.pool, async tx => {
      await tx.query('SET TRANSACTION ISOLATION LEVEL REPEATABLE READ');
      const user = await this.auth.authenticate(headers, true, tx);
      const rows = async (sql: string) => {
        const result = (await tx.query(sql + ' LIMIT 10001', [user.id])).rows;
        assert(result.length <= 10000, 413, 'ACCOUNT_EXPORT_TOO_LARGE', '账户资料超过单次导出上限，请联系支持分批取回');
        return result;
      };
      const data = {
        version: 1, exported_at: new Date().toISOString(), user_id: user.id,
        profile: { id: user.id, email: user.email, name: user.name, avatar: user.avatar_url ?? null },
        devices: await rows('SELECT id,name,platform,created_at,last_seen_at FROM fr_devices WHERE user_id=$1 ORDER BY id'),
        memberships: await rows('SELECT workspace_id,role FROM fr_memberships WHERE user_id=$1 ORDER BY workspace_id'),
        owned_projects: await rows('SELECT id,workspace_id,title,state,revision,snapshot FROM fr_projects WHERE owner_id=$1 ORDER BY id'),
        comments: await rows('SELECT id,project_id,body,anchor,state,created_at FROM fr_comments WHERE author_id=$1 ORDER BY id'),
        research: await rows('SELECT id,project_id,kind,title,body,url,created_at FROM fr_research_references WHERE created_by=$1 ORDER BY id'),
        proposals: await rows('SELECT id,project_id,title,state,operations FROM fr_proposals WHERE author_id=$1 ORDER BY id'),
        ai_usage: await rows('SELECT request_id,state,reserved,charged,created_at,finished_at FROM fr_ai_requests WHERE user_id=$1 ORDER BY created_at'),
        invoices: await rows('SELECT id,amount,currency,status,issued_at FROM fr_billing_invoices WHERE user_id=$1 ORDER BY id'),
        notes: ['不含密码、会话令牌、设备密钥、第三方支付标识或模型密钥。', '照片原片仅在来源设备；请另用本机项目导出取回原片。'],
      };
      assert(Buffer.byteLength(JSON.stringify(data)) <= 8 * 1024 * 1024, 413, 'ACCOUNT_EXPORT_TOO_LARGE', '账户资料超过单次导出上限');
      return data;
    });
  }

  async erase(headers: Headers, input: unknown) {
    const value = z.strictObject({ password: z.string().min(8).max(1024), confirm_email: z.email().max(254),
      understand_retention: z.literal(true) }).parse(input);
    const identity = await this.auth.authenticate(headers, true);
    const row = (await this.pool.query('SELECT password_hash FROM fr_users WHERE id=$1', [identity.id])).rows[0];
    const parts: string[] = row?.password_hash.split('$') ?? [];
    const actual = await derivePassword(value.password, parts[1] ?? 'invalid-account');
    const expected = Buffer.from(parts[2] ?? ''.padEnd(64, '0'), 'hex');
    assert(parts[0] === 'scrypt-v1' && actual.length === expected.length && timingSafeEqual(actual, expected)
      && value.confirm_email.toLowerCase() === identity.email, 403, 'REAUTH_REQUIRED', '请核对当前密码和账号邮箱');
    const replacement = await hashPassword(randomBytes(32).toString('hex'));
    const receipt = await transaction(this.pool, async tx => {
      const current = (await tx.query('SELECT password_hash FROM fr_users WHERE id=$1 FOR UPDATE', [identity.id])).rows[0];
      const user = await this.auth.authenticate(headers, true, tx);
      assert(current.password_hash === row.password_hash, 409, 'REAUTH_REQUIRED', '密码已变化，请重新确认');
      // 不能在未结算任务/订阅仍有效时关闭身份，避免未知扣费与后台执行变成孤儿任务。
      const inflight = (await tx.query("SELECT 1 FROM fr_ai_requests WHERE user_id=$1 AND state='PENDING' LIMIT 1", [user.id])).rowCount;
      const agent = (await tx.query("SELECT 1 FROM fr_agent_jobs WHERE created_by=$1 AND status IN ('QUEUED','RUNNING') LIMIT 1", [user.id])).rowCount;
      assert(!inflight && !agent, 409, 'ACCOUNT_TASKS_ACTIVE', '请先等待或取消正在进行的 AI 任务');
      const subscription = (await tx.query("SELECT 1 FROM fr_billing_subscriptions WHERE user_id=$1 AND status NOT IN ('canceled','incomplete_expired') LIMIT 1", [user.id])).rowCount;
      assert(!subscription, 409, 'ACCOUNT_SUBSCRIPTION_ACTIVE', '请先在订阅管理中结束订阅，避免继续扣费');
      const receiptID = randomUUID();
      await tx.query('INSERT INTO fr_account_closures(user_id,receipt_id) VALUES($1,$2)', [user.id, receiptID]);
      // 工作区先于项目加锁，沿用权威通道顺序。只有没有其他可访问主体的项目才清除作品内容。
      await tx.query('SELECT id FROM fr_workspaces WHERE owner_id=$1 ORDER BY id FOR UPDATE', [user.id]);
      const projects = (await tx.query(`SELECT p.id FROM fr_projects p LEFT JOIN fr_project_sync s ON s.project_id=p.id
        WHERE p.owner_id=$1 AND (s.mode IN ('PRIVATE_SYNCED','SYNC_ENROLLING','ENROLL_FAILED') OR
          (NOT EXISTS(SELECT 1 FROM fr_grants g WHERE g.project_id=p.id AND g.active AND g.user_id<>$1)
           AND NOT EXISTS(SELECT 1 FROM fr_memberships m WHERE m.workspace_id=p.workspace_id AND m.user_id<>$1 AND m.role IN ('OWNER','ADMIN'))))
        ORDER BY p.id FOR UPDATE OF p`, [user.id])).rows;
      for (const { id } of projects) {
        await tx.query(`INSERT INTO fr_erasure_audit(receipt_id,project_id,detail)
          SELECT $2,id,jsonb_build_object('revision',revision,'manifest_digest',manifest_digest) FROM fr_projects WHERE id=$1`, [id, receiptID]);
        await tx.query('INSERT INTO fr_erasure_objects(object_key) SELECT object_key FROM fr_previews WHERE project_id=$1 ON CONFLICT DO NOTHING', [id]);
        for (const table of ['fr_previews', 'fr_boards', 'fr_source_assets', 'fr_comments', 'fr_proposals', 'fr_research_references', 'fr_drafts', 'fr_author_hypotheses', 'fr_agent_task_results', 'fr_agent_jobs', 'fr_model_consents', 'fr_agent_scopes']) {
          // 表名为固定白名单；标识值全部参数化。保留最小权威行与审计身份，不保留私有正文。
          await tx.query(`DELETE FROM ${table} WHERE project_id=$1`, [id]);
        }
        await tx.query("UPDATE fr_projects SET state='ABORTED',title='已删除项目',snapshot='{\"photos\":[],\"sequences\":[]}' WHERE id=$1", [id]);
        await tx.query("UPDATE fr_versions SET snapshot='{\"photos\":[],\"sequences\":[]}' WHERE project_id=$1", [id]);
        await tx.query("UPDATE fr_commands SET envelope='{\"erased\":true}' WHERE project_id=$1", [id]);
        await tx.query("UPDATE fr_events SET detail='{\"erased\":true}' WHERE project_id=$1", [id]);
      }
      await tx.query("UPDATE fr_model_consents SET state='REVOKED',revoked_at=now() WHERE granted_by=$1 AND state='ACTIVE'", [user.id]);
      await tx.query('UPDATE fr_agent_scopes SET revoked=true WHERE issued_by=$1', [user.id]);
      await tx.query('DELETE FROM fr_password_reset_tokens WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_email_verify_tokens WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_magic_login_codes WHERE email=$1', [user.email]);
      await tx.query("UPDATE fr_invitations SET state='REVOKED' WHERE state='PENDING' AND (email=$1 OR invited_by=$2)", [user.email, user.id]);
      await tx.query('UPDATE fr_invitations SET email=$2 WHERE email=$1', [user.email, `deleted-${receiptID}@deleted.invalid`]);
      await tx.query('DELETE FROM fr_devices WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_sessions WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_drafts WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_presence WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_board_presence WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_team_seats WHERE user_id=$1', [user.id]);
      await tx.query('DELETE FROM fr_memberships WHERE user_id=$1', [user.id]);
      await tx.query('UPDATE fr_grants SET active=false WHERE user_id=$1', [user.id]);
      await tx.query('UPDATE fr_ai_requests SET result_json=NULL WHERE user_id=$1', [user.id]);
      await tx.query("UPDATE fr_users SET email=$2,display_name='已删除账户',avatar_url=NULL,email_verified_at=NULL,password_hash=$3 WHERE id=$1", [user.id, `deleted-${receiptID}@deleted.invalid`, replacement]);
      await tx.query("UPDATE fr_workspaces SET title='已关闭账户的工作区' WHERE owner_id=$1", [user.id]);
      return { closed: true, receipt_id: receiptID, erased_private_projects: projects.length,
        retained: ['共享协作作品及匿名历史', '计量、财务和访问审计记录'], media_cleanup: 'scheduled', local_originals_retained: true };
    });
    await this.clean().catch(() => {});
    return receipt;
  }

  clean(): Promise<void> {
    if (this.cleaning) return this.cleaning;
    const work = this.purge(); this.cleaning = work;
    void work.finally(() => { if (this.cleaning === work) this.cleaning = undefined; }).catch(() => {});
    return work;
  }
  private async purge() {
    if (!this.store.remove) return;
    const rows = (await this.pool.query('SELECT object_key FROM fr_erasure_objects ORDER BY created_at LIMIT 100')).rows;
    for (const { object_key: key } of rows) {
      await transaction(this.pool, async tx => {
        await tx.query('SELECT pg_advisory_xact_lock(hashtextextended($1,0))', ['media:' + key]);
        const used = (await tx.query('SELECT 1 FROM fr_previews WHERE object_key=$1 UNION ALL SELECT 1 FROM fr_renditions WHERE object_key=$1 LIMIT 1', [key])).rowCount;
        if (!used) await this.store.remove!(key);
        await tx.query('DELETE FROM fr_erasure_objects WHERE object_key=$1', [key]);
      });
    }
  }
}
