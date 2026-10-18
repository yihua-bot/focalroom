import { randomUUID } from "node:crypto";
import type { Pool, PoolClient } from "pg";
import {
  dispatch,
  parseCommand,
  sharedSnapshotSchema,
  type SharedSnapshot,
  type Principal,
  type CommandEnvelope,
} from "@betweenframes/contracts";
import { assert, denied, ApiError } from "./errors.ts";
import { digest, transaction } from "./database.ts";
import { AuthService, type Headers, type Identity } from "./auth.ts";
export type Project = {
  id: string;
  workspace_id: string;
  owner_id: string;
  title: string;
  state: "STAGING" | "SHARED" | "ABORTED";
  revision: number;
  snapshot: SharedSnapshot;
};
export type Scope = {
  tx: PoolClient;
  user: Identity;
  project: Project;
  principal: Extract<Principal, { type: "user" }>;
};
export function parseCloudCommand(input: unknown): CommandEnvelope {
  try {
    return parseCommand(input);
  } catch {
    throw new ApiError(
      400,
      "VALIDATION_ERROR",
      "命令不符合 v2 契约或包含重复操作 ID",
    );
  }
}
export class AuthorityService {
  readonly pool: Pool;
  readonly auth: AuthService;
  constructor(pool: Pool, auth: AuthService) {
    this.pool = pool;
    this.auth = auth;
  }
  async principal(
    tx: Pick<PoolClient, "query">,
    user: Identity,
    project: Project,
  ): Promise<Scope["principal"]> {
    const membership = (
      await tx.query(
        "SELECT role FROM fr_memberships WHERE workspace_id=$1 AND user_id=$2",
        [project.workspace_id, user.id],
      )
    ).rows[0];
    const grant = (
      await tx.query(
        "SELECT role,can_approve,can_edit_formal FROM fr_grants WHERE project_id=$1 AND user_id=$2 AND active",
        [project.id, user.id],
      )
    ).rows[0];
    // 工作区成员关系本身不授予所有项目；只有 Owner/Admin 或独立项目 Grant。
    const privileged =
      project.owner_id === user.id ||
      ["OWNER", "ADMIN"].includes(membership?.role);
    if (!privileged && !grant) throw denied();
    const role = privileged
      ? project.owner_id === user.id
        ? "OWNER"
        : membership.role
      : grant.role;
    return {
      id: user.id,
      workspace_id: project.workspace_id,
      project_id: project.id,
      type: "user",
      role,
      can_approve: privileged || !!grant?.can_approve,
      can_edit_formal: privileged || !!grant?.can_edit_formal,
      can_propose: role !== "VIEWER",
    };
  }
  async scope<T>(
    headers: Headers,
    id: string,
    mutation: boolean,
    action: (scope: Scope) => Promise<T>,
  ) {
    try {
      return await transaction(this.pool, async (tx) => {
        const user = await this.auth.authenticate(headers, mutation, tx);
        const pre = (
          await tx.query("SELECT workspace_id FROM fr_projects WHERE id=$1", [
            id,
          ])
        ).rows[0];
        assert(pre, 404, "NOT_FOUND", "项目不存在");
        await tx.query("SELECT id FROM fr_workspaces WHERE id=$1 FOR SHARE", [
          pre.workspace_id,
        ]);
        const project = (
          await tx.query(
            `SELECT * FROM fr_projects WHERE id=$1 ${mutation ? "FOR UPDATE" : "FOR SHARE"}`,
            [id],
          )
        ).rows[0] as Project;
        const principal = await this.principal(tx, user, project);
        const result = await action({ tx, user, project, principal });
        await tx.query(
          "INSERT INTO fr_acl_audit(user_id,project_id,action,allowed) VALUES($1,$2,$3,true)",
          [user.id, id, mutation ? "write" : "read"],
        );
        return result;
      });
    } catch (error) {
      if ((error as { status?: number }).status === 403) {
        const user = await this.auth
          .authenticate(headers, false)
          .catch(() => null);
        if (user)
          await this.pool.query(
            "INSERT INTO fr_acl_audit(user_id,project_id,action,allowed) VALUES($1,$2,$3,false)",
            [user.id, id, mutation ? "write" : "read"],
          );
      }
      throw error;
    }
  }
  async command(headers: Headers, id: string, input: unknown) {
    const command = parseCloudCommand(input);
    return this.scope(headers, id, true, async (scope) => {
      const { tx, user, project, principal } = scope;
      assert(
        command.project_id === id &&
          command.workspace_id === project.workspace_id,
        403,
        "PERMISSION_DENIED",
        "命令不属于此项目",
      );
      assert(
        project.state === "SHARED",
        409,
        "AUTHORITY_MISMATCH",
        "项目尚未完成共享迁移",
      );
      const hash = digest(JSON.stringify(command));
      const replay = (
        await tx.query(
          "SELECT command_id,actor_id,payload_digest,reply FROM fr_commands WHERE project_id=$1 AND (command_id=$2 OR (actor_id=$3 AND idempotency_key=$4))",
          [id, command.command_id, user.id, command.idempotency_key],
        )
      ).rows;
      if (replay.length) {
        assert(
          replay.length === 1 &&
            replay[0].actor_id === user.id &&
            replay[0].command_id === command.command_id &&
            replay[0].payload_digest === hash,
          409,
          "IDEMPOTENCY_CONFLICT",
          "幂等键已绑定其他命令",
        );
        const permission = dispatch(
          command,
          principal,
          {
            workspace_id: project.workspace_id,
            project_id: id,
            mode: "SHARED",
            local_state: "SHARED_CACHE",
            revision: command.expected_revision,
          },
          "CLOUD",
        );
        assert(
          !permission.ok && permission.error.code === "NOT_IMPLEMENTED",
          403,
          "PERMISSION_DENIED",
          "当前权限不再允许重放此命令",
        );
        return replay[0].reply;
      }
      const decision = dispatch(
        command,
        principal,
        {
          workspace_id: project.workspace_id,
          project_id: id,
          mode: "SHARED",
          local_state: "SHARED_CACHE",
          revision: project.revision,
        },
        "CLOUD",
      );
      assert(
        !decision.ok && decision.error.code === "NOT_IMPLEMENTED",
        decision.ok
          ? 400
          : decision.error.code === "REVISION_CONFLICT"
            ? 409
            : 403,
        decision.ok ? "VALIDATION_ERROR" : decision.error.code,
        "权限或项目版本已变化，请刷新后重新提交",
      );
      const formal = await applyCommand(scope, command);
      if (formal) {
        sharedSnapshotSchema.parse(project.snapshot);
        project.revision++;
        await tx.query(
          "UPDATE fr_projects SET revision=$2,snapshot=$3 WHERE id=$1",
          [id, project.revision, JSON.stringify(project.snapshot)],
        );
        await tx.query(
          "INSERT INTO fr_versions(project_id,revision,snapshot,actor_id,command_id) VALUES($1,$2,$3,$4,$5)",
          [
            id,
            project.revision,
            JSON.stringify(project.snapshot),
            user.id,
            command.command_id,
          ],
        );
      }
      const event = (
        await tx.query(
          "INSERT INTO fr_events(project_id,actor_id,kind,revision,detail) VALUES($1,$2,$3,$4,$5) RETURNING id",
          [
            id,
            user.id,
            formal ? "ProjectVersionCommitted" : command.kind,
            project.revision,
            JSON.stringify({ command_id: command.command_id }),
          ],
        )
      ).rows[0];
      const reply = {
        ok: true,
        command_id: command.command_id,
        new_revision: project.revision,
        event_id: String(event.id),
        affected_ids: affectedIds(command),
      };
      await tx.query(
        "INSERT INTO fr_commands(project_id,command_id,actor_id,idempotency_key,payload_digest,envelope,reply) VALUES($1,$2,$3,$4,$5,$6,$7)",
        [
          id,
          command.command_id,
          user.id,
          command.idempotency_key,
          hash,
          JSON.stringify(command),
          JSON.stringify(reply),
        ],
      );
      return reply;
    });
  }
}
const affectedIds = (command: CommandEnvelope) =>
  command.kind === "proposal.create" || command.kind === "comment.add"
    ? [command.command_id]
    : [];
function conflict(condition: unknown, message: string): asserts condition {
  assert(condition, 409, "REVISION_CONFLICT", message);
}
function sequence(snapshot: SharedSnapshot, id: string) {
  const found = snapshot.sequences.find((s) => s.id === id);
  assert(found, 400, "VALIDATION_ERROR", "序列不存在");
  return found;
}
async function applyCommand(
  scope: Scope,
  command: CommandEnvelope,
): Promise<boolean> {
  const { tx, user, project, principal } = scope;
  const state = project.snapshot;
  switch (command.kind) {
    case "photo.set_selection":
      for (const id of command.payload.photo_ids) {
        const photo = state.photos.find((p) => p.id === id);
        assert(photo, 400, "VALIDATION_ERROR", "照片不存在");
        photo.selection = command.payload.selection_state;
      }
      return true;
    case "sequence.add_item": {
      const { sequence_id, photo_id, to_index } = command.payload;
      const seq = sequence(state, sequence_id);
      assert(
        state.photos.some((p) => p.id === photo_id) &&
          !seq.items.some((i) => i.photo_id === photo_id) &&
          to_index <= seq.items.length,
        400,
        "VALIDATION_ERROR",
        "照片或插入位置不合法",
      );
      seq.items.splice(to_index, 0, { id: command.command_id, photo_id });
      return true;
    }
    case "sequence.move_item":
    case "sequence.remove_item": {
      const seq = sequence(state, command.payload.sequence_id);
      const index = seq.items.findIndex(
        (i) => i.id === command.payload.item_id,
      );
      assert(index >= 0, 400, "VALIDATION_ERROR", "序列项不存在");
      if (command.kind === "sequence.remove_item") seq.items.splice(index, 1);
      else {
        assert(
          command.payload.to_index < seq.items.length,
          400,
          "VALIDATION_ERROR",
          "移动位置不合法",
        );
        const [item] = seq.items.splice(index, 1);
        seq.items.splice(command.payload.to_index, 0, item);
      }
      return true;
    }
    case "proposal.create":
      assert(
        command.payload.title.trim(),
        400,
        "VALIDATION_ERROR",
        "提案标题不能为空",
      );
      for (const op of command.payload.operations)
        assert(
          state.photos.some((p) => p.id === op.photo_id),
          400,
          "VALIDATION_ERROR",
          "提案含有其他项目的照片",
        );
      await tx.query(
        "INSERT INTO fr_proposals(id,project_id,author_id,title,base_revision,state,operations,nonce) VALUES($1,$2,$3,$4,$5,'DRAFT',$6,$7)",
        [
          command.command_id,
          project.id,
          user.id,
          command.payload.title,
          project.revision,
          JSON.stringify(command.payload.operations),
          randomUUID(),
        ],
      );
      return false;
    case "proposal.submit": {
      const { proposal_id, expected_proposal_revision } = command.payload;
      const result = await tx.query(
        "UPDATE fr_proposals SET state='SUBMITTED',revision=revision+1 WHERE id=$1 AND project_id=$2 AND author_id=$3 AND state='DRAFT' AND revision=$4",
        [proposal_id, project.id, user.id, expected_proposal_revision],
      );
      conflict(result.rowCount === 1, "提案状态或作者已变化");
      return false;
    }
    case "proposal.apply": {
      const p = command.payload;
      const row = (
        await tx.query(
          "SELECT * FROM fr_proposals WHERE id=$1 AND project_id=$2 FOR UPDATE",
          [p.proposal_id, project.id],
        )
      ).rows[0];
      conflict(
        row &&
          row.state === "SUBMITTED" &&
          row.revision === p.expected_proposal_revision &&
          row.base_revision === project.revision,
        "提案基线或状态已变化；原提案已保留",
      );
      assert(
        row.nonce === p.approval_nonce,
        403,
        "PERMISSION_DENIED",
        "审批凭据不匹配",
      );
      const ops = row.operations as {
        op: "add" | "move" | "remove";
        operation_id: string;
        photo_id: string;
        to_index?: number;
      }[];
      assert(
        p.accepted_operation_ids.every((id) =>
          ops.some((o) => o.operation_id === id),
        ),
        400,
        "VALIDATION_ERROR",
        "选择了未知提案操作",
      );
      const seq = state.sequences[0];
      for (const op of ops.filter((o) =>
        p.accepted_operation_ids.includes(o.operation_id),
      )) {
        const index = seq.items.findIndex((i) => i.photo_id === op.photo_id);
        if (op.op === "add") {
          conflict(
            index < 0 && op.to_index! <= seq.items.length,
            "新增照片已存在或位置越界",
          );
          seq.items.splice(op.to_index!, 0, {
            id: `${row.id}:${op.operation_id}`,
            photo_id: op.photo_id,
          });
        }
        if (op.op === "remove") {
          conflict(index >= 0, "移除照片已不存在");
          seq.items.splice(index, 1);
        }
        if (op.op === "move") {
          conflict(
            index >= 0 && op.to_index! < seq.items.length,
            "重排照片已变化",
          );
          const [item] = seq.items.splice(index, 1);
          seq.items.splice(op.to_index!, 0, item);
        }
      }
      await tx.query(
        "UPDATE fr_proposals SET state=$2,revision=revision+1,decided_by=$3 WHERE id=$1",
        [
          row.id,
          p.accepted_operation_ids.length ? "APPLIED" : "REJECTED",
          user.id,
        ],
      );
      return p.accepted_operation_ids.length > 0;
    }
    case "comment.add": {
      const anchor = command.payload.anchor;
      if (anchor.type === "photo")
        assert(
          state.photos.some((p) => p.id === anchor.photo_id),
          400,
          "VALIDATION_ERROR",
          "评论照片不属于此项目",
        );
      else if (anchor.type === "revision") {
        const version = (
          await tx.query(
            "SELECT 1 FROM fr_versions WHERE project_id=$1 AND revision::text=$2",
            [project.id, anchor.revision_id],
          )
        ).rows[0];
        assert(
          version,
          400,
          "VALIDATION_ERROR",
          "版本锚点不存在",
        );
      } else {
        const version = (
          await tx.query(
            "SELECT snapshot FROM fr_versions WHERE project_id=$1 AND revision::text=$2",
            [project.id, anchor.revision_id],
          )
        ).rows[0];
        assert(
          version &&
            (version.snapshot as SharedSnapshot).sequences.some((s) =>
              s.items.some((i) => i.id === anchor.sequence_item_id),
            ),
          400,
          "VALIDATION_ERROR",
          "序列锚点与版本不匹配",
        );
      }
      assert(
        command.payload.body.trim(),
        400,
        "VALIDATION_ERROR",
        "评论不能为空",
      );
      await tx.query(
        "INSERT INTO fr_comments(id,project_id,author_id,body,anchor) VALUES($1,$2,$3,$4,$5)",
        [
          command.command_id,
          project.id,
          user.id,
          command.payload.body.trim(),
          JSON.stringify(anchor),
        ],
      );
      return false;
    }
    case "comment.resolve": {
      const row = (
        await tx.query(
          "SELECT author_id FROM fr_comments WHERE id=$1 AND project_id=$2",
          [command.payload.thread_id, project.id],
        )
      ).rows[0];
      assert(
        row &&
          (row.author_id === user.id ||
            ["OWNER", "ADMIN", "EDITOR"].includes(principal.role)),
        403,
        "PERMISSION_DENIED",
        "只能处理自己的评论或由编辑处理",
      );
      await tx.query("UPDATE fr_comments SET state=$2 WHERE id=$1", [
        command.payload.thread_id,
        command.payload.state,
      ]);
      return false;
    }
  }
}
