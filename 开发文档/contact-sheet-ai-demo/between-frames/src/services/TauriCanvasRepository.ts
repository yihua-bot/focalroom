import { convertFileSrc } from '@tauri-apps/api/core';
import { invokeCommand as invoke } from './tauriCommand';
import { z } from 'zod';
import type { CanvasRepository } from '../domain/canvas';

const boardSchema = z.object({
  id: z.string(), project_id: z.string(), title: z.string(), state: z.enum(['draft', 'official', 'frozen']),
}).transform(value => ({ id: value.id, projectId: value.project_id, title: value.title, state: value.state }));
const nodeSchema = z.object({
  id: z.string(), boardId: z.string(), nodeType: z.string(),
  x: z.number().finite(), y: z.number().finite(), width: z.number().finite(), height: z.number().finite(),
  rotation: z.number().finite(), zOrder: z.number().int(), assetId: z.string().nullable(), renditionRef: z.string().nullable(),
  contentJson: z.string().nullable(), styleJson: z.string().nullable(), childrenJson: z.string().nullable(),
  locked: z.boolean(), visible: z.boolean(),
});
const annotationSchema = z.object({
  id: z.string(), boardId: z.string(), anchorType: z.string(),
  x: z.number().nullable(), y: z.number().nullable(),
  author: z.string(), body: z.string(), resolved: z.boolean(), createdAt: z.string(),
});
const photoSourceSchema = z.object({ assetId: z.string(), thumbnailPath: z.string() });

export class TauriCanvasRepository implements CanvasRepository {
  async listBoards(projectId: string) {
    return z.array(boardSchema).parse(await invoke('canvas_list_boards', { projectId }));
  }
  async createBoard(projectId: string, title: string) {
    return boardSchema.parse(await invoke('canvas_create_board', { projectId, title }));
  }
  async getNodes(boardId: string) {
    const [raw, photos] = await Promise.all([
      invoke('canvas_get_nodes', { boardId }),
      invoke('canvas_get_photo_sources', { boardId }),
    ]);
    const sources = new Map(z.array(photoSourceSchema).parse(photos).map(p => [p.assetId, convertFileSrc(p.thumbnailPath)]));
    return z.array(nodeSchema).parse(raw).filter(n => n.visible).map(n => ({
      ...n, thumbnailPath: n.assetId ? sources.get(n.assetId) : undefined,
    }));
  }
  async placePhoto(projectId: string, boardId: string, assetId: string, x: number, y: number, width: number, height: number) {
    return nodeSchema.parse(await invoke('canvas_place_photo', { projectId, cmd: { boardId, assetId, x, y, width, height } }));
  }
  async moveNodes(projectId: string, boardId: string, nodeIds: string[], dx: number, dy: number) {
    await invoke('canvas_move_nodes', { projectId, boardId, nodeIds, dx, dy });
  }
  async addText(projectId: string, boardId: string, nodeType: string, x: number, y: number, width: number, height: number, content: string, color?: string) {
    return nodeSchema.parse(await invoke('canvas_add_text', { projectId, cmd: { boardId, nodeType, x, y, width, height, content, color } }));
  }
  async deleteNode(projectId: string, boardId: string, nodeId: string) {
    await invoke('canvas_delete_node', { projectId, boardId, nodeId });
  }
  async addAnnotation(projectId: string, boardId: string, x: number, y: number, body: string) {
    annotationSchema.parse(await invoke('canvas_add_annotation', { projectId, cmd: { boardId, x, y, body } }));
  }
  async resolveAnnotation(projectId: string, annotationId: string, resolved: boolean) {
    await invoke('canvas_resolve_annotation', { projectId, cmd: { annotationId, resolved } });
  }
  async getAnnotations(boardId: string) {
    return z.array(annotationSchema).parse(await invoke('canvas_get_annotations', { boardId }));
  }
  async addArrow(projectId: string, boardId: string, fromNodeId: string, toNodeId: string, color?: string) {
    nodeSchema.parse(await invoke('canvas_add_arrow', { projectId, cmd: { boardId, fromNodeId, toNodeId, color } }));
  }
  async undo(projectId: string, boardId: string) {
    return z.boolean().parse(await invoke('canvas_undo', { projectId, boardId }));
  }
  async addInk(projectId: string, boardId: string, x: number, y: number, width: number, height: number,
    points: Array<{x:number;y:number;pressure?:number}>, color?: string, strokeWidth?: number) {
    nodeSchema.parse(await invoke('canvas_add_ink', { projectId, cmd: { boardId, x, y, width, height, points, color, strokeWidth } }));
  }

  async groupNodes(projectId: string, boardId: string, nodeIds: string[], label: string) {
    await invoke('canvas_group_nodes', {
      projectId,
      cmd: { boardId, nodeIds, label },
    });
  }

  async resizeNode(projectId: string, boardId: string, nodeId: string, width: number, height: number) {
    await invoke('canvas_resize_node', { projectId, boardId, nodeId, width, height });
  }
  async setNodeLocked(projectId: string, boardId: string, nodeId: string, locked: boolean) {
    await invoke('canvas_set_node_locked', { projectId, boardId, nodeId, locked });
  }
}
