import { sendPasswordResetEmail, sendVerificationEmail } from "./email";
import { listTaskResults, storeTaskResult, summarizeProjectComments } from "./agent-tasks";
import { randomBytes, randomUUID, scrypt } from "node:crypto";
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
import { assert } from "./errors.ts";
import { sanitizePreview, type PreviewStore } from "./storage.ts";
const inviteInput = z.strictObject({
  email: z
    .email()
    .max(254)
    .transform((s) => s.toLowerCase()),
  role: z.enum(["ADMIN", "EDITOR", "REVIEWER", "VIEWER"]),
  project_id: z.string().min(1).max(160).optional(),
});
export class CloudService {
  readonly pool: Pool;
  readonly auth: AuthService;
  readonly authority: AuthorityService;
  readonly store: PreviewStore;
  constructor(pool: Pool, origin: string, store: PreviewStore) {
    this.pool = pool;
    this.auth = new AuthService(pool, origin);
    this.authority = new AuthorityService(pool, this.auth);
    this.store = store;
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
        "SELECT DISTINCT p.id,p.workspace_id,p.title,p.revision,p.state FROM fr_projects p LEFT JOIN fr_memberships m ON m.workspace_id=p.workspace_id AND m.user_id=$1 LEFT JOIN fr_grants g ON g.project_id=p.id AND g.user_id=$1 AND g.active WHERE (p.owner_id=$1 OR m.role IN ('OWNER','ADMIN') OR g.user_id=$1) AND p.state IN ('STAGING','SHARED') AND (p.state='SHARED' OR p.owner_id=$1) ORDER BY p.id",
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
  async promote(headers: Headers, input: unknown) {
    const value = promotionSchema.parse(input),
      hash = digest(JSON.stringify(value));
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
        "SELECT u.id,u.email,u.display_name AS name,s.csrf,s.digest AS session FROM fr_sessions s JOIN fr_users u ON u.id=s.user_id WHERE s.digest=$1 AND s.expires_at>now() FOR SHARE OF s",
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

  async verifyEmail(input: unknown) {
    const token = String((input as Record<string, unknown>)?.token ?? "");
    assert(token.length === 64, 400, "VALIDATION_ERROR", "验证令牌不合法");
    return transaction(this.pool, async (tx) => {
      const row = (
        await tx.query(
          "SELECT * FROM fr_email_verify_tokens WHERE digest=$1 AND used_at IS NULL AND expires_at>now() FOR UPDATE",
          [digest(token)],
        )
      ).rows[0];
      assert(row, 400, "VALIDATION_ERROR", "令牌无效或已过期");
      await tx.query("UPDATE fr_users SET email_verified_at=now() WHERE id=$1", [row.user_id]);
      await tx.query("UPDATE fr_email_verify_tokens SET used_at=now() WHERE digest=$1", [digest(token)]);
      return { verified: true };
    });
  }

  async resendVerification(headers: Headers) {
    return transaction(this.pool, async (tx) => {
      const user = await this.auth.authenticate(headers, true, tx);
      const already = (
        await tx.query("SELECT email_verified_at FROM fr_users WHERE id=$1", [user.id])
      ).rows[0];
      if (already?.email_verified_at) return { already_verified: true };
      const recent = (
        await tx.query(
          "SELECT COUNT(*) FROM fr_email_verify_tokens WHERE user_id=$1 AND created_at>now()-interval '10 minutes'",
          [user.id],
        )
      ).rows[0].count;
      assert(Number(recent) < 3, 429, "RATE_LIMITED", "重发过于频繁，请 10 分钟后再试");
      const raw = randomBytes(32).toString("hex");
      await tx.query(
        "INSERT INTO fr_email_verify_tokens(digest,user_id,expires_at) VALUES($1,$2,now()+interval '48 hours')",
        [digest(raw), user.id],
      );
      await sendVerificationEmail(user.email, raw, process.env.FOCALROOM_WEB_ORIGIN ?? "http://127.0.0.1:8084");
      return { sent: true };
    });
  }

  async forgotPassword(input: unknown, ip: string) {
    const email = String((input as Record<string, unknown>)?.email ?? "");
    if (!email.includes("@")) return { sent: true }; // 不暴露邮箱是否存在
    return transaction(this.pool, async (tx) => {
      const user = (
        await tx.query("SELECT id, email FROM fr_users WHERE email=$1", [email])
      ).rows[0];
      if (!user) return { sent: true }; // 同上
      // 每邮箱每小时最多 3 次。
      const recent = (
        await tx.query(
          "SELECT COUNT(*) FROM fr_password_reset_tokens WHERE user_id=$1 AND created_at>now()-interval '1 hour'",
          [user.id],
        )
      ).rows[0].count;
      assert(Number(recent) < 3, 429, "RATE_LIMITED", "重置请求过于频繁，请稍后再试");
      const raw = randomBytes(32).toString("hex");
      const tokenDigest = digest(raw);
      await tx.query(
        "INSERT INTO fr_password_reset_tokens(digest,user_id,expires_at,ip) VALUES($1,$2,now()+interval '30 minutes',$3)",
        [tokenDigest, user.id, ip],
      );
      await sendPasswordResetEmail(email, raw, process.env.FOCALROOM_WEB_ORIGIN ?? "http://127.0.0.1:8084");
      return { sent: true };
    });
  }

  async resetPassword(input: unknown) {
    const token = String((input as Record<string, unknown>)?.token ?? "");
    const password = String((input as Record<string, unknown>)?.password ?? "");
    assert(token.length === 64, 400, "VALIDATION_ERROR", "重置令牌不合法");
    assert(password.length >= 8, 400, "VALIDATION_ERROR", "密码至少 8 位");
    return transaction(this.pool, async (tx) => {
      const row = (
        await tx.query(
          "SELECT * FROM fr_password_reset_tokens WHERE digest=$1 AND used_at IS NULL AND expires_at>now() FOR UPDATE",
          [digest(token)],
        )
      ).rows[0];
      assert(row, 400, "VALIDATION_ERROR", "令牌无效或已过期");
      const salt = randomBytes(16).toString("hex");
      const derived = await new Promise<Buffer>((resolve, reject) =>
        scrypt(password, salt, 32, (err, buf) => (err ? reject(err) : resolve(buf))));
      const passwordHash = `scrypt-v1$${salt}$${derived.toString("hex")}`;
      await tx.query("UPDATE fr_users SET password_hash=$1 WHERE id=$2", [passwordHash, row.user_id]);
      await tx.query("UPDATE fr_password_reset_tokens SET used_at=now() WHERE digest=$1", [digest(token)]);
      // 作废该用户所有会话。
      await tx.query("UPDATE fr_sessions SET expires_at=now() WHERE user_id=$1", [row.user_id]);
      return { reset: true };
    });
  }

  async agentSummarize(headers: Headers, projectId: string) {
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
      const comments = (
        await tx.query(
          "SELECT u.display_name AS reviewer, c.body FROM fr_comments c JOIN fr_users u ON u.id=c.author_id WHERE c.project_id=$1 ORDER BY c.created_at LIMIT 50",
          [projectId],
        )
      ).rows;
      if (comments.length === 0) {
        return { summary: "", comment_count: 0, message: "没有评论可汇总" };
      }
      await tx.query(
        "INSERT INTO fr_acl_audit(user_id,project_id,action,allowed) VALUES($1,$2,'agent.summarize',true)",
        [user.id, projectId],
      );
      const result = await summarizeProjectComments(this.pool, projectId, comments);
      const taskId = await storeTaskResult(this.pool, projectId, user.id, result);
      return { task_id: taskId, ...result };
    });
  }

  async agentTaskList(headers: Headers, projectId: string) {
    return transaction(this.pool, async (tx) => {
      const project = (
        await tx.query("SELECT id FROM fr_projects WHERE id=$1", [projectId])
      ).rows[0];
      assert(project, 404, "NOT_FOUND", "项目不存在");
      const user = await this.auth.authenticate(headers, true, tx);
      await this.authority.principal(tx, user, project);
      return { tasks: await listTaskResults(this.pool, projectId) };
    });
  }
}
