/**
 * 云画布的草稿与批注。权限、事务及审计统一复用项目权威服务。
 * 正式内容的修改仍通过命令事务处理。
 */
import { randomUUID } from "node:crypto";
import { z } from "zod";
import type { Headers } from "./auth.ts";
import { AuthorityService, type Scope } from "./authority.ts";
import { digest } from "./database.ts";
import { assert } from "./errors.ts";

const projectIdSchema = z.string().trim().min(1).max(160);
const idSchema = z.uuid();
const coordinateSchema = z.number().finite().min(-1e9).max(1e9);
const createSchema = z.strictObject({
  projectId: projectIdSchema,
  title: z.string().trim().min(1).max(200),
});
const annotationSchema = z.strictObject({
  boardId: idSchema,
  x: coordinateSchema,
  y: coordinateSchema,
  body: z.string().trim().min(1).max(2000),
});
const resolveSchema = z.strictObject({ resolved: z.boolean() });

const nodeTypeSchema = z.enum(['photo', 'text', 'sticky', 'shape', 'arrow', 'ink', 'group', 'chapter']);
const extentSchema = z.number().finite().min(1).max(10000);
const canvasActionSchema = z.discriminatedUnion('type', [
  z.strictObject({
    type: z.literal('createNode'), nodeId: idSchema.optional(), nodeType: nodeTypeSchema,
    x: coordinateSchema, y: coordinateSchema, width: extentSchema, height: extentSchema,
    rotation: z.number().finite().min(-3600).max(3600).optional(), zIndex: z.number().int().optional(),
    assetId: z.string().trim().min(1).max(512).nullable().optional(),
    contentJson: z.unknown().optional(), styleJson: z.unknown().optional(),
  }),
  z.strictObject({
    type: z.literal('updateNode'), nodeId: idSchema,
    x: coordinateSchema.optional(), y: coordinateSchema.optional(),
    width: extentSchema.optional(), height: extentSchema.optional(),
    rotation: z.number().finite().min(-3600).max(3600).optional(), zIndex: z.number().int().optional(),
    locked: z.boolean().optional(), visible: z.boolean().optional(),
    contentJson: z.unknown().optional(), styleJson: z.unknown().optional(),
  }),
  z.strictObject({
    type: z.literal('moveNodes'),
    moves: z.array(z.strictObject({ nodeId: idSchema, x: coordinateSchema, y: coordinateSchema })).min(1).max(100),
  }),
  z.strictObject({ type: z.literal('deleteNode'), nodeId: idSchema }),
]);
const canvasCommandSchema = z.strictObject({
  schemaVersion: z.literal(3),
  commandId: idSchema,
  idempotencyKey: z.string().trim().min(1).max(160),
  projectId: projectIdSchema,
  baseRevision: z.number().int().min(0),
  operations: z.array(z.strictObject({ opId: idSchema, action: canvasActionSchema })).min(1).max(100),
}).superRefine((command, ctx) => {
  if (new Set(command.operations.map(op => op.opId)).size !== command.operations.length)
    ctx.addIssue({ code: 'custom', message: '命令包含重复操作 ID' });
});
export type CanvasCommand = z.infer<typeof canvasCommandSchema>;

export interface CloudBoard {
  id: string;
  project_id: string;
  title: string;
  state: string;
  created_at: string;
}

export interface CloudCanvasNode {
  id: string;
  board_id: string;
  node_type: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
  z_order: number;
  asset_id: string | null;
  photo_id: string | null;
  rendition_ref: string | null;
  content_json: unknown;
  style_json: unknown;
  children_json: unknown;
  locked: boolean;
  visible: boolean;
}

export class BoardService {
  private readonly authority: AuthorityService;
  constructor(authority: AuthorityService) { this.authority = authority; }

  private projectScope<T>(
    headers: Headers, projectId: string, mutation: boolean,
    action: (scope: Scope) => Promise<T>,
  ) {
    return this.authority.scope(headers, projectId, mutation, async (scope) => {
      assert(scope.project.state === "SHARED", 409, "AUTHORITY_MISMATCH", "项目尚未完成共享");
      if (mutation) {
        assert(scope.principal.can_propose, 403, "PERMISSION_DENIED", "当前权限不能修改画布草稿或批注");
      }
      return action(scope);
    });
  }

  private async boardScope<T>(
    headers: Headers, boardId: string, mutation: boolean,
    action: (scope: Scope) => Promise<T>,
  ) {
    // 先认证，再查所属项目；项目锁内重新确认归属，防止检查与写入分离。
    await this.authority.auth.authenticate(headers, mutation);
    const board = (await this.authority.pool.query(
      "SELECT project_id FROM fr_boards WHERE id=$1", [boardId],
    )).rows[0];
    assert(board, 404, "NOT_FOUND", "画布不存在");
    return this.projectScope(headers, board.project_id, mutation, async (scope) => {
      const current = await scope.tx.query(
        "SELECT id FROM fr_boards WHERE id=$1 AND project_id=$2 FOR SHARE",
        [boardId, scope.project.id],
      );
      assert(current.rowCount, 404, "NOT_FOUND", "画布不存在");
      return action(scope);
    });
  }

  async list(headers: Headers, projectId: unknown): Promise<CloudBoard[]> {
    const id = projectIdSchema.parse(projectId);
    return this.projectScope(headers, id, false, async ({ tx }) => (
      await tx.query(
        "SELECT id, project_id, title, state, created_at FROM fr_boards WHERE project_id=$1 ORDER BY created_at,id",
        [id],
      )
    ).rows);
  }

  async create(headers: Headers, input: unknown): Promise<CloudBoard> {
    const value = createSchema.parse(input);
    return this.projectScope(headers, value.projectId, true, async ({ tx, user }) => (
      await tx.query(
        "INSERT INTO fr_boards(id, project_id, title, created_by) VALUES($1,$2,$3,$4) RETURNING id, project_id, title, state, created_at",
        [randomUUID(), value.projectId, value.title, user.id],
      )
    ).rows[0]);
  }

  async nodes(headers: Headers, boardId: unknown): Promise<CloudCanvasNode[]> {
    const id = idSchema.parse(boardId);
    return this.boardScope(headers, id, false, async ({ tx }) => (
      await tx.query(
        `SELECT id, board_id, node_type, x, y, width, height, rotation, z_order,
          n.asset_id, n.rendition_ref, n.content_json, n.style_json, n.children_json, n.locked, n.visible,
          a.photo_id
         FROM fr_canvas_nodes n
         LEFT JOIN fr_asset_aliases a ON a.asset_id::text = n.asset_id
         WHERE n.board_id=$1 AND n.visible=true ORDER BY n.z_order, n.id`,
        [id],
      )
    ).rows);
  }

  async annotate(headers: Headers, input: unknown): Promise<string> {
    const value = annotationSchema.parse(input);
    return this.boardScope(headers, value.boardId, true, async ({ tx, user }) => {
      const id = randomUUID();
      await tx.query(
        "INSERT INTO fr_board_annotations(id, board_id, anchor_type, x, y, author_id, body) VALUES($1,$2,'board',$3,$4,$5,$6)",
        [id, value.boardId, value.x, value.y, user.id, value.body],
      );
      return id;
    });
  }

  async annotations(headers: Headers, boardId: unknown): Promise<unknown[]> {
    const id = idSchema.parse(boardId);
    return this.boardScope(headers, id, false, async ({ tx }) => (
      await tx.query(
        `SELECT a.id, a.board_id, a.x, a.y, a.body, a.resolved, a.created_at,
          a.author_id, u.display_name AS author
         FROM fr_board_annotations a JOIN fr_users u ON u.id=a.author_id
         WHERE a.board_id=$1 ORDER BY a.created_at,a.id`,
        [id],
      )
    ).rows);
  }

  async resolve(headers: Headers, annotationId: unknown, input: unknown) {
    const id = idSchema.parse(annotationId);
    const { resolved } = resolveSchema.parse(input);
    await this.authority.auth.authenticate(headers, true);
    const annotation = (await this.authority.pool.query(
      "SELECT board_id FROM fr_board_annotations WHERE id=$1", [id],
    )).rows[0];
    assert(annotation, 404, "NOT_FOUND", "批注不存在");
    return this.boardScope(headers, annotation.board_id, true, async ({ tx, user, principal }) => {
      const current = (await tx.query(
        "SELECT author_id FROM fr_board_annotations WHERE id=$1 AND board_id=$2 FOR UPDATE",
        [id, annotation.board_id],
      )).rows[0];
      assert(current, 404, "NOT_FOUND", "批注不存在");
      assert(current.author_id === user.id || principal.can_approve || principal.can_edit_formal,
        403, "PERMISSION_DENIED", "只能处理自己的批注，或由项目编辑处理");
      await tx.query(
        "UPDATE fr_board_annotations SET resolved=$2 WHERE id=$1 AND board_id=$3",
        [id, resolved, annotation.board_id],
      );
      return { id, resolved };
    });
  }

  async commands(headers: Headers, boardId: unknown, input: unknown) {
    const command = canvasCommandSchema.parse(input);
    const id = idSchema.parse(boardId);
    return this.boardScope(headers, id, true, async ({ tx, user }) => {
      // 独占 Board 行锁：串行化并发命令的读取-校验-推进，避免修订竞争。
      const board = (await tx.query(
        "SELECT project_id,state,revision FROM fr_boards WHERE id=$1 FOR UPDATE", [id],
      )).rows[0];
      assert(board, 404, "NOT_FOUND", "画布不存在");
      assert(command.projectId === board.project_id, 403, "PERMISSION_DENIED", "命令不属于此画布的项目");
      assert(board.state === "draft", 409, "BOARD_READ_ONLY", "仅草稿画布接受正式命令");
      const hash = digest(JSON.stringify(command));
      const replay = (await tx.query(
        "SELECT command_id,actor_id,payload_digest,reply FROM fr_canvas_commands WHERE board_id=$1 AND (command_id=$2 OR (project_id=$3 AND actor_id=$4 AND idempotency_key=$5))",
        [id, command.commandId, board.project_id, user.id, command.idempotencyKey],
      )).rows;
      if (replay.length) {
        assert(
          replay.length === 1 && replay[0].actor_id === user.id &&
            replay[0].command_id === command.commandId && replay[0].payload_digest === hash,
          409, "IDEMPOTENCY_CONFLICT", "幂等键已绑定其他命令",
        );
        return replay[0].reply;
      }
      assert(command.baseRevision === board.revision, 409, "REVISION_CONFLICT", "画布修订已变化，请刷新后重新提交");
      const affected: string[] = [];
      const lockNode = async (nodeId: string) => {
        const current = (await tx.query(
          "SELECT id,locked FROM fr_canvas_nodes WHERE id=$1 AND board_id=$2 FOR UPDATE", [nodeId, id],
        )).rows[0];
        assert(current, 409, "NODE_NOT_FOUND", `节点 ${nodeId} 不在此画布`);
        return current;
      };
      for (const { action } of command.operations) {
        if (action.type === "createNode") {
          const nodeId = action.nodeId ?? randomUUID();
          await tx.query(
            "INSERT INTO fr_canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,content_json,style_json) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
            [nodeId, id, action.nodeType, action.x, action.y, action.width, action.height,
              action.rotation ?? 0, action.zIndex ?? 0, action.assetId ?? null,
              action.contentJson === undefined ? null : JSON.stringify(action.contentJson),
              action.styleJson === undefined ? null : JSON.stringify(action.styleJson)],
          );
          affected.push(nodeId);
        } else if (action.type === "updateNode") {
          const current = await lockNode(action.nodeId);
          // 锁定节点仅允许"只解锁"操作；其余修改须先解锁。
          const touchesBeyondUnlock = action.x !== undefined || action.y !== undefined ||
            action.width !== undefined || action.height !== undefined || action.rotation !== undefined ||
            action.zIndex !== undefined || action.visible !== undefined ||
            action.contentJson !== undefined || action.styleJson !== undefined;
          assert(!current.locked || (action.locked === false && !touchesBeyondUnlock),
            409, "NODE_LOCKED", "节点已锁定，先解锁再修改");
          const sets: string[] = [];
          const values: unknown[] = [];
          const column = (name: string, value: unknown) => {
            values.push(value);
            sets.push(`${name}=$${values.length}`);
          };
          if (action.x !== undefined) column("x", action.x);
          if (action.y !== undefined) column("y", action.y);
          if (action.width !== undefined) column("width", action.width);
          if (action.height !== undefined) column("height", action.height);
          if (action.rotation !== undefined) column("rotation", action.rotation);
          if (action.zIndex !== undefined) column("z_order", action.zIndex);
          if (action.locked !== undefined) column("locked", action.locked);
          if (action.visible !== undefined) column("visible", action.visible);
          if (action.contentJson !== undefined) column("content_json", JSON.stringify(action.contentJson));
          if (action.styleJson !== undefined) column("style_json", JSON.stringify(action.styleJson));
          assert(sets.length, 400, "VALIDATION_ERROR", "更新操作不包含任何字段");
          values.push(action.nodeId, id);
          await tx.query(
            `UPDATE fr_canvas_nodes SET ${sets.join(",")},updated_at=now() WHERE id=$${values.length - 1} AND board_id=$${values.length}`,
            values,
          );
          affected.push(action.nodeId);
        } else if (action.type === "moveNodes") {
          for (const move of action.moves) {
            const current = await lockNode(move.nodeId);
            assert(!current.locked, 409, "NODE_LOCKED", "节点已锁定，先解锁再修改");
            await tx.query(
              "UPDATE fr_canvas_nodes SET x=$3,y=$4,updated_at=now() WHERE id=$1 AND board_id=$2",
              [move.nodeId, id, move.x, move.y],
            );
            affected.push(move.nodeId);
          }
        } else {
          const current = await lockNode(action.nodeId);
          assert(!current.locked, 409, "NODE_LOCKED", "锁定节点须先解锁再删除");
          await tx.query("DELETE FROM fr_canvas_nodes WHERE id=$1 AND board_id=$2", [action.nodeId, id]);
          affected.push(action.nodeId);
        }
      }
      const newRevision = board.revision + 1;
      await tx.query("UPDATE fr_boards SET revision=$2,updated_at=now() WHERE id=$1", [id, newRevision]);
      const reply = { ok: true, commandId: command.commandId, newRevision, affectedNodeIds: affected };
      await tx.query(
        "INSERT INTO fr_canvas_commands(command_id,board_id,project_id,actor_id,idempotency_key,payload_digest,envelope,reply) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        [command.commandId, id, board.project_id, user.id, command.idempotencyKey, hash,
          JSON.stringify(command), JSON.stringify(reply)],
      );
      return reply;
    });
  }
}
