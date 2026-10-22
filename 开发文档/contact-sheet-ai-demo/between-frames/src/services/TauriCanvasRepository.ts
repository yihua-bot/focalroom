import { convertFileSrc } from '@tauri-apps/api/core';
import { invokeCommand as invoke } from './tauriCommand';
import { z } from 'zod';
import type { CanvasRepository } from '../domain/canvas';
import type { CanvasAction, CanvasReceipt } from '../domain/canvasCommands';
import { CanvasCommandClient } from './CanvasCommandClient';
import { canvasBoardRecordSchema, canvasNodeRecordSchema as nodeSchema, canvasAnnotationRecordSchema as annotationSchema } from '@betweenframes/contracts';

const boardSchema = canvasBoardRecordSchema.transform(value => ({ id: value.id, projectId: value.project_id, title: value.title, state: value.state, owningTaskId: value.owning_task_id, revision: value.revision }));
const photoSourceSchema = z.object({ assetId: z.string(), thumbnailPath: z.string() });

export class TauriCanvasRepository implements CanvasRepository {
  async compareDraft(projectId: string, boardId: string) {
    const comparison = z.object({ draft: boardSchema, sourceBoardId: z.string().nullable(), sourceChanged: z.boolean(), baselineNodes: z.array(nodeSchema),
      changes: z.array(z.object({ id: z.string(), nodeId: z.string(), kind: z.enum(['add', 'change', 'remove']), nodeType: z.string() })), photoSources: z.array(photoSourceSchema) }).parse(await invoke('canvas_comparison', { projectId, boardId }));
    if (comparison.draft.id !== boardId || comparison.draft.projectId !== projectId) throw new Error('无法读取这份画布方案。');
    const photos = new Map(comparison.photoSources.map(photo => [photo.assetId, convertFileSrc(photo.thumbnailPath)]));
    return { ...comparison, draftRevision: comparison.draft.revision, baselineNodes: comparison.baselineNodes.map(node => ({ ...node, thumbnailPath: node.assetId ? photos.get(node.assetId) : undefined })) };
  }
  private versions = new Map<string, { projectId: string; revision: number }>();
  private annotationBoards = new Map<string, string>();
  private commands = new CanvasCommandClient(request => invoke('canvas_execute', { request }), window.localStorage);
  private remember(receipt: CanvasReceipt) {
    const old = this.versions.get(receipt.boardId);
    this.versions.set(receipt.boardId, { projectId: receipt.projectId, revision: Math.max(old?.revision ?? 0, receipt.revision) });
  }
  private async edit(projectId: string, boardId: string, actions: CanvasAction[]) {
    const version = this.versions.get(boardId);
    if (!version || version.projectId !== projectId) throw new Error('请重新打开画布后再操作。');
    const receipt = await this.commands.execute(projectId, boardId, version.revision, actions);
    this.remember(receipt);
    return receipt.results;
  }
  pendingCommands(projectId: string) {
    return this.commands.pending(projectId).map(request => ({ boardId: request.boardId }));
  }
  async retryPending(projectId: string, boardId: string) {
    this.remember(await this.commands.retry(projectId, boardId));
  }
  async listBoards(projectId: string) {
    const boards = z.array(boardSchema).parse(await invoke('canvas_list_boards', { projectId }));
    for (const board of boards) if (!this.versions.has(board.id)) this.versions.set(board.id, { projectId, revision: board.revision });
    return boards;
  }
  async createBoard(projectId: string, title: string) {
    const boardId = crypto.randomUUID();
    const receipt = await this.commands.execute(projectId, boardId, 0, [{ type: 'CreateBoard', data: { title } }]);
    this.remember(receipt);
    return boardSchema.parse(receipt.results[0]);
  }
  async getSnapshot(boardId: string) {
    const version = this.versions.get(boardId);
    if (!version) throw new Error('请重新打开画布。');
    const snapshot = z.object({ board: boardSchema, nodes: z.array(nodeSchema), annotations: z.array(annotationSchema), photoSources: z.array(photoSourceSchema) }).parse(
      await invoke('canvas_snapshot', { projectId: version.projectId, boardId }));
    if (snapshot.board.id !== boardId || snapshot.board.projectId !== version.projectId || snapshot.board.revision < version.revision) throw new Error('画布已更新，请重新打开后继续。');
    this.versions.set(boardId, { projectId: version.projectId, revision: snapshot.board.revision });
    const sources = new Map(snapshot.photoSources.map(p => [p.assetId, convertFileSrc(p.thumbnailPath)]));
    for (const annotation of snapshot.annotations) this.annotationBoards.set(annotation.id, boardId);
    return { board: snapshot.board, annotations: snapshot.annotations, nodes: snapshot.nodes.filter(n => n.visible).map(n => ({ ...n, thumbnailPath: n.assetId ? sources.get(n.assetId) : undefined })) };
  }
  async getNodes(boardId: string) { return (await this.getSnapshot(boardId)).nodes; }
  async placePhoto(projectId: string, boardId: string, assetId: string, x: number, y: number, width: number, height: number) {
    return nodeSchema.parse((await this.edit(projectId, boardId, [{ type: 'PlacePhoto', data: { boardId, assetId, x, y, width, height } }]))[0]);
  }
  async moveNodes(projectId: string, boardId: string, nodeIds: string[], dx: number, dy: number) {
    await this.edit(projectId, boardId, [{ type: 'MoveNodes', data: { boardId, nodeIds, dx, dy } }]);
  }
  async addText(projectId: string, boardId: string, nodeType: string, x: number, y: number, width: number, height: number, content: string, color?: string) {
    return nodeSchema.parse((await this.edit(projectId, boardId, [{ type: 'AddText', data: { boardId, nodeType: z.enum(['text', 'sticky', 'shape']).parse(nodeType), x, y, width, height, content, color } }]))[0]);
  }
  async deleteNode(projectId: string, boardId: string, nodeId: string) {
    await this.edit(projectId, boardId, [{ type: 'DeleteNode', data: { boardId, nodeId } }]);
  }
  async addAnnotation(projectId: string, boardId: string, x: number, y: number, body: string) {
    await this.edit(projectId, boardId, [{ type: 'AddAnnotation', data: { boardId, x, y, body } }]);
  }
  async resolveAnnotation(projectId: string, annotationId: string, resolved: boolean) {
    const boardId = this.annotationBoards.get(annotationId);
    if (!boardId) throw new Error('请重新打开评论后操作。');
    await this.edit(projectId, boardId, [{ type: 'ResolveAnnotation', data: { annotationId, resolved } }]);
  }
  async getAnnotations(boardId: string) {
    return (await this.getSnapshot(boardId)).annotations;
  }
  async addArrow(projectId: string, boardId: string, fromNodeId: string, toNodeId: string, color?: string) {
    await this.edit(projectId, boardId, [{ type: 'AddArrow', data: { boardId, fromNodeId, toNodeId, color } }]);
  }
  async undo(projectId: string, boardId: string) {
    return z.boolean().parse((await this.edit(projectId, boardId, [{ type: 'Undo', data: { boardId } }]))[0]);
  }
  async redo(projectId: string, boardId: string) {
    return z.boolean().parse((await this.edit(projectId, boardId, [{ type: 'Redo', data: { boardId } }]))[0]);
  }
  async adoptBoard(projectId: string, boardId: string) {
    await this.edit(projectId, boardId, [{ type: 'AdoptBoard', data: { boardId } }]);
  }
  async addInk(projectId: string, boardId: string, x: number, y: number, width: number, height: number,
    points: Array<{x:number;y:number;pressure?:number}>, color?: string, strokeWidth?: number) {
    await this.edit(projectId, boardId, [{ type: 'AddInk', data: { boardId, x, y, width, height, points, color, strokeWidth } }]);
  }

  async groupNodes(projectId: string, boardId: string, nodeIds: string[], label: string) {
    await this.edit(projectId, boardId, [{ type: 'GroupNodes', data: { boardId, nodeIds, label } }]);
  }

  async resizeNode(projectId: string, boardId: string, nodeId: string, width: number, height: number) {
    await this.edit(projectId, boardId, [{ type: 'ResizeNode', data: { boardId, nodeId, width, height } }]);
  }
  async setNodeLocked(projectId: string, boardId: string, nodeId: string, locked: boolean) {
    await this.edit(projectId, boardId, [{ type: 'SetNodeLocked', data: { boardId, nodeId, locked } }]);
  }
  async placePhotos(projectId: string, boardId: string, photos: Array<{ assetId: string; x: number; y: number; width: number; height: number }>) {
    await this.edit(projectId, boardId, photos.map(photo => ({ type: 'PlacePhoto', data: { boardId, ...photo } })));
  }
  async setNodesLocked(projectId: string, boardId: string, nodeIds: string[], locked: boolean) {
    await this.edit(projectId, boardId, nodeIds.map(nodeId => ({ type: 'SetNodeLocked', data: { boardId, nodeId, locked } })));
  }
}
