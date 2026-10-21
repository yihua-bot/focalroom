/**
 * 云画布的草稿与批注。权限、事务及审计统一复用项目权威服务。
 * 正式内容的修改仍通过命令事务处理。
 */
import { randomUUID } from "node:crypto";
import { z } from "zod";
import type { Headers } from "./auth.ts";
import { AuthorityService, type Scope } from "./authority.ts";
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
}
