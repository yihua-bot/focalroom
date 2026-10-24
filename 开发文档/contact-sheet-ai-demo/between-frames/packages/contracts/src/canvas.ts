import { z } from 'zod';

const id = z.uuid();
// 项目沿用既有稳定编号（包括内置演示项目），不能强制转换为 UUID。
const projectId = z.string().min(1).max(160);
const coordinate = z.number().finite().min(-1e9).max(1e9);
const rectangle = { x: coordinate, y: coordinate, width: z.number().positive().max(1e9), height: z.number().positive().max(1e9) };
const board = { boardId: id };
const nodeIds = z.array(id).min(1).max(300).refine(ids => new Set(ids).size === ids.length);
const color = z.string().max(100).optional();
export const editorialField = z.enum(['position','size','rotation','order','content','style','children','rendition']);
const action = <T extends string, S extends z.ZodRawShape>(type: T, shape: S) => z.strictObject({ type: z.literal(type), data: z.strictObject(shape) });
export const editorialSelectionSchema = z.strictObject({
  draftId: id, draftRevision: z.number().int().positive(), targetBoardId: id,
  expectedTargetRevision: z.number().int().positive(), selectedChangeIds: z.array(z.string().regex(/^[a-f0-9]{64}$/)).max(300),
  resolutions: z.array(z.strictObject({ changeId: z.string().regex(/^[a-f0-9]{64}$/), field: editorialField, choice: z.enum(['keep_current','use_draft']) })).max(2400).optional(),
  fieldsByChange: z.record(z.string().regex(/^[a-f0-9]{64}$/), z.array(editorialField).min(1).max(8)).optional(),
  structuralResolutions: z.array(z.strictObject({ changeId: z.string().regex(/^[a-f0-9]{64}$/), choice: z.enum(['restore_proposed','remove_proposed']) })).max(300).refine(values => new Set(values.map(value => value.changeId)).size === values.length).optional(),
});
export const canvasActionSchema = z.discriminatedUnion('type', [
  action('CreateBoard', { title: z.string().trim().min(1).max(200) }),
  action('PlacePhoto', { ...board, assetId: z.string().min(1), ...rectangle }),
  action('MoveNodes', { ...board, nodeIds, dx: coordinate, dy: coordinate }),
  action('AddText', { ...board, nodeType: z.enum(['text', 'sticky', 'shape']), ...rectangle, content: z.string().max(5000), color }),
  action('DeleteNode', { ...board, nodeId: id }),
  action('AddAnnotation', { ...board, x: coordinate, y: coordinate, body: z.string().trim().min(1).max(2000) }),
  action('ResolveAnnotation', { annotationId: id, resolved: z.boolean() }),
  action('AddArrow', { ...board, fromNodeId: id, toNodeId: id, color }),
  action('AddInk', { ...board, x: coordinate, y: coordinate, width: z.number().min(0).max(1e9), height: z.number().min(0).max(1e9),
    points: z.array(z.strictObject({ x: coordinate, y: coordinate, pressure: z.number().min(0).max(1).optional() })).min(1).max(10000), color, strokeWidth: z.number().min(0.1).max(1000).optional() }),
  action('GroupNodes', { ...board, nodeIds: nodeIds.refine(ids => ids.length <= 100), label: z.string().trim().min(1).max(50) }),
  action('ResizeNode', { ...board, nodeId: id, width: z.number().min(20).max(10000), height: z.number().min(20).max(10000) }),
  action('SetNodeLocked', { ...board, nodeId: id, locked: z.boolean() }),
  action('Undo', board), action('Redo', board), action('AdoptBoard', board),
  action('ApplyEditorialChanges', { selection: editorialSelectionSchema, simulationId: z.string().regex(/^[a-f0-9]{64}$/) }),
  action('RecordEditorialDecision', { draftId: id, draftRevision: z.number().int().positive(), changeIds: z.array(z.string().regex(/^[a-f0-9]{64}$/)).min(1).max(300), decision: z.enum(['pending','rejected','deferred']), fieldsByChange: z.record(z.string().regex(/^[a-f0-9]{64}$/), z.array(editorialField).min(1).max(8)).optional() }),
  action('RevertEditorialChanges', { mergeCommandId: id, simulationId: z.string().regex(/^[a-f0-9]{64}$/) }),
  action('AddEditorialComment', { draftId: id, draftRevision: z.number().int().positive(), changeId: z.string().regex(/^[a-f0-9]{64}$/).nullable(), threadId: id.nullable(), body: z.string().trim().min(1).max(2000) }),
  action('ResolveEditorialThread', { draftId: id, draftRevision: z.number().int().positive(), threadId: id, resolved: z.boolean() }),
]);
export const canvasRequestSchema = z.strictObject({
  schemaVersion: z.literal(3), commandId: id, idempotencyKey: z.string().min(1).max(160),
  projectId, boardId: id, baseRevision: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER - 1000),
  operations: z.array(z.strictObject({ opId: id, action: canvasActionSchema })).min(1).max(300),
}).superRefine((request, ctx) => {
  const seen = new Set<string>();
  request.operations.forEach((op, i) => {
    if (seen.has(op.opId)) ctx.addIssue({ code: 'custom', message: 'Duplicate operation', path: ['operations', i, 'opId'] });
    seen.add(op.opId);
    if ('boardId' in op.action.data && op.action.data.boardId !== request.boardId) ctx.addIssue({ code: 'custom', message: 'Board mismatch' });
    if (['CreateBoard', 'Undo', 'Redo', 'AdoptBoard', 'ApplyEditorialChanges', 'RecordEditorialDecision', 'RevertEditorialChanges', 'AddEditorialComment', 'ResolveEditorialThread'].includes(op.action.type) && request.operations.length !== 1) ctx.addIssue({ code: 'custom', message: 'Must be a single operation' });
    if ((op.action.type === 'AddEditorialComment' || op.action.type === 'ResolveEditorialThread') && (op.action.data.draftId !== request.boardId || op.action.data.draftRevision !== request.baseRevision)) ctx.addIssue({ code: 'custom', message: 'Discussion version mismatch' });
    if (op.action.type === 'ApplyEditorialChanges' && (op.action.data.selection.targetBoardId !== request.boardId || op.action.data.selection.expectedTargetRevision !== request.baseRevision)) ctx.addIssue({ code: 'custom', message: 'Preview mismatch' });
  });
});
export const canvasReceiptSchema = z.object({ commandId: id, projectId, boardId: id,
  baseRevision: z.number().int().nonnegative(), revision: z.number().int().positive(), changed: z.boolean(), results: z.array(z.unknown()) });
export const canvasBoardRecordSchema = z.object({ id: z.string(), project_id: z.string(), title: z.string(), state: z.enum(['draft', 'official', 'frozen']),
  owning_task_id: z.string().nullable(), revision: z.number().int().positive() });
export const canvasNodeRecordSchema = z.object({
  id: z.string(), boardId: z.string(), nodeType: z.string(),
  x: coordinate, y: coordinate, width: z.number().finite(), height: z.number().finite(),
  rotation: z.number().finite(), zOrder: z.number().int(), assetId: z.string().nullable(), renditionRef: z.string().nullable(),
  contentJson: z.string().nullable(), styleJson: z.string().nullable(), childrenJson: z.string().nullable(), locked: z.boolean(), visible: z.boolean(),
});
export const canvasAnnotationRecordSchema = z.object({ id: z.string(), boardId: z.string(), anchorType: z.string(), x: z.number().nullable(), y: z.number().nullable(),
  author: z.string(), body: z.string(), resolved: z.boolean(), createdAt: z.string() });

/** 清除待确认记录前，同时验证回执身份及每个操作的业务结果。 */
export function parseCanvasReceipt(request: CanvasCommandRequest, value: unknown): CanvasReceipt {
  const receipt = canvasReceiptSchema.parse(value);
  const invalid = () => { throw new Error('画布操作结果尚未确认'); };
  if (receipt.commandId !== request.commandId || receipt.projectId !== request.projectId || receipt.boardId !== request.boardId || receipt.baseRevision !== request.baseRevision || receipt.results.length !== request.operations.length || receipt.revision !== request.baseRevision + Number(receipt.changed)) invalid();
  request.operations.forEach((op, i) => {
    const result = receipt.results[i];
    switch (op.action.type) {
      case 'CreateBoard': {
        const board = canvasBoardRecordSchema.parse(result);
        if (!receipt.changed || board.id !== request.boardId || board.project_id !== request.projectId || board.revision !== receipt.revision) invalid();
        break;
      }
      case 'PlacePhoto': case 'AddText': case 'AddArrow': case 'AddInk': case 'GroupNodes': {
        const node = canvasNodeRecordSchema.parse(result);
        const type = op.action.type === 'AddText' ? op.action.data.nodeType : { PlacePhoto: 'photo', AddArrow: 'arrow', AddInk: 'ink', GroupNodes: 'group' }[op.action.type];
        if (!receipt.changed || node.id !== op.opId || node.boardId !== request.boardId || node.nodeType !== type || !node.visible) invalid();
        break;
      }
      case 'AddAnnotation': {
        const annotation = canvasAnnotationRecordSchema.parse(result);
        if (!receipt.changed || annotation.id !== op.opId || annotation.boardId !== request.boardId) invalid();
        break;
      }
      case 'Undo': case 'Redo': if (z.boolean().parse(result) !== receipt.changed) invalid(); break;
      default: z.null().parse(result);
    }
  });
  return receipt;
}
export type CanvasAction = z.infer<typeof canvasActionSchema>;
export type CanvasCommandRequest = z.infer<typeof canvasRequestSchema>;
export type CanvasReceipt = z.infer<typeof canvasReceiptSchema>;
