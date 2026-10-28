import { historicalObservationSchema, withLocalRevisionPreviews } from './revisionContextSchemas';
import { convertFileSrc } from '@tauri-apps/api/core';
import { invokeCommand as invoke } from './tauriCommand';
import { z } from 'zod';
import type { CanvasRepository, EditorialSelection, RevertSimulation } from '../domain/canvas';
import type { CanvasAction, CanvasReceipt } from '../domain/canvasCommands';
import { CanvasCommandClient } from './CanvasCommandClient';
import { editorialField, canvasBoardRecordSchema, canvasNodeRecordSchema as nodeSchema, canvasAnnotationRecordSchema as annotationSchema } from '@betweenframes/contracts';

const boardSchema = canvasBoardRecordSchema.transform(value => ({ id: value.id, projectId: value.project_id, title: value.title, state: value.state, owningTaskId: value.owning_task_id, revision: value.revision }));
const photoSourceSchema = z.object({ assetId: z.string(), photoId: z.string().optional(), thumbnailPath: z.string() });
const discussionSchema = z.object({ draftId: z.string(), draftRevision: z.number().int().positive(), cursor: z.number().int().nonnegative(), eventCount: z.number().int().nonnegative(), totalMessages: z.number().int().nonnegative(), nextOffset: z.number().int().nonnegative().nullable(), messages: z.array(z.object({ id: z.string(), threadId: z.string(), changeId: z.string().nullable(), body: z.string(), rootBody: z.string(), createdAt: z.string(), resolved: z.boolean(), isReply: z.boolean() })) });
const structuralConflictSchema = z.object({changeId:z.string(),nodeId:z.string(),kind:z.enum(['current_removed','remove_changed']),currentModified:z.boolean(),relatedNodes:z.array(z.object({nodeId:z.string(),nodeType:z.string(),effect:z.enum(['detach','remove'])})).max(5000),preservedCommentCount:z.number().int().nonnegative().max(1000),resolved:z.boolean()});

export class TauriCanvasRepository implements CanvasRepository {
  async editorialRevisionContext(projectId: string, draftId: string, offset = 0) {
    const decision = z.enum(['pending','accepted','rejected','deferred','partial']);
    const result = z.object({ draftId:z.string(),parent:z.object({kind:z.literal('board'),id:z.string(),revision:z.number().int().positive()}),parentTitle:z.string(),discussion:discussionSchema,
      decisions:z.array(z.object({changeId:z.string(),nodeType:z.string(),kind:z.enum(['add','change','remove']),decision,fieldDecisions:z.record(z.string(),decision).refine(fields => Object.keys(fields).every(field => editorialField.safeParse(field).success))})),
      observations:z.array(historicalObservationSchema),
      totalDecisions:z.number().int().nonnegative(),totalObservations:z.number().int().nonnegative(),nextOffset:z.number().int().nonnegative().nullable(),hasEarlier:z.boolean(),
    }).nullable().parse(await invoke('canvas_editorial_revision_context',{projectId,draftId,offset}));
    if (result && (result.draftId!==draftId || result.discussion.draftId!==result.parent.id || result.discussion.draftRevision!==result.parent.revision || result.parent.id===draftId || (result.nextOffset!==null && result.nextOffset<=offset))) throw new Error('无法读取这份方案的修订依据。');
    return result ? {...result,observations:withLocalRevisionPreviews(result.observations)} : null;
  }
  async editorialDiscussion(projectId: string, draftId: string, revision: number, offset = 0, cursor?: number) {
    const result = discussionSchema.parse(await invoke('canvas_editorial_discussion', { projectId, draftId, revision, offset, cursor: cursor ?? null }));
    if (result.draftId !== draftId || result.draftRevision !== revision || (cursor !== undefined && result.cursor !== cursor)) throw new Error('讨论已更新，请重新打开方案。');
    return result;
  }
  async addEditorialComment(projectId: string, draftId: string, revision: number, comment: { changeId: string | null; threadId: string | null; body: string }) {
    this.remember(await this.commands.execute(projectId, draftId, revision, [{ type: 'AddEditorialComment', data: { draftId, draftRevision: revision, ...comment } }]));
  }
  async resolveEditorialThread(projectId: string, draftId: string, revision: number, threadId: string, resolved: boolean) {
    this.remember(await this.commands.execute(projectId, draftId, revision, [{ type: 'ResolveEditorialThread', data: { draftId, draftRevision: revision, threadId, resolved } }]));
  }
  private photoPaths = new Map<string, string>();
  private photos<T extends { assetId: string | null }>(nodes: T[]) {
    return nodes.map(node => ({ ...node, thumbnailPath: node.assetId ? this.photoPaths.get(node.assetId) : undefined }));
  }
  async compareDraft(projectId: string, boardId: string) {
    const comparison = z.object({ draft: boardSchema, sourceBoardId: z.string().nullable(), sourceRevision: z.number().nullable(), currentSourceRevision: z.number().nullable(), sourceChanged: z.boolean(), baselineNodes: z.array(nodeSchema), draftNodes: z.array(nodeSchema), currentSourceNodes: z.array(nodeSchema),
      changes: z.array(z.object({ id: z.string(), nodeId: z.string(), sourceNodeId: z.string().nullable(), kind: z.enum(['add', 'change', 'remove']), nodeType: z.string(), before: nodeSchema.nullable(), after: nodeSchema.nullable(), kinds: z.array(z.string()), requiredDependencies: z.array(z.string()), decision: z.enum(['pending','accepted','rejected','deferred','partial']), fields: z.array(editorialField), acceptedFields: z.array(editorialField), fieldDecisions: z.record(z.string(), z.enum(['pending','accepted','rejected','deferred'])) })), photoSources: z.array(photoSourceSchema) }).parse(await invoke('canvas_comparison', { projectId, boardId }));
    if (comparison.draft.id !== boardId || comparison.draft.projectId !== projectId) throw new Error('无法读取这份画布方案。');
    for (const photo of comparison.photoSources) this.photoPaths.set(photo.assetId, convertFileSrc(photo.thumbnailPath));
    return { ...comparison, draftRevision: comparison.draft.revision, baselineNodes: this.photos(comparison.baselineNodes), currentSourceNodes: this.photos(comparison.currentSourceNodes), draftNodes: this.photos(comparison.draftNodes), changes: comparison.changes.map(change => ({ ...change, before: change.before ? this.photos([change.before])[0] : null, after: change.after ? this.photos([change.after])[0] : null })) };
  }
  async simulateMerge(projectId: string, selection: EditorialSelection) {
    const result = z.object({ simulationId: z.string(), draftRevision: z.number(), targetRevision: z.number(), effectiveChangeIds: z.array(z.string()), autoIncludedChangeIds: z.array(z.string()), conflicts: z.array(z.string()), attributeConflicts: z.array(z.object({ changeId: z.string(), nodeId: z.string(), field: z.enum(['position','size','rotation','order','content','style','children','rendition']), baseline: z.unknown(), current: z.unknown(), proposed: z.unknown() })), structuralConflicts:z.array(structuralConflictSchema).max(300).optional().default([]), mergedNodes: z.array(nodeSchema), canApply: z.boolean(), nodeIdMap: z.record(z.string(), z.string()) }).parse(await invoke('canvas_simulate_merge', { projectId, selection }));
    if (result.draftRevision !== selection.draftRevision || result.targetRevision !== selection.expectedTargetRevision || result.mergedNodes.some(node => node.boardId !== selection.targetBoardId) || result.structuralConflicts.some(c => !result.effectiveChangeIds.includes(c.changeId) || (c.resolved && !selection.structuralResolutions?.some(r => r.changeId === c.changeId && r.choice === (c.kind==='current_removed'?'restore_proposed':'remove_proposed')))) || (result.canApply && (result.conflicts.length || result.attributeConflicts.length || result.structuralConflicts.some(c => !c.resolved)))) throw new Error('方案已有修订，请重新打开审稿。');
    return { ...result, mergedNodes: this.photos(result.mergedNodes) };
  }
  async applyMerge(projectId: string, selection: EditorialSelection, simulationId: string) {
    const receipt = await this.commands.execute(projectId, selection.targetBoardId, selection.expectedTargetRevision, [{ type: 'ApplyEditorialChanges', data: { selection, simulationId } }]);
    this.remember(receipt);
  }
  async recordDecision(projectId: string, selection: EditorialSelection, decision: 'pending'|'rejected'|'deferred') {
    this.remember(await this.commands.execute(projectId, selection.targetBoardId, selection.expectedTargetRevision, [{ type: 'RecordEditorialDecision', data: { draftId: selection.draftId, draftRevision: selection.draftRevision, changeIds: selection.selectedChangeIds, decision, ...(selection.fieldsByChange ? { fieldsByChange: selection.fieldsByChange } : {}) } }]));
  }
  async editorialHistory(projectId: string, boardId: string, before?: number) {
    return z.array(z.object({ cursor: z.number().int().positive(), commandId: z.string(), draftId: z.string(), changeCount: z.number().int().nonnegative(), createdAt: z.string(), undone: z.boolean(), compensated: z.boolean(), isCompensation: z.boolean(), nodeTypes: z.array(z.string()) })).parse(await invoke('canvas_editorial_history', { projectId, boardId, before: before ?? null }));
  }
  async simulateRevert(projectId: string, boardId: string, commandId: string) {
    const result = z.object({ mergeCommandId: z.string(), targetRevision: z.number().int().positive(), simulationId: z.string(), currentNodes: z.array(nodeSchema), mergedNodes: z.array(nodeSchema), conflicts: z.array(z.string()), canApply: z.boolean(), photoSources: z.array(photoSourceSchema),restoredCommentCount:z.number().int().nonnegative().max(1000).optional().default(0) }).parse(await invoke('canvas_simulate_revert', { projectId, boardId, commandId }));
    if (result.mergeCommandId !== commandId || [...result.currentNodes, ...result.mergedNodes].some(n => n.boardId !== boardId)) throw new Error('撤回内容已变化，请重新打开画布。');
    for (const photo of result.photoSources) this.photoPaths.set(photo.assetId, convertFileSrc(photo.thumbnailPath));
    return { ...result, currentNodes: this.photos(result.currentNodes), mergedNodes: this.photos(result.mergedNodes) };
  }
  async revertMerge(projectId: string, boardId: string, simulation: RevertSimulation) {
    this.remember(await this.commands.execute(projectId, boardId, simulation.targetRevision, [{ type: 'RevertEditorialChanges', data: { mergeCommandId: simulation.mergeCommandId, simulationId: simulation.simulationId } }]));
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
    const photoIds = new Map(snapshot.photoSources.map(p => [p.assetId, p.photoId]));
    for (const annotation of snapshot.annotations) this.annotationBoards.set(annotation.id, boardId);
    return { board: snapshot.board, annotations: snapshot.annotations, nodes: snapshot.nodes.filter(n => n.visible).map(n => ({ ...n, photoId: n.assetId ? photoIds.get(n.assetId) : undefined, thumbnailPath: n.assetId ? sources.get(n.assetId) : undefined })) };
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
  async deleteNodes(projectId: string, boardId: string, nodeIds: string[]) {
    await this.edit(projectId, boardId, [...new Set(nodeIds)].map(nodeId => ({ type: 'DeleteNode', data: { boardId, nodeId } })));
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
