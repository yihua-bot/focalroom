/**
 * G6 Shared Workspace：云端画布 API——Board CRUD + 节点操作 + 评论。
 * Web 协作者在此查看/批注画布；正式变更走命令事务。
 */
import type { Pool } from "pg";
import { randomUUID } from "node:crypto";
import { assert } from "./errors.ts";

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
  z_order: number;
  asset_id: string | null;
  content_json: unknown;
  style_json: unknown;
  visible: boolean;
}

export class BoardService {
  private pool: Pool;
  constructor(pool: Pool) { this.pool = pool; }

  async list(projectId: string): Promise<CloudBoard[]> {
    const rows = await this.pool.query(
      "SELECT id, project_id, title, state, created_at FROM fr_boards WHERE project_id=$1 ORDER BY created_at",
      [projectId],
    );
    return rows.rows;
  }

  async create(userId: string, projectId: string, title: string): Promise<CloudBoard> {
    title = title.trim().slice(0, 200);
    assert(title.length > 0, 400, "VALIDATION_ERROR", "标题不能为空");
    const id = randomUUID();
    const row = await this.pool.query(
      "INSERT INTO fr_boards(id, project_id, title, created_by) VALUES($1,$2,$3,$4) RETURNING id, project_id, title, state, created_at",
      [id, projectId, title, userId],
    );
    return row.rows[0];
  }

  async nodes(boardId: string): Promise<CloudCanvasNode[]> {
    const rows = await this.pool.query(
      `SELECT id, board_id, node_type, x, y, width, height, z_order, asset_id, content_json, style_json, visible
       FROM fr_canvas_nodes WHERE board_id=$1 AND visible=true ORDER BY z_order`,
      [boardId],
    );
    return rows.rows;
  }

  async annotate(userId: string, boardId: string, x: number, y: number, body: string): Promise<string> {
    body = body.trim().slice(0, 2000);
    assert(body.length > 0, 400, "VALIDATION_ERROR", "评论不能为空");
    const id = randomUUID();
    await this.pool.query(
      "INSERT INTO fr_board_annotations(id, board_id, anchor_type, x, y, author_id, body) VALUES($1,$2,'board',$3,$4,$5,$6)",
      [id, boardId, x, y, userId, body],
    );
    return id;
  }

  async annotations(boardId: string): Promise<unknown[]> {
    const rows = await this.pool.query(
      `SELECT a.id, a.board_id, a.x, a.y, a.body, a.resolved, a.created_at, u.display_name AS author
       FROM fr_board_annotations a JOIN fr_users u ON u.id=a.author_id
       WHERE a.board_id=$1 ORDER BY a.created_at`,
      [boardId],
    );
    return rows.rows;
  }

  async resolve(annotationId: string, resolved: boolean): Promise<void> {
    await this.pool.query(
      "UPDATE fr_board_annotations SET resolved=$2 WHERE id=$1",
      [annotationId, resolved],
    );
  }
}
