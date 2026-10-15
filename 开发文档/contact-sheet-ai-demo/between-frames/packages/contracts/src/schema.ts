import { z } from 'zod';

const id = z.string().min(1).max(160);
const nonce = z.string().min(8).max(160);
const revision = z.number().int().min(0);
const position = z.number().int().min(0).max(999_999);
const ids = (maximum: number) => z.array(id).min(1).max(maximum).refine(values => new Set(values).size === values.length, 'ID 不得重复');
const operationBase = { operation_id: id, photo_id: id, reason: z.string().max(1000).optional() };
const operation = z.discriminatedUnion('op', [
  z.strictObject({ ...operationBase, op: z.literal('add'), to_index: position }),
  z.strictObject({ ...operationBase, op: z.literal('move'), to_index: position }),
  z.strictObject({ ...operationBase, op: z.literal('remove') }),
]);
const anchor = z.discriminatedUnion('type', [
  z.strictObject({ type: z.literal('photo'), photo_id: id, x: z.number().min(0).max(1), y: z.number().min(0).max(1) }),
  z.strictObject({ type: z.literal('sequence'), sequence_item_id: id, revision_id: id }),
]);
const envelope = {
  version: z.literal('2.0'), command_id: nonce, workspace_id: id, project_id: id,
  expected_revision: revision, idempotency_key: nonce, client_trace_id: id.optional(),
};
const approvalPayload = z.strictObject({ proposal_id: id, expected_proposal_revision: revision, accepted_operation_ids: ids(200), approval_nonce: nonce });
export const approvalSchema = z.strictObject({ version: envelope.version, workspace_id: id, project_id: id, expected_revision: revision, ...approvalPayload.shape });
export type Approval = z.infer<typeof approvalSchema>;
export const projectModeSchema = z.enum(['LOCAL', 'SHARED']);
export type ProjectMode = z.infer<typeof projectModeSchema>;
export const mediaPolicySchema = z.discriminatedUnion('mode', [
  z.strictObject({ mode: z.literal('LOCAL_ONLY'), allow_original_upload: z.literal(false), strip_gps: z.literal(true) }),
  z.strictObject({ mode: z.literal('AUTHORIZED_PREVIEW_ONLY'), allow_original_upload: z.literal(false), strip_gps: z.literal(true), max_long_edge: z.number().int().min(1).max(2048), consent_id: id, allowed_provider_ids: ids(10) }),
]);
export type MediaPolicy = z.infer<typeof mediaPolicySchema>;
export const commandSchema = z.discriminatedUnion('kind', [
  z.strictObject({ ...envelope, kind: z.literal('photo.set_selection'), payload: z.strictObject({ photo_ids: ids(1000), selection_state: z.enum(['UNDECIDED', 'KEEP', 'REJECT']) }) }),
  z.strictObject({ ...envelope, kind: z.literal('sequence.move_item'), payload: z.strictObject({ sequence_id: id, item_id: id, to_index: position }) }),
  z.strictObject({ ...envelope, kind: z.literal('sequence.add_item'), payload: z.strictObject({ sequence_id: id, photo_id: id, to_index: position }) }),
  z.strictObject({ ...envelope, kind: z.literal('sequence.remove_item'), payload: z.strictObject({ sequence_id: id, item_id: id }) }),
  z.strictObject({ ...envelope, kind: z.literal('proposal.create'), payload: z.strictObject({ base_revision: revision, title: z.string().min(1).max(200), operations: z.array(operation).min(1).max(200) }) }),
  z.strictObject({ ...envelope, kind: z.literal('proposal.submit'), payload: z.strictObject({ proposal_id: id, expected_proposal_revision: revision }) }),
  z.strictObject({ ...envelope, kind: z.literal('proposal.apply'), payload: approvalPayload }),
  z.strictObject({ ...envelope, kind: z.literal('comment.add'), payload: z.strictObject({ body: z.string().min(1).max(5000), anchor }) }),
  z.strictObject({ ...envelope, kind: z.literal('comment.resolve'), payload: z.strictObject({ thread_id: id, state: z.enum(['RESOLVED', 'REOPENED']) }) }),
]);
export type CommandEnvelope = z.infer<typeof commandSchema>;
export const errorCodes = ['VALIDATION_ERROR', 'PERMISSION_DENIED', 'AUTH_REVOKED', 'AUTHORITY_MISMATCH', 'REVISION_CONFLICT', 'NOT_IMPLEMENTED'] as const;
export type ErrorCode = typeof errorCodes[number];
export type CommandReply =
  | { ok: true; command_id: string; new_revision: number; event_id: string; affected_ids: string[] }
  | { ok: false; command_id: string; error: { code: ErrorCode; message: string } };
export const toolResultSchema = z.discriminatedUnion('ok', [
  z.strictObject({ call_id: id, task_id: id, ok: z.literal(true), result: z.unknown() }),
  z.strictObject({ call_id: id, task_id: id, ok: z.literal(false), error: z.strictObject({ code: z.string().min(1).max(80), message: z.string().min(1).max(2000) }) }),
]);
export type ToolResult = z.infer<typeof toolResultSchema>;

export function parseCommand(input: unknown): CommandEnvelope {
  const command = commandSchema.parse(input);
  const unique = (values: string[]) => new Set(values).size === values.length;
  if (command.kind === 'proposal.create') {
    if (!unique(command.payload.operations.map(item => item.operation_id))) throw new Error('重复提案操作 ID');
    if (command.payload.base_revision !== command.expected_revision) throw new Error('提案基线与命令版本不一致');
  }
  return command;
}
