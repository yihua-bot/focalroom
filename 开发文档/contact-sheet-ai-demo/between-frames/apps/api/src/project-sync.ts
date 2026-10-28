import { z } from 'zod';
import type { PoolClient } from 'pg';
import type { AuthorityService, Project } from './authority.ts';
import type { Headers } from './auth.ts';
import { digest } from './database.ts';
import { assert } from './errors.ts';

export async function syncState(tx: Pick<PoolClient, 'query'>, project: Project) {
  const row = (await tx.query('SELECT mode,version FROM fr_project_sync WHERE project_id=$1', [project.id])).rows[0];
  return { project_id: project.id, mode: row?.mode ?? (project.state === 'STAGING' ? 'SYNC_ENROLLING' : project.state === 'ABORTED' ? 'ENROLL_FAILED' : 'SHARED'),
    version: row?.version ?? 0, authority: project.state === 'SHARED' ? 'cloud' : 'enrolling' };
}

export class ProjectSyncService {
  private readonly authority: AuthorityService;
  constructor(authority: AuthorityService) { this.authority = authority; }

  read(headers: Headers, id: string) {
    return this.authority.scope(headers, id, false, async ({ tx, user, project }) => {
      assert(project.state === 'SHARED' || project.owner_id === user.id, 403, 'PERMISSION_DENIED', '迁移尚未完成');
      return syncState(tx, project);
    });
  }

  change(headers: Headers, id: string, input: unknown) {
    const value = z.strictObject({ command_id: z.uuid(), expected_version: z.number().int().min(0),
      mode: z.enum(['PRIVATE_SYNCED', 'SHARED']) }).parse(input);
    return this.authority.scope(headers, id, true, async ({ tx, user, project }) => {
      assert(project.owner_id === user.id && project.state === 'SHARED', 403, 'PERMISSION_DENIED', '仅云端项目所有者可调整共享范围');
      const hash = digest(JSON.stringify(value));
      const prior = (await tx.query('SELECT actor_id,payload_digest,reply FROM fr_sync_commands WHERE project_id=$1 AND command_id=$2', [id, value.command_id])).rows[0];
      if (prior) {
        assert(prior.actor_id === user.id && prior.payload_digest === hash, 409, 'IDEMPOTENCY_CONFLICT', '命令编号已用于其他操作');
        return prior.reply;
      }
      const current = await syncState(tx, project);
      assert(current.version === value.expected_version, 409, 'SYNC_VERSION_CONFLICT', '共享范围已改变，请刷新');
      const version = current.version + 1;
      if (value.mode === 'PRIVATE_SYNCED') {
        await tx.query('UPDATE fr_grants SET active=false WHERE project_id=$1', [id]);
        await tx.query("UPDATE fr_invitations SET state='REVOKED' WHERE project_id=$1 AND state='PENDING'", [id]);
        await tx.query('UPDATE fr_agent_scopes SET revoked=true WHERE project_id=$1', [id]);
        await tx.query("UPDATE fr_model_consents SET state='REVOKED',revoked_at=now() WHERE project_id=$1 AND state='ACTIVE'", [id]);
      }
      await tx.query(`INSERT INTO fr_project_sync(project_id,mode,version) VALUES($1,$2,$3)
        ON CONFLICT(project_id) DO UPDATE SET mode=excluded.mode,version=excluded.version,updated_at=now()`, [id, value.mode, version]);
      const reply = { project_id: id, mode: value.mode, version, authority: 'cloud' };
      await tx.query('INSERT INTO fr_sync_commands(project_id,command_id,actor_id,payload_digest,reply) VALUES($1,$2,$3,$4,$5)', [id, value.command_id, user.id, hash, JSON.stringify(reply)]);
      await tx.query("INSERT INTO fr_events(project_id,actor_id,kind,revision,detail) VALUES($1,$2,'SyncModeChanged',$3,$4)", [id, user.id, project.revision, JSON.stringify(reply)]);
      return reply;
    });
  }
}
