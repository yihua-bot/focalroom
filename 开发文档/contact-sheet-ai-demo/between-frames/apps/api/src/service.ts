import { requestMagicCode, verifyMagicCode, type MagicEmailDelivery } from "./magic-login.ts";
import { loginWithGoogle } from "./google-login.ts";
import { BoardService } from "./boards.ts";
import { ProjectSyncService } from './project-sync.ts';
import { AccountDataService } from './account-data.ts';
import { BillingService } from './billing.ts';
import type { BillingConfig, BillingGateway } from './billing-gateway.ts';
import { DesktopLoginService } from "./desktop-login.ts";
import { HostedAiService, type HostedAiConfig, type HostedAiTransport } from "./hosted-ai.ts";
import { AccountRecoveryService, type AccountEmailDelivery } from "./account-recovery.ts";
import { listTaskResults, modelEndpointLabel } from "./agent-tasks.ts";
import { MetricsRegistry } from "./metrics.ts";
import { randomBytes, randomUUID } from "node:crypto";
import type { Pool } from "pg";
import { z } from "zod";
import { promotionSchema } from "@betweenframes/contracts";
import {
  AuthorityService,
  parseCloudCommand,
  type Scope,
} from "./authority.ts";
import { AuthService, type Headers } from "./auth.ts";
import { digest, transaction } from "./database.ts";
import { assert, ApiError } from "./errors.ts";
import { sanitizePreview, type PreviewStore } from "./storage.ts";
const inviteInput = z.strictObject({
  email: z
    .email()
    .max(254)
    .transform((s) => s.toLowerCase()),
  role: z.enum(["ADMIN", "EDITOR", "REVIEWER", "VIEWER"]),
  project_id: z.string().min(1).max(160).optional(),
});
type SnapshotEntity = { id: string; [key: string]: unknown };
// 版本快照在写入时经 manifest/命令通道校验；读取侧再验证结构，坏数据报错不静默。
function snapshotEntities(value: unknown): SnapshotEntity[] {
  assert(Array.isArray(value), 500, "SNAPSHOT_INVALID", "版本快照结构不合法");
  return value.map((entity) => {
    assert(
      entity !== null && typeof entity === "object" &&
        typeof (entity as Record<string, unknown>).id === "string",
      500, "SNAPSHOT_INVALID", "版本快照结构不合法",
    );
    return entity as SnapshotEntity;
  });
}
function diffEntities(before: SnapshotEntity[], after: SnapshotEntity[]) {
  const beforeMap = new Map(before.map((entity) => [entity.id, entity]));
  const afterMap = new Map(after.map((entity) => [entity.id, entity]));
  const changed = after
    .filter((entity) => beforeMap.has(entity.id))
    .map((entity) => {
      const prior = beforeMap.get(entity.id)!;
      const fields: Record<string, { before: unknown; after: unknown }> = {};
      for (const key of new Set([...Object.keys(prior), ...Object.keys(entity)])) {
        if (key !== "id" && JSON.stringify(prior[key]) !== JSON.stringify(entity[key]))
          fields[key] = { before: prior[key], after: entity[key] };
      }
      return { id: entity.id, fields };
    })
    .filter((entry) => Object.keys(entry.fields).length > 0);
  return {
    added: after.filter((entity) => !beforeMap.has(entity.id)).map((entity) => entity.id),
    removed: before.filter((entity) => !afterMap.has(entity.id)).map((entity) => entity.id),
    changed,
  };
}
type ProposalOperation = { op: "add" | "move" | "remove"; operation_id: string; photo_id: string; to_index?: number };
// 提案操作在创建时经命令契约校验；读取侧轻量验证结构，坏数据报错不静默。
function proposalOperations(value: unknown): ProposalOperation[] {
  assert(Array.isArray(value), 500, "PROPOSAL_OPS_INVALID", "提案操作结构不合法");
  return value.map((operation) => {
    assert(operation !== null && typeof operation === "object", 500, "PROPOSAL_OPS_INVALID", "提案操作结构不合法");
    const record = operation as Record<string, unknown>;
    assert(
      (record.op === "add" || record.op === "move" || record.op === "remove") &&
        typeof record.operation_id === "string" && typeof record.photo_id === "string" &&
        (record.to_index === undefined || typeof record.to_index === "number"),
      500, "PROPOSAL_OPS_INVALID", "提案操作结构不合法",
    );
    return operation as ProposalOperation;
  });
}
const researchReferenceSchema = z.discriminatedUnion("kind", [
  z.strictObject({
    kind: z.literal("note"), title: z.string().trim().min(1).max(200),
    body: z.string().trim().min(1).max(10000),
    author: z.string().trim().min(1).max(200).optional(),
    source: z.string().trim().min(1).max(500).optional(),
    license: z.string().trim().min(1).max(500).optional(),
  }),
  z.strictObject({
    kind: z.literal("link"), title: z.string().trim().min(1).max(200),
    url: z.string().url().max(2000),
    author: z.string().trim().min(1).max(200).optional(),
    source: z.string().trim().min(1).max(500).optional(),
    license: z.string().trim().min(1).max(500).optional(),
  }),
]);
const researchEditSchema = z.strictObject({
  title: z.string().trim().min(1).max(200).optional(),
  body: z.string().trim().min(1).max(10000).optional(),
  url: z.string().url().max(2000).optional(),
  author: z.string().trim().min(1).max(200).optional(),
  source: z.string().trim().min(1).max(500).optional(),
  license: z.string().trim().min(1).max(500).optional(),
});
export class CloudService {
  /** 运维指标注册表（每实例独立；/metrics 抓取时含 DB 派生指标）。 */
  readonly metrics = new MetricsRegistry();
  readonly pool: Pool;
  readonly auth: AuthService;
  readonly authority: AuthorityService;
  readonly store: PreviewStore;
  readonly accounts: AccountRecoveryService;
  readonly desktop: DesktopLoginService;
  readonly boards: BoardService;
  readonly sync: ProjectSyncService;
  readonly data: AccountDataService;
  readonly ai: HostedAiService;
  readonly billing: BillingService;
  private readonly magicEmail?: MagicEmailDelivery;
  constructor(pool: Pool, origin: string, store: PreviewStore, email?: AccountEmailDelivery, magicEmail?: MagicEmailDelivery, hostedAi?: {config?:HostedAiConfig;transport?:HostedAiTransport}, billing?: {config?:BillingConfig;gateway?:BillingGateway}) {
    this.pool = pool;
    this.magicEmail = magicEmail;
    this.auth = new AuthService(pool, origin);
    this.billing = new BillingService(pool,this.auth,billing?.config,billing?.gateway,Boolean(hostedAi?.config));
    this.ai = new HostedAiService(pool,this.auth,hostedAi?.config,hostedAi?.transport);
    this.desktop = new DesktopLoginService(pool, this.auth);
    this.authority = new AuthorityService(pool, this.auth);
    this.boards = new BoardService(this.authority);
    this.sync = new ProjectSyncService(this.authority);
    this.store = store;
    this.data = new AccountDataService(pool, this.auth, store);
    this.accounts = new AccountRecoveryService(pool, this.auth, origin, email);
  }
  async workspaces(headers: Headers) {
    const user = await this.auth.authenticate(headers, false);
    return (
      await this.pool.query(
        "SELECT w.*,m.role FROM fr_workspaces w JOIN fr_memberships m ON m.workspace_id=w.id WHERE m.user_id=$1 ORDER BY w.id",
        [user.id],
      )
    ).rows;
  }
  async devices(headers: Headers) {
    const user = await this.auth.authenticate(headers, false);
    const rows = (
      await this.pool.query(
        `SELECT d.id,d.name,d.platform,d.created_at,d.last_seen_at,
                EXISTS(SELECT 1 FROM fr_sessions s WHERE s.device_id=d.id AND s.digest=$2) AS current
         FROM fr_devices d WHERE d.user_id=$1 ORDER BY d.last_seen_at DESC`,
        [user.id, user.session],
      )
    ).rows;
    return rows.map((row: { current: boolean }) => ({ ...row, current: !!row.current }));
  }
  async registerDevice(headers: Headers, input: unknown) {
    const value = z
      .strictObject({
        device_key: z.uuid(),
        name: z.string().trim().min(1).max(120),
        platform: z.enum(["ios", "mac", "web", "desktop"]),
      })
      .parse(input);
    const user = await this.auth.authenticate(headers, true);
    // 同一 (user, device_key) 重复注册只刷新名称与活跃时间，不产生第二条设备。
    return (
      await this.pool.query(
        `INSERT INTO fr_devices(user_id,device_key,name,platform) VALUES($1,$2,$3,$4)
         ON CONFLICT(user_id,device_key)
         DO UPDATE SET name=EXCLUDED.name, platform=EXCLUDED.platform, last_seen_at=now()
         RETURNING id,name,platform,created_at,last_seen_at`,
        [user.id, value.device_key, value.name, value.platform],
      )
    ).rows[0];
  }
  async bindDeviceSession(headers: Headers, id: string) {
    const deviceId = z.uuid().parse(id);
    const user = await this.auth.authenticate(headers, true);
    const owned = (
      await this.pool.query("SELECT id FROM fr_devices WHERE id=$1 AND user_id=$2", [deviceId, user.id])
    ).rows[0];
    assert(owned, 404, "NOT_FOUND", "设备不存在");
    const bound = await this.pool.query(
      "UPDATE fr_sessions SET device_id=$2 WHERE digest=$1 AND user_id=$3 AND (device_id IS NULL OR device_id<>$2)",
      [user.session, deviceId, user.id],
    );
    return { bound: bound.rowCount === 1 };
  }
  async revokeDevice(headers: Headers, id: string) {
    const deviceId = z.uuid().parse(id);
    const user = await this.auth.authenticate(headers, true);
    // 级联删除该设备名下会话；若含当前会话，客户端下一次请求即 401。
    const revoked = await this.pool.query("DELETE FROM fr_devices WHERE id=$1 AND user_id=$2", [deviceId, user.id]);
    assert(revoked.rowCount === 1, 404, "NOT_FOUND", "设备不存在");
    return { revoked: true };
  }
  async members(headers: Headers, workspaceId: string, removeId?: string) {
    return transaction(this.pool, async (tx) => {
      const user = await this.auth.authenticate(headers, !!removeId, tx);
      await tx.query("SELECT id FROM fr_workspaces WHERE id=$1 FOR UPDATE", [
        workspaceId,
      ]);
      const role = (
        await tx.query(
          "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
          [workspaceId, user.id],
        )
      ).rows[0]?.role;
      assert(
        ["OWNER", "ADMIN"].includes(role),
        403,
        "PERMISSION_DENIED",
        "只有工作区管理员可管理成员",
      );
      if (removeId) {
        const target = (
          await tx.query(
            "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
            [workspaceId, removeId],
          )
        ).rows[0]?.role;
        assert(
          target &&
            target !== "OWNER" &&
            (target !== "ADMIN" || role === "OWNER"),
          403,
          "PERMISSION_DENIED",
          "不能移除所有者；管理员只由所有者管理",
        );
        await tx.query(
          "DELETE FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
          [workspaceId, removeId],
        );
        await tx.query(
          "INSERT INTO fr_acl_audit(user_id,action,allowed) VALUES($1,$2,true)",
          [user.id, "workspace.member.remove:" + workspaceId + ":" + removeId],
        );
        return { removed: true, project_grants_unchanged: true };
      }
      return (
        await tx.query(
          "SELECT m.user_id,m.role,u.email,u.display_name FROM fr_memberships m JOIN fr_users u ON u.id=m.user_id WHERE workspace_id=$1 ORDER BY m.role,u.email",
          [workspaceId],
        )
      ).rows;
    });
  }
  async projects(headers: Headers) {
    const user = await this.auth.authenticate(headers, false);
    return (
      await this.pool.query(
        "SELECT DISTINCT p.id,p.workspace_id,p.title,p.revision,p.state,s.mode AS sync_mode FROM fr_projects p LEFT JOIN fr_project_sync s ON s.project_id=p.id LEFT JOIN fr_memberships m ON m.workspace_id=p.workspace_id AND m.user_id=$1 LEFT JOIN fr_grants g ON g.project_id=p.id AND g.user_id=$1 AND g.active WHERE (p.owner_id=$1 OR m.role IN ('OWNER','ADMIN') OR g.user_id=$1) AND p.state IN ('STAGING','SHARED') AND (p.state='SHARED' OR p.owner_id=$1) AND (s.mode IS NULL OR s.mode='SHARED' OR p.owner_id=$1) ORDER BY p.id",
        [user.id],
      )
    ).rows;
  }
  async invite(headers: Headers, workspaceId: string, input: unknown) {
    const value = inviteInput.parse(input);
    return transaction(this.pool, async (tx) => {
      const user = await this.auth.authenticate(headers, true, tx);
      const workspace = (
        await tx.query("SELECT * FROM fr_workspaces WHERE id=$1 FOR UPDATE", [
          workspaceId,
        ])
      ).rows[0];
      assert(workspace, 404, "NOT_FOUND", "工作区不存在");
      const member = (
        await tx.query(
          "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
          [workspaceId, user.id],
        )
      ).rows[0];
      if (value.project_id) {
        const mode = (await tx.query('SELECT mode FROM fr_project_sync WHERE project_id=$1', [value.project_id])).rows[0]?.mode;
        assert(!mode || mode === 'SHARED', 409, 'PROJECT_PRIVATE', '请先由项目所有者显式开启共享');
        const project = (
          await tx.query(
            "SELECT * FROM fr_projects WHERE id=$1 AND workspace_id=$2 AND state='SHARED' FOR UPDATE",
            [value.project_id, workspaceId],
          )
        ).rows[0];
        assert(
          project &&
            (project.owner_id === user.id ||
              ["OWNER", "ADMIN"].includes(member?.role)) &&
            value.role !== "ADMIN",
          403,
          "PERMISSION_DENIED",
          "无权发送项目邀请",
        );
      } else
        assert(
          ["OWNER", "ADMIN"].includes(member?.role),
          403,
          "PERMISSION_DENIED",
          "只有工作区管理员可邀请成员",
        );
      const token = randomBytes(32).toString("hex"),
        id = randomUUID();
      await tx.query(
        "INSERT INTO fr_invitations(id,workspace_id,project_id,email,role,digest,expires_at,invited_by) VALUES($1,$2,$3,$4,$5,$6,now()+interval '7 days',$7)",
        [
          id,
          workspaceId,
          value.project_id ?? null,
          value.email,
          value.role,
          digest(token),
          user.id,
        ],
      );
      return {
        id,
        invite_url: `${this.auth.origin}/shared#invite=${token}`,
        expires_in: 604800,
        delivery: "MANUAL_LINK",
      };
    });
  }
  async acceptInvite(headers: Headers, input: unknown) {
    const { token } = z
      .strictObject({ token: z.string().regex(/^[a-f0-9]{64}$/) })
      .parse(input);
    return transaction(this.pool, async (tx) => {
      const user = await this.auth.authenticate(headers, true, tx);
      const pre = (
        await tx.query(
          "SELECT workspace_id FROM fr_invitations WHERE digest=$1",
          [digest(token)],
        )
      ).rows[0];
      assert(pre, 404, "NOT_FOUND", "邀请不存在");
      await tx.query("SELECT id FROM fr_workspaces WHERE id=$1 FOR UPDATE", [
        pre.workspace_id,
      ]);
      const invite = (
        await tx.query(
          "SELECT * FROM fr_invitations WHERE digest=$1 AND state='PENDING' AND expires_at>now() FOR UPDATE",
          [digest(token)],
        )
      ).rows[0];
      assert(
        invite && invite.email === user.email,
        403,
        "INVITE_INVALID",
        "邀请已使用、过期、撤销或与登录邮箱不匹配",
      );
      if (invite.project_id) {
        const mode = (await tx.query('SELECT mode FROM fr_project_sync WHERE project_id=$1', [invite.project_id])).rows[0]?.mode;
        assert(!mode || mode === 'SHARED', 409, 'PROJECT_PRIVATE', '项目已收回为私有');
        const project = (
          await tx.query(
            "SELECT id,owner_id FROM fr_projects WHERE id=$1 FOR UPDATE",
            [invite.project_id],
          )
        ).rows[0];
        assert(
          project.owner_id !== user.id,
          400,
          "VALIDATION_ERROR",
          "项目所有者无需接受自己的邀请",
        );
        await tx.query(
          "INSERT INTO fr_grants(project_id,user_id,role) VALUES($1,$2,$3) ON CONFLICT(project_id,user_id) DO UPDATE SET role=excluded.role,active=true,can_approve=false,can_edit_formal=false",
          [invite.project_id, user.id, invite.role],
        );
      } else {
        const existing = (
          await tx.query(
            "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
            [invite.workspace_id, user.id],
          )
        ).rows[0];
        assert(
          existing?.role !== "OWNER",
          400,
          "VALIDATION_ERROR",
          "不能替换工作区所有者",
        );
        await tx.query(
          "INSERT INTO fr_memberships VALUES($1,$2,$3) ON CONFLICT(workspace_id,user_id) DO UPDATE SET role=excluded.role",
          [invite.workspace_id, user.id, invite.role],
        );
      }
      await tx.query(
        "UPDATE fr_invitations SET state='ACCEPTED',accepted_by=$2 WHERE id=$1",
        [invite.id, user.id],
      );
      return {
        project_id: invite.project_id,
        workspace_id: invite.workspace_id,
        role: invite.role,
      };
    });
  }
  async revokeInvite(headers: Headers, workspaceId: string, id: string) {
    return transaction(this.pool, async (tx) => {
      const user = await this.auth.authenticate(headers, true, tx);
      await tx.query("SELECT id FROM fr_workspaces WHERE id=$1 FOR UPDATE", [
        workspaceId,
      ]);
      const invite = (
        await tx.query(
          "SELECT * FROM fr_invitations WHERE id=$1 AND workspace_id=$2 FOR UPDATE",
          [id, workspaceId],
        )
      ).rows[0];
      assert(invite, 404, "NOT_FOUND", "邀请不存在");
      const role = (
        await tx.query(
          "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
          [workspaceId, user.id],
        )
      ).rows[0]?.role;
      assert(
        invite.invited_by === user.id || ["OWNER", "ADMIN"].includes(role),
        403,
        "PERMISSION_DENIED",
        "无权撤销邀请",
      );
      assert(
        invite.state !== "ACCEPTED",
        409,
        "INVITE_ACCEPTED",
        "邀请已接受，请在项目权限中撤销访问",
      );
      await tx.query("UPDATE fr_invitations SET state='REVOKED' WHERE id=$1", [
        id,
      ]);
      return { revoked: true };
    });
  }
  async promote(headers: Headers, input: unknown, privateSync = false) {
    const value = promotionSchema.parse(input),
      hash = digest(JSON.stringify(privateSync ? { ...value, sync_mode: 'PRIVATE_SYNCED' } : value));
    return transaction(this.pool, async (tx) => {
      const user = await this.auth.authenticate(headers, true, tx);
      await tx.query("SELECT id FROM fr_workspaces WHERE id=$1 FOR SHARE", [
        value.workspace_id,
      ]);
      const role = (
        await tx.query(
          "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
          [value.workspace_id, user.id],
        )
      ).rows[0]?.role;
      assert(
        ["OWNER", "ADMIN", "EDITOR"].includes(role),
        403,
        "PERMISSION_DENIED",
        "无权向此工作区发布项目",
      );
      // 同一作者的迁移键用 advisory lock 串行化；重试不创建第二权威项目。
      await tx.query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))", [
        `${user.id}:${value.promotion_id}`,
      ]);
      const old = (
        await tx.query(
          "SELECT id,state,manifest_digest FROM fr_projects WHERE owner_id=$1 AND promotion_key=$2",
          [user.id, value.promotion_id],
        )
      ).rows[0];
      if (old) {
        assert(
          old.manifest_digest === hash,
          409,
          "IDEMPOTENCY_CONFLICT",
          "迁移键已绑定其他快照",
        );
        return { project_id: old.id, state: old.state, manifest_digest: hash };
      }
      const id = randomUUID();
      await tx.query(
        "INSERT INTO fr_projects(id,workspace_id,owner_id,title,state,revision,snapshot,promotion_key,manifest_digest,initial_revision) VALUES($1,$2,$3,$4,'STAGING',$5,$6,$7,$8,$5)",
        [
          id,
          value.workspace_id,
          user.id,
          value.title,
          value.from_revision,
          JSON.stringify(value.snapshot),
          value.promotion_id,
          hash,
        ],
      );
      if (privateSync) await tx.query("INSERT INTO fr_project_sync(project_id,mode) VALUES($1,'SYNC_ENROLLING')", [id]);
      return { project_id: id, state: "STAGING", manifest_digest: hash };
    });
  }
  async upload(headers: Headers, id: string, photoId: string, bytes: Buffer) {
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, project }) => {
        assert(
          project.owner_id === user.id && project.state === "STAGING",
          403,
          "PERMISSION_DENIED",
          "只能向自己迁移中的项目上传预览",
        );
        assert(
          project.snapshot.photos.some((p) => p.id === photoId),
          400,
          "MEDIA_POLICY",
          "照片未包含在已授权清单",
        );
        const preview = await sanitizePreview(bytes);
        const old = (
          await tx.query(
            "SELECT content_sha256 FROM fr_previews WHERE project_id=$1 AND photo_id=$2",
            [id, photoId],
          )
        ).rows[0];
        assert(
          !old || old.content_sha256 === preview.sha,
          409,
          "MEDIA_CONFLICT",
          "照片已绑定不同预览",
        );
        await tx.query('SELECT pg_advisory_xact_lock(hashtextextended($1,0))', ['media:' + preview.key]);
        await this.store.put(preview.key, preview.data);
        await tx.query(
          "INSERT INTO fr_previews VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING",
          [
            id,
            photoId,
            preview.key,
            preview.sha,
            preview.width,
            preview.height,
            preview.data.length,
          ],
        );
        // G1 FR3-010：同步写入资产域映射（source_asset + alias）
        // PG 事务内失败会 abort——用 SAVEPOINT 保护，失败回滚到保存点
        await tx.query('SAVEPOINT g1_asset_map');
        try {
          // 旧照片 ID 允许非 UUID；资产表使用独立 UUID，通过 alias 保持业务身份。
          const assetId = z.uuid().safeParse(photoId).success ? photoId : randomUUID();
          await tx.query(
            `INSERT INTO fr_source_assets(id, project_id, photo_id, content_sha256, file_size, width, height, format)
             VALUES($1, $2, $3, $4, $5, $6, $7, 'jpeg')
             ON CONFLICT (project_id, photo_id) DO NOTHING`,
            [assetId, id, photoId, preview.sha, preview.data.length, preview.width, preview.height],
          );
          const source = (await tx.query(
            'SELECT id FROM fr_source_assets WHERE project_id=$1 AND photo_id=$2',
            [id, photoId],
          )).rows[0];
          await tx.query(
            `INSERT INTO fr_asset_aliases(photo_id, asset_id, project_id) VALUES($1, $2, $3)
             ON CONFLICT (photo_id) DO NOTHING`,
            [photoId, source.id, id],
          );
          await tx.query('RELEASE SAVEPOINT g1_asset_map');
        } catch {
          await tx.query('ROLLBACK TO SAVEPOINT g1_asset_map');
        }
        return {
          photo_id: photoId,
          sha256: preview.sha,
          width: preview.width,
          height: preview.height,
          gps_stripped: true,
        };
      },
    );
  }
  async finalize(headers: Headers, id: string, input: unknown) {
    const { manifest_digest } = z
      .strictObject({ manifest_digest: z.string().regex(/^[a-f0-9]{64}$/) })
      .parse(input);
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, project }) => {
        assert(
          project.owner_id === user.id,
          403,
          "PERMISSION_DENIED",
          "只有迁移发起者可完成共享",
        );
        assert(
          project.state !== "ABORTED",
          409,
          "PROMOTION_ABORTED",
          "迁移已显式取消，不能提交",
        );
        const row = (
          await tx.query(
            "SELECT manifest_digest,promotion_key,initial_revision FROM fr_projects WHERE id=$1",
            [id],
          )
        ).rows[0];
        assert(
          row.manifest_digest === manifest_digest,
          409,
          "MANIFEST_MISMATCH",
          "快照校验值不匹配",
        );
        const uploaded = Number(
          (
            await tx.query(
              "SELECT count(*) FROM fr_previews WHERE project_id=$1",
              [id],
            )
          ).rows[0].count,
        );
        assert(
          uploaded === project.snapshot.photos.length,
          409,
          "MEDIA_INCOMPLETE",
          "预览尚未全部上传，可安全重试",
        );
        if (project.state === "STAGING") {
          await tx.query("UPDATE fr_projects SET state='SHARED' WHERE id=$1", [
            id,
          ]);
          await tx.query("UPDATE fr_project_sync SET mode='PRIVATE_SYNCED',updated_at=now() WHERE project_id=$1 AND mode='SYNC_ENROLLING'", [id]);
          await tx.query(
            "INSERT INTO fr_versions(project_id,revision,snapshot,actor_id,command_id) VALUES($1,$2,$3,$4,$5)",
            [
              id,
              project.revision,
              JSON.stringify(project.snapshot),
              user.id,
              `promotion:${row.promotion_key}`,
            ],
          );
          await tx.query(
            "INSERT INTO fr_events(project_id,actor_id,kind,revision,detail) VALUES($1,$2,'PromotionCommitted',$3,$4)",
            [
              id,
              user.id,
              project.revision,
              JSON.stringify({ manifest_digest }),
            ],
          );
        }
        return {
          project_id: id,
          workspace_id: project.workspace_id,
          authority: "CLOUD",
          promotion_id: row.promotion_key,
          from_revision: row.initial_revision,
          manifest_digest,
        };
      },
    );
  }
  async snapshot(headers: Headers, id: string) {
    return this.authority.scope(
      headers,
      id,
      false,
      async ({ tx, project, principal }) => {
        const comments = (
          await tx.query(
            "SELECT c.*,u.display_name AS reviewer FROM fr_comments c JOIN fr_users u ON u.id=c.author_id WHERE project_id=$1 ORDER BY c.created_at,c.id LIMIT 1000",
            [id],
          )
        ).rows;
        const proposals = (
          await tx.query(
            "SELECT p.*,u.display_name AS author FROM fr_proposals p JOIN fr_users u ON u.id=p.author_id WHERE project_id=$1 AND (p.state<>'DRAFT' OR p.author_id=$2) ORDER BY p.id LIMIT 1000",
            [id, principal.id],
          )
        ).rows;
        const previews = (
          await tx.query(
            "SELECT photo_id,width,height,content_sha256 FROM fr_previews WHERE project_id=$1 ORDER BY photo_id",
            [id],
          )
        ).rows;
        const grants = ["OWNER", "ADMIN"].includes(principal.role)
          ? (
              await tx.query(
                "SELECT g.*,u.email FROM fr_grants g JOIN fr_users u ON u.id=g.user_id WHERE project_id=$1",
                [id],
              )
            ).rows
          : [];
        const cursor = (
          await tx.query(
            "SELECT coalesce(max(id),0)::text AS cursor FROM fr_events WHERE project_id=$1",
            [id],
          )
        ).rows[0].cursor;
        return {
          project_id: id,
          workspace_id: project.workspace_id,
          title: project.title,
          state: project.state,
          revision: project.revision,
          snapshot: project.snapshot,
          principal,
          comments,
          proposals,
          previews,
          grants,
          cursor,
        };
      },
    );
  }
  // 持久事件游标：客户端持 snapshot.cursor 按 after=<cursor> 增量拉取，事件只追加不修改。
  async projectEvents(
    headers: Headers,
    id: string,
    after: unknown,
    limit: unknown,
  ) {
    const cursor = z.coerce.number().int().min(0).default(0).parse(after);
    const size = z.coerce.number().int().min(1).max(100).default(50).parse(limit);
    return this.authority.scope(headers, id, false, async ({ tx }) => {
      const rows = (
        await tx.query(
          "SELECT id,kind,revision,actor_id,detail,created_at FROM fr_events WHERE project_id=$1 AND id>$2 ORDER BY id LIMIT $3",
          [id, cursor, size],
        )
      ).rows;
      // BIGSERIAL 由 pg 以字符串返回；统一转为数字，客户端比较与续读游标才可靠。
      const events = rows.map((row: { id: string }) => ({ ...row, id: Number(row.id) }));
      const nextCursor = events.length === size ? events[events.length - 1].id : null;
      return { events, nextCursor };
    });
  }
  // 回执查询：客户端 UNKNOWN 结果先按原 commandId 查权威回执，命中即视为已提交，不得盲目重发。
  async commandReceipt(headers: Headers, id: string, commandId: unknown) {
    const command = z.uuid().parse(commandId);
    return this.authority.scope(headers, id, false, async ({ tx }) => {
      const row = (
        await tx.query(
          "SELECT command_id,actor_id,idempotency_key,reply,created_at FROM fr_commands WHERE project_id=$1 AND command_id=$2",
          [id, command],
        )
      ).rows[0];
      assert(row, 404, "NOT_FOUND", "命令回执不存在");
      return { command_id: row.command_id, actor_id: row.actor_id, idempotency_key: row.idempotency_key, reply: row.reply, created_at: row.created_at };
    });
  }
  async version(headers: Headers, id: string, revision: number) {
    assert(
      Number.isSafeInteger(revision) && revision >= 0,
      400,
      "VALIDATION_ERROR",
      "版本号不合法",
    );
    return this.authority.scope(headers, id, false, async ({ tx }) => {
      const row = (
        await tx.query(
          "SELECT project_id,revision,snapshot,actor_id,command_id,created_at FROM fr_versions WHERE project_id=$1 AND revision=$2",
          [id, revision],
        )
      ).rows[0];
      assert(row, 404, "NOT_FOUND", "历史版本不存在");
      return row;
    });
  }
  async versionDiff(headers: Headers, id: string, from: number, to: number) {
    assert(
      Number.isSafeInteger(from) && Number.isSafeInteger(to) &&
        from >= 0 && to >= 0 && from !== to,
      400, "VALIDATION_ERROR", "版本号不合法",
    );
    return this.authority.scope(headers, id, false, async ({ tx }) => {
      const rows = (
        await tx.query(
          "SELECT revision,snapshot,created_at FROM fr_versions WHERE project_id=$1 AND revision IN ($2,$3)",
          [id, from, to],
        )
      ).rows;
      const before = rows.find((row) => row.revision === from);
      const after = rows.find((row) => row.revision === to);
      assert(before && after, 404, "NOT_FOUND", "历史版本不存在");
      return {
        project_id: id,
        from: { revision: from, created_at: before.created_at },
        to: { revision: to, created_at: after.created_at },
        photos: diffEntities(snapshotEntities(before.snapshot.photos), snapshotEntities(after.snapshot.photos)),
        sequences: diffEntities(snapshotEntities(before.snapshot.sequences), snapshotEntities(after.snapshot.sequences)),
      };
    });
  }
  async researchList(headers: Headers, id: string) {
    return this.authority.scope(headers, id, false, async ({ tx }) => (
      await tx.query(
        `SELECT r.id,r.kind,r.title,r.body,r.url,r.author,r.source,r.license,r.created_by,u.display_name AS created_by_name,r.created_at,r.updated_at
         FROM fr_research_references r JOIN fr_users u ON u.id=r.created_by
         WHERE r.project_id=$1 ORDER BY r.created_at,r.id`,
        [id],
      )
    ).rows);
  }
  async researchCreate(headers: Headers, id: string, input: unknown) {
    const value = researchReferenceSchema.parse(input);
    return this.authority.scope(headers, id, true, async ({ tx, user, project, principal }) => {
      assert(project.state === "SHARED" && principal.role !== "VIEWER", 403, "PERMISSION_DENIED", "当前角色不能添加研究参考");
      const body = value.kind === "note" ? value.body : null;
      const url = value.kind === "link" ? value.url : null;
      return (await tx.query(
        `INSERT INTO fr_research_references(project_id,kind,title,body,url,author,source,license,created_by)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)
         RETURNING id,kind,title,body,url,author,source,license,created_by,created_at,updated_at`,
        [id, value.kind, value.title, body, url,
          value.author ?? null, value.source ?? null, value.license ?? null, user.id],
      )).rows[0];
    });
  }
  async researchUpdate(headers: Headers, id: string, referenceId: string, input: unknown) {
    const value = researchEditSchema.parse(input);
    return this.authority.scope(headers, id, true, async ({ tx, user, principal }) => {
      const current = (await tx.query(
        "SELECT created_by FROM fr_research_references WHERE id=$1 AND project_id=$2 FOR UPDATE",
        [referenceId, id],
      )).rows[0];
      assert(current, 404, "NOT_FOUND", "研究参考不存在");
      assert(current.created_by === user.id || principal.can_approve || principal.can_edit_formal,
        403, "PERMISSION_DENIED", "只能修改自己的研究参考，或由项目编辑处理");
      const sets: string[] = [];
      const values: unknown[] = [];
      const column = (name: string, fieldValue: unknown) => {
        values.push(fieldValue);
        sets.push(`${name}=$${values.length}`);
      };
      for (const field of ["title", "body", "url", "author", "source", "license"] as const) {
        if (value[field] !== undefined) column(field, value[field]);
      }
      assert(sets.length, 400, "VALIDATION_ERROR", "更新操作不包含任何字段");
      values.push(referenceId, id);
      return (await tx.query(
        `UPDATE fr_research_references SET ${sets.join(",")},updated_at=now() WHERE id=$${values.length - 1} AND project_id=$${values.length} RETURNING id,kind,title,body,url,author,source,license,created_by,created_at,updated_at`,
        values,
      )).rows[0];
    });
  }
  async researchDelete(headers: Headers, id: string, referenceId: string) {
    return this.authority.scope(headers, id, true, async ({ tx, user, principal }) => {
      const current = (await tx.query(
        "SELECT created_by FROM fr_research_references WHERE id=$1 AND project_id=$2 FOR UPDATE",
        [referenceId, id],
      )).rows[0];
      assert(current, 404, "NOT_FOUND", "研究参考不存在");
      assert(current.created_by === user.id || principal.can_approve || principal.can_edit_formal,
        403, "PERMISSION_DENIED", "只能删除自己的研究参考，或由项目编辑处理");
      await tx.query("DELETE FROM fr_research_references WHERE id=$1 AND project_id=$2", [referenceId, id]);
      return { deleted: true, id: referenceId };
    });
  }
  async proposalDiff(headers: Headers, id: string, proposalId: string) {
    return this.authority.scope(headers, id, false, async ({ tx, user, project }) => {
      const row = (await tx.query(
        "SELECT state,base_revision,author_id,operations FROM fr_proposals WHERE id=$1 AND project_id=$2",
        [proposalId, id],
      )).rows[0];
      assert(row, 404, "NOT_FOUND", "提案不存在");
      // 未提交草案仅作者可见，与快照中的提案可见性一致。
      assert(row.state !== "DRAFT" || row.author_id === user.id, 404, "NOT_FOUND", "提案不存在");
      const sequence = project.snapshot.sequences[0];
      assert(sequence && Array.isArray(sequence.items), 500, "SNAPSHOT_INVALID", "版本快照结构不合法");
      if (row.base_revision !== project.revision) {
        return { proposalId, state: row.state, baseRevision: row.base_revision, currentRevision: project.revision, stale: true, sequence: null };
      }
      const ops = proposalOperations(row.operations);
      // 投影复用 apply 的顺序语义（逐操作 splice），预览本身不落任何状态。
      const items = structuredClone(sequence.items) as { id: string; photo_id: string }[];
      for (const op of ops) {
        const index = items.findIndex((item) => item.photo_id === op.photo_id);
        if (op.op === "add") {
          assert(index < 0 && op.to_index! <= items.length, 409, "REVISION_CONFLICT", "新增照片已存在或位置越界");
          items.splice(op.to_index!, 0, { id: `${proposalId}:${op.operation_id}`, photo_id: op.photo_id });
        } else if (op.op === "remove") {
          assert(index >= 0, 409, "REVISION_CONFLICT", "移除照片已不存在");
          items.splice(index, 1);
        } else {
          assert(index >= 0 && op.to_index! < items.length, 409, "REVISION_CONFLICT", "重排照片已变化");
          const [moved] = items.splice(index, 1);
          items.splice(op.to_index!, 0, moved);
        }
      }
      const before = (sequence.items as { id: string; photo_id: string }[]).map((item) => item.photo_id);
      const after = items.map((item) => item.photo_id);
      const moved = after
        .map((photo_id, toIndex) => ({ photo_id, toIndex }))
        .filter(({ photo_id, toIndex }) => before.includes(photo_id) && before.indexOf(photo_id) !== toIndex)
        .map(({ photo_id, toIndex }) => ({ photo_id, fromIndex: before.indexOf(photo_id), toIndex }));
      return {
        proposalId, state: row.state, baseRevision: row.base_revision, currentRevision: project.revision, stale: false,
        sequence: {
          id: sequence.id,
          before, after,
          added: after.filter((photo_id) => !before.includes(photo_id)),
          removed: before.filter((photo_id) => !after.includes(photo_id)),
          moved,
        },
      };
    });
  }
  async reviseProposal(
    headers: Headers,
    id: string,
    proposalId: string,
    input: unknown,
  ) {
    const value = z
      .strictObject({
        idempotency_key: z.uuid(),
        expected_proposal_revision: z.number().int().min(0),
        base_revision: z.number().int().min(0),
        title: z.string().trim().min(1).max(200),
        operations: z.unknown(),
      })
      .parse(input);
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, project, principal }) => {
        assert(
          project.state === "SHARED" && principal.role !== "VIEWER",
          403,
          "PERMISSION_DENIED",
          "当前角色不能修订提案",
        );
        const proposal = (
          await tx.query(
            "SELECT * FROM fr_proposals WHERE id=$1 AND project_id=$2 FOR UPDATE",
            [proposalId, id],
          )
        ).rows[0];
        assert(
          proposal && proposal.author_id === user.id,
          403,
          "PERMISSION_DENIED",
          "只能修订自己的未提交提案",
        );
        const hash = digest(
          JSON.stringify({ proposal_id: proposalId, ...value }),
        );
        const replay = (
          await tx.query(
            "SELECT actor_id,payload_digest,reply FROM fr_commands WHERE project_id=$1 AND command_id=$2",
            [id, value.idempotency_key],
          )
        ).rows[0];
        if (replay) {
          assert(
            replay.actor_id === user.id && replay.payload_digest === hash,
            409,
            "IDEMPOTENCY_CONFLICT",
            "修订幂等键已绑定其他请求",
          );
          return replay.reply;
        }
        assert(
          proposal.state === "DRAFT" &&
            proposal.revision === value.expected_proposal_revision &&
            project.revision === value.base_revision,
          409,
          "REVISION_CONFLICT",
          "提案已提交、基线或草案版本变化；原提案保留",
        );
        const parsed = parseCloudCommand({
          version: "2.0",
          command_id: value.idempotency_key,
          idempotency_key: value.idempotency_key,
          workspace_id: project.workspace_id,
          project_id: id,
          expected_revision: value.base_revision,
          kind: "proposal.create",
          payload: {
            base_revision: value.base_revision,
            title: value.title,
            operations: value.operations,
          },
        });
        assert(
          parsed.kind === "proposal.create",
          400,
          "VALIDATION_ERROR",
          "提案格式不合法",
        );
        assert(
          parsed.payload.operations.every((op) =>
            project.snapshot.photos.some((p) => p.id === op.photo_id),
          ),
          400,
          "VALIDATION_ERROR",
          "提案含有其他项目的照片",
        );
        const nextRevision = proposal.revision + 1;
        await tx.query(
          "UPDATE fr_proposals SET title=$2,operations=$3,base_revision=$4,revision=$5,nonce=$6 WHERE id=$1",
          [
            proposalId,
            value.title,
            JSON.stringify(parsed.payload.operations),
            value.base_revision,
            nextRevision,
            randomUUID(),
          ],
        );
        await this.event({ tx, user }, id, "proposal.revised", {
          proposal_id: proposalId,
          previous: {
            title: proposal.title,
            operations: proposal.operations,
            base_revision: proposal.base_revision,
            revision: proposal.revision,
          },
          new_revision: nextRevision,
        });
        const reply = {
          proposal_id: proposalId,
          proposal_revision: nextRevision,
        };
        await tx.query(
          "INSERT INTO fr_commands(project_id,command_id,actor_id,idempotency_key,payload_digest,envelope,reply) VALUES($1,$2,$3,$2,$4,$5,$6)",
          [
            id,
            value.idempotency_key,
            user.id,
            hash,
            JSON.stringify({
              protocol: "cloud.proposal.revise.v1",
              project_id: id,
              proposal_id: proposalId,
              ...value,
            }),
            JSON.stringify(reply),
          ],
        );
        return reply;
      },
    );
  }
  async abort(headers: Headers, id: string, input: unknown) {
    const { manifest_digest } = z
      .strictObject({ manifest_digest: z.string().regex(/^[a-f0-9]{64}$/) })
      .parse(input);
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, project }) => {
        const row = (
          await tx.query(
            "SELECT manifest_digest FROM fr_projects WHERE id=$1",
            [id],
          )
        ).rows[0];
        assert(
          project.owner_id === user.id &&
            row.manifest_digest === manifest_digest,
          403,
          "PERMISSION_DENIED",
          "迁移身份不匹配",
        );
        assert(
          project.state !== "SHARED",
          409,
          "AUTHORITY_MISMATCH",
          "云端已提交，不能恢复本地写入；请恢复迁移确认",
        );
        await tx.query("UPDATE fr_projects SET state='ABORTED' WHERE id=$1", [
          id,
        ]);
        await tx.query("UPDATE fr_project_sync SET mode='ENROLL_FAILED',updated_at=now() WHERE project_id=$1 AND mode='SYNC_ENROLLING'", [id]);
        return { aborted: true, manifest_digest };
      },
    );
  }
  async grants(headers: Headers, id: string, target: string, input?: unknown) {
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, principal, user }) => {
        assert(
          ["OWNER", "ADMIN"].includes(principal.role),
          403,
          "PERMISSION_DENIED",
          "只有所有者或管理员可修改项目权限",
        );
        const existing = (
          await tx.query(
            "SELECT * FROM fr_grants WHERE project_id=$1 AND user_id=$2",
            [id, target],
          )
        ).rows[0];
        assert(existing, 404, "NOT_FOUND", "项目授权不存在");
        if (input) {
          const value = z
            .strictObject({
              role: z.enum(["EDITOR", "REVIEWER", "VIEWER"]),
              can_approve: z.boolean(),
              can_edit_formal: z.boolean(),
            })
            .parse(input);
          assert(
            value.role === "EDITOR" ||
              (!value.can_approve && !value.can_edit_formal),
            400,
            "VALIDATION_ERROR",
            "只有编辑可获得正式编辑或审批权限",
          );
          await tx.query(
            "UPDATE fr_grants SET role=$3,can_approve=$4,can_edit_formal=$5,active=true WHERE project_id=$1 AND user_id=$2",
            [id, target, value.role, value.can_approve, value.can_edit_formal],
          );
        } else
          await tx.query(
            "UPDATE fr_grants SET active=false WHERE project_id=$1 AND user_id=$2",
            [id, target],
          );
        await this.event({ tx, user }, id, "ProjectGrantChanged", {
          user_id: target,
          previous: existing,
          next: input ?? { active: false },
        });
        return { updated: true };
      },
    );
  }
  async ticket(headers: Headers, id: string, photoId?: string) {
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, project }) => {
        assert(
          project.state === "SHARED",
          409,
          "AUTHORITY_MISMATCH",
          "迁移尚未完成",
        );
        if (photoId)
          assert(
            project.snapshot.photos.some((p) => p.id === photoId),
            404,
            "NOT_FOUND",
            "照片不存在",
          );
        const token = randomBytes(32).toString("hex");
        await tx.query("DELETE FROM fr_tickets WHERE expires_at<now()");
        await tx.query(
          "INSERT INTO fr_tickets VALUES($1,$2,$3,$4,$5,now()+interval '30 seconds')",
          [
            digest(token),
            user.session,
            id,
            photoId ?? null,
            photoId ? "PREVIEW" : "WS",
          ],
        );
        return {
          token,
          expires_in: 30,
          url: photoId ? `/api/v2/media/${token}` : undefined,
        };
      },
    );
  }
  async useTicket(token: string, purpose: "WS" | "PREVIEW") {
    assert(/^[a-f0-9]{64}$/.test(token), 401, "TICKET_INVALID", "访问票据无效");
    return transaction(this.pool, async (tx) => {
      const row = (
        await tx.query(
          "DELETE FROM fr_tickets WHERE digest=$1 AND purpose=$2 AND expires_at>now() RETURNING *",
          [digest(token), purpose],
        )
      ).rows[0];
      assert(row, 401, "TICKET_INVALID", "访问票据已使用或过期");
      const user = await this.identityForSession(row.session_digest, tx);
      const project = (
        await tx.query(
          "SELECT * FROM fr_projects WHERE id=$1 AND state='SHARED'",
          [row.project_id],
        )
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      await this.authority.principal(tx, user, project);
      return { user, project, photoId: row.photo_id as string | null };
    });
  }
  async identityForSession(
    session: string,
    tx = this.pool as Pick<Pool, "query">,
  ) {
    const row = (
      await tx.query(
        "SELECT u.id,u.email,u.display_name AS name,u.avatar_url,u.email_verified_at,s.csrf,s.digest AS session FROM fr_sessions s JOIN fr_users u ON u.id=s.user_id WHERE s.digest=$1 AND s.expires_at>now() AND NOT EXISTS(SELECT 1 FROM fr_account_closures c WHERE c.user_id=u.id) FOR SHARE OF s,u",
        [session],
      )
    ).rows[0];
    assert(row, 401, "AUTH_REVOKED", "会话已失效");
    return row;
  }
  async media(token: string) {
    const { user, project, photoId } = await this.useTicket(token, "PREVIEW");
    const preview = (
      await this.pool.query(
        "SELECT * FROM fr_previews WHERE project_id=$1 AND photo_id=$2",
        [project.id, photoId],
      )
    ).rows[0];
    assert(preview, 404, "NOT_FOUND", "预览不存在");
    const bytes = await this.store.get(preview.object_key);
    assert(
      digest(bytes) === preview.content_sha256,
      500,
      "MEDIA_INTEGRITY",
      "预览完整性检查失败",
    );
    // IO 后再验当前 session/ACL；票据不绕过撤权。
    const live = await this.identityForSession(user.session);
    await this.authority.principal(this.pool, live, project);
    return bytes;
  }
  async reply(headers: Headers, id: string, thread: string, input: unknown) {
    const { body, idempotency_key } = z
      .strictObject({
        body: z.string().trim().min(1).max(5000),
        idempotency_key: z.uuid(),
      })
      .parse(input);
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, principal }) => {
        assert(
          principal.role !== "VIEWER",
          403,
          "PERMISSION_DENIED",
          "只读角色不能评论",
        );
        const payloadHash = digest(JSON.stringify({ body, thread }));
        const replay = (
          await tx.query(
            "SELECT actor_id,payload_digest,reply FROM fr_commands WHERE project_id=$1 AND command_id=$2",
            [id, idempotency_key],
          )
        ).rows[0];
        if (replay) {
          assert(
            replay.actor_id === user.id &&
              replay.payload_digest === payloadHash,
            409,
            "IDEMPOTENCY_CONFLICT",
            "回复幂等键已绑定其他请求",
          );
          return replay.reply;
        }
        const parent = (
          await tx.query(
            "SELECT id,anchor,parent_id FROM fr_comments WHERE id=$1 AND project_id=$2",
            [thread, id],
          )
        ).rows[0];
        assert(
          parent && !parent.parent_id,
          400,
          "VALIDATION_ERROR",
          "回复必须属于本项目的根评论",
        );
        const cid = idempotency_key;
        await tx.query(
          "INSERT INTO fr_comments(id,project_id,author_id,body,anchor,parent_id) VALUES($1,$2,$3,$4,$5,$6)",
          [cid, id, user.id, body, JSON.stringify(parent.anchor), thread],
        );
        await this.event({ tx, user }, id, "comment.reply", {
          comment_id: cid,
        });
        const reply = { id: cid };
        await tx.query(
          "INSERT INTO fr_commands(project_id,command_id,actor_id,idempotency_key,payload_digest,envelope,reply) VALUES($1,$2,$3,$2,$4,$5,$6)",
          [
            id,
            cid,
            user.id,
            payloadHash,
            JSON.stringify({
              kind: "comment.reply",
              thread_id: thread,
              body,
              idempotency_key,
            }),
            JSON.stringify(reply),
          ],
        );
        return reply;
      },
    );
  }
  async event(
    scope: Pick<Scope, "tx" | "user">,
    id: string,
    kind: string,
    detail: unknown,
  ) {
    await scope.tx.query(
      "INSERT INTO fr_events(project_id,actor_id,kind,revision,detail) SELECT id,$2,$3,revision,$4 FROM fr_projects WHERE id=$1",
      [id, scope.user.id, kind, JSON.stringify(detail)],
    );
  }
  async aiComments(headers: Headers, id: string) {
    // 这是本地授权取材端点，不做云模型调用；共享授权绝不兼作图像或作者记忆授权。
    return this.authority.scope(headers, id, false, async ({ tx }) => ({
      comments: (
        await tx.query(
          "SELECT c.id,c.author_id,u.display_name AS reviewer,c.body,c.anchor,c.parent_id FROM fr_comments c JOIN fr_users u ON u.id=c.author_id WHERE c.project_id=$1 AND c.state<>'RESOLVED' ORDER BY c.created_at LIMIT 100",
          [id],
        )
      ).rows,
      provenance: "HUMAN_PROJECT_COMMENTS",
      grants_model_upload: false,
      creates_author_memory: false,
    }));
  }
  async issueAgentScope(headers: Headers, id: string) {
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, project }) => {
        assert(
          project.state === "SHARED",
          409,
          "AUTHORITY_MISMATCH",
          "迁移尚未完成",
        );
        const token = randomBytes(32).toString("hex"),
          scopeId = randomUUID();
        await tx.query(
          "INSERT INTO fr_agent_scopes(id,digest,project_id,session_digest,issued_by,expires_at) VALUES($1,$2,$3,$4,$5,now()+interval '5 minutes')",
          [scopeId, digest(token), id, user.session, user.id],
        );
        return {
          id: scopeId,
          task_token: token,
          expires_in: 300,
          scope: ["project.comments.read"],
          permits_model_upload: false,
        };
      },
    );
  }
  async revokeAgentScope(headers: Headers, id: string, scopeId: string) {
    return this.authority.scope(
      headers,
      id,
      true,
      async ({ tx, user, principal }) => {
        const result = await tx.query(
          "UPDATE fr_agent_scopes SET revoked=true WHERE id=$1 AND project_id=$2 AND (issued_by=$3 OR $4)",
          [scopeId, id, user.id, ["OWNER", "ADMIN"].includes(principal.role)],
        );
        assert(result.rowCount, 403, "PERMISSION_DENIED", "无权撤销此任务授权");
        return { revoked: true };
      },
    );
  }
  async requestMagicLogin(input: unknown, ip: string) {
    const email = String((input as Record<string, unknown>)?.email ?? "");
    return requestMagicCode(this.pool, email, ip, this.auth.origin, this.magicEmail);
  }

  async updateProfile(user: { id: string }, input: unknown) {
    const body = (input ?? {}) as Record<string, unknown>;
    const name = typeof body.display_name === "string" ? body.display_name.trim().slice(0, 200) : undefined;
    const avatar = typeof body.avatar_url === "string" ? body.avatar_url.slice(0, 400_000) : undefined;
    if (name !== undefined) assert(name.length > 0, 400, "VALIDATION_ERROR", "昵称不能为空");
    if (avatar !== undefined) {
      const ok = avatar === "" || avatar.startsWith("data:image/") || avatar.startsWith("https://");
      assert(ok, 400, "VALIDATION_ERROR", "头像格式不支持");
    }
    return transaction(this.pool, async tx => {
      await tx.query('SELECT id FROM fr_users WHERE id=$1 FOR UPDATE', [user.id]);
      const closed = (await tx.query('SELECT 1 FROM fr_account_closures WHERE user_id=$1', [user.id])).rowCount;
      assert(!closed, 401, 'AUTH_REVOKED', '账号已关闭');
      const row = (await tx.query(
        "UPDATE fr_users SET display_name=COALESCE($2,display_name), avatar_url=COALESCE($3,avatar_url) WHERE id=$1 RETURNING id,email,display_name AS name,avatar_url,email_verified_at",
        [user.id, name ?? null, avatar ?? null],
      )).rows[0];
      assert(row, 401, 'AUTH_REVOKED', '账号不可访问');
      return { id: row.id, email: row.email, name: row.name, avatar: row.avatar_url ?? null, emailVerified: !!row.email_verified_at };
    });
  }

  async googleLogin(input: unknown, ip: string) {
    return loginWithGoogle(this.pool, input, ip);
  }

  async verifyMagicLogin(input: unknown, ip: string) {
    const email = String((input as Record<string, unknown>)?.email ?? "");
    const code = String((input as Record<string, unknown>)?.code ?? "");
    const result = await verifyMagicCode(this.pool, email, code, ip);
    assert(result, 400, "VALIDATION_ERROR", "验证码错误或已过期");
    return result;
  }

  async agentComments(headers: Headers) {
    const token = headers.authorization?.match(/^Task ([a-f0-9]{64})$/)?.[1];
    assert(token, 401, "UNAUTHORIZED", "需要短期任务身份");
    return transaction(this.pool, async (tx) => {
      const scope = (
        await tx.query(
          "SELECT * FROM fr_agent_scopes WHERE digest=$1 AND NOT revoked AND expires_at>now() FOR SHARE",
          [digest(token)],
        )
      ).rows[0];
      assert(scope, 401, "AUTH_REVOKED", "任务身份已过期或撤销");
      const user = await this.identityForSession(scope.session_digest, tx);
      const project = (
        await tx.query(
          "SELECT * FROM fr_projects WHERE id=$1 AND state='SHARED' FOR SHARE",
          [scope.project_id],
        )
      ).rows[0];
      assert(project, 403, "PERMISSION_DENIED", "项目不可访问");
      await this.authority.principal(tx, user, project);
      const comments = (
        await tx.query(
          "SELECT c.id,c.author_id,u.display_name AS reviewer,c.body,c.anchor,c.parent_id FROM fr_comments c JOIN fr_users u ON u.id=c.author_id WHERE c.project_id=$1 AND c.state<>'RESOLVED' ORDER BY c.created_at LIMIT 100",
          [project.id],
        )
      ).rows;
      await tx.query(
        "INSERT INTO fr_acl_audit(user_id,project_id,action,allowed) VALUES($1,$2,'agent.comments.read',true)",
        [user.id, project.id],
      );
      return {
        task_scope_id: scope.id,
        project_id: project.id,
        comments,
        provenance: "HUMAN_PROJECT_COMMENTS",
        permits_model_upload: false,
        creates_author_memory: false,
      };
    });
  }

  /** 独立模型发送授权：项目/管理员权限不是同意，须显式创建并可在任务前撤回。 */
  async modelConsentList(headers: Headers, projectId: string) {
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, false, tx);
      await this.authority.principal(tx, user, project);
      const consents = (
        await tx.query(
          `SELECT c.id, c.scope, c.endpoint_label, c.model_id, c.max_tasks, c.used, c.state,
                  c.granted_at, c.revoked_at, u.display_name AS granted_by_name
           FROM fr_model_consents c JOIN fr_users u ON u.id=c.granted_by
           WHERE c.project_id=$1 ORDER BY c.granted_at DESC`,
          [projectId],
        )
      ).rows;
      return { consents, endpoint_label: modelEndpointLabel() };
    });
  }

  async modelConsentGrant(headers: Headers, projectId: string, body: unknown) {
    const input = z.strictObject({
      scope: z.enum(["summarize_comments", "summarize_disagreement"]),
      max_tasks: z.number().int().min(1).max(100),
    }).parse(body);
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1 FOR SHARE", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, true, tx);
      const principal = await this.authority.principal(tx, user, project);
      assert(
        principal.role === "OWNER" || principal.role === "ADMIN",
        403, "PERMISSION_DENIED", "只有所有者或管理员可以授予模型发送",
      );
      const id = randomUUID();
      const endpoint = modelEndpointLabel();
      await tx.query(
        `INSERT INTO fr_model_consents(id, project_id, granted_by, endpoint_label, model_id, scope, max_tasks)
         VALUES ($1,$2,$3,$4,$5,$6,$7)`,
        [id, projectId, user.id, endpoint, endpoint.split("@")[0], input.scope, input.max_tasks],
      );
      return { consent_id: id, endpoint_label: endpoint, scope: input.scope, max_tasks: input.max_tasks };
    });
  }

  async modelConsentRevoke(headers: Headers, projectId: string, consentId: string) {
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1 FOR SHARE", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, true, tx);
      const principal = await this.authority.principal(tx, user, project);
      const consent = (
        await tx.query("SELECT * FROM fr_model_consents WHERE id=$1 AND project_id=$2 FOR UPDATE", [consentId, projectId])
      ).rows[0];
      assert(consent, 404, "NOT_FOUND", "授权不存在");
      assert(
        principal.role === "OWNER" || principal.role === "ADMIN" || consent.granted_by === user.id,
        403, "PERMISSION_DENIED", "只能撤回自己授予或由管理员撤回",
      );
      await tx.query(
        "UPDATE fr_model_consents SET state='REVOKED', revoked_at=now() WHERE id=$1 AND state='ACTIVE'",
        [consentId],
      );
      const cancelled = await tx.query(
        `UPDATE fr_agent_jobs SET status='CANCELLED', error='授权已撤回', updated_at=now()
         WHERE consent_id=$1 AND status IN ('QUEUED','RUNNING') RETURNING id`,
        [consentId],
      );
      return { revoked: true, cancelled_jobs: cancelled.rows.length };
    });
  }

  async agentJobList(headers: Headers, projectId: string) {
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, false, tx);
      await this.authority.principal(tx, user, project);
      const jobs = (
        await tx.query(
          `SELECT j.id, j.kind, j.status, j.attempts, j.error, j.result_id, j.created_at, j.updated_at,
                  u.display_name AS created_by_name
           FROM fr_agent_jobs j JOIN fr_users u ON u.id=j.created_by
           WHERE j.project_id=$1 ORDER BY j.created_at DESC LIMIT 50`,
          [projectId],
        )
      ).rows;
      return { jobs };
    });
  }

  async agentSummarize(headers: Headers, projectId: string, body: unknown) {
    const input = z.strictObject({
      kind: z.enum(["summarize_comments", "summarize_disagreement"]).default("summarize_comments"),
      consent_id: z.string().min(1).max(160),
    }).parse(body);
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1 FOR SHARE", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, true, tx);
      const principal = await this.authority.principal(tx, user, project);
      assert(
        principal.role === "OWNER" || principal.role === "ADMIN",
        403, "PERMISSION_DENIED", "只有所有者或管理员可以触发 AI 汇总",
      );
      // 项目/管理员权限不是向模型发送评论的同意；必须绑定显式授权。
      const consent = (
        await tx.query(
          "SELECT * FROM fr_model_consents WHERE id=$1 AND project_id=$2 AND scope=$3 FOR UPDATE",
          [input.consent_id, projectId, input.kind],
        )
      ).rows[0];
      if (!consent || consent.state !== "ACTIVE" || consent.used >= consent.max_tasks)
        throw new ApiError(403, "MODEL_CONSENT_REQUIRED", "缺少有效模型发送授权；请先创建或更换未用尽的授权");
      const job = randomUUID();
      await tx.query(
        `INSERT INTO fr_agent_jobs(id, project_id, consent_id, created_by, kind)
         VALUES ($1,$2,$3,$4,$5)`,
        [job, projectId, consent.id, user.id, input.kind],
      );
      return { job_id: job, status: "QUEUED", kind: input.kind };
    });
  }

  async agentJobCancel(headers: Headers, projectId: string, jobId: string) {
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1 FOR SHARE", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, true, tx);
      const principal = await this.authority.principal(tx, user, project);
      const job = (
        await tx.query("SELECT * FROM fr_agent_jobs WHERE id=$1 AND project_id=$2 FOR UPDATE", [jobId, projectId])
      ).rows[0];
      assert(job, 404, "NOT_FOUND", "任务不存在");
      assert(
        job.created_by === user.id || principal.role === "OWNER" || principal.role === "ADMIN",
        403, "PERMISSION_DENIED", "只能取消自己创建或由管理员取消",
      );
      assert(
        job.status === "QUEUED" || job.status === "RUNNING",
        409, "REVISION_CONFLICT", "任务已结束，不能取消",
      );
      await tx.query(
        "UPDATE fr_agent_jobs SET status='CANCELLED', updated_at=now() WHERE id=$1",
        [jobId],
      );
      return { cancelled: true };
    });
  }

  async agentTaskList(headers: Headers, projectId: string) {
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT * FROM fr_projects WHERE id=$1", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, false, tx);
      await this.authority.principal(tx, user, project);
      return { tasks: await listTaskResults(this.pool, projectId) };
    });
  }
}
