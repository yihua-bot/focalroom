import { memoryTraceSchema } from './memoryTraceSchemas';
import { photoJudgmentSchema } from './photoJudgmentSchemas';
import { knowledgeMatchSchema } from './knowledgeMatchSchema';
import { knowledgeSearchSchema } from './knowledgeSearchSchema';
import { selectionRevisionSchema, withLocalRevisionPreviews } from './revisionContextSchemas';
import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { z } from 'zod';
import { canvasBoardRecordSchema } from '@betweenframes/contracts';
import { AGENT_TASK_UPDATED_EVENT, DRAFT_UPDATED_EVENT } from '../domain/commands.generated';
import type { AgentRepository, TaskCommand } from '../domain/agent';
const budget = z.object({ maxModelTurns: z.number().int(), maxToolCalls: z.number().int(), maxDurationSeconds: z.number().int(), maxCloudImages: z.number().int().min(0).max(100), maxCostMinorUsd: z.number().int(), maxContextTokens: z.number().int(), maxTotalTokens: z.number().int(), maxOutputTokens: z.number().int() });
const visionRecovery = z.object({ state: z.enum(['pending','completed','unknown','rejected','expired']), images: z.number().int().min(1).max(100) });
// 聊天回执只有 completed 才带正文；服务端把文字留在别处却回 completed，等于查无实据，按未确认处理。
const chatJob = z.discriminatedUnion('state', [z.object({state:z.literal('completed'),text:z.string().min(1)}),z.object({state:z.enum(['pending','unknown','rejected','expired']),text:z.null()})]);
const queuedRecoveryConfig = z.discriminatedUnion('enabled', [z.object({enabled:z.literal(false)}),z.object({enabled:z.literal(true),policy:z.literal('encrypted-queue-v1'),maxQueueSeconds:z.literal(600)})]);
const projection = z.object({
 tasks: z.array(z.object({ memoryTrace: memoryTraceSchema.optional(), photoJudgments: z.array(photoJudgmentSchema).max(20).default([]), queueCancellation: z.enum(['cancelled','already_sent','unconfirmed']).nullable().optional(), visionRecovery: visionRecovery.nullable().optional(), creativeReferences: z.array(z.object({ kind: z.enum(['knowledge','context','memory']), label: z.string(), version: z.union([z.string(),z.number()]).nullable().optional(), citationId: z.string().optional(), passage: z.string().nullable().optional(), license: z.string().optional(), source: z.string().nullable().optional(), institution: z.string().nullable().optional(), reviewStatus: z.string().nullable().optional(), disputed: z.boolean().optional(), detailError: z.object({code:z.string(),message:z.string()}).nullable().optional(), section: z.string().optional(), memoryId: z.number().int().nonnegative().optional(), origin: z.string().optional(), scope: z.string().optional(), sourceProjectId: z.string().optional(), rounds: z.number().int().nonnegative().optional(), retrievals: z.number().int().nonnegative().optional(), memoryTextVersion: z.enum(['verified','changed','unrecorded','source_unavailable']).optional(), memoryStatus: z.string().nullable().optional(), memoryDisabled: z.boolean().nullable().optional(), summaryOnly: z.boolean().optional(), matchReason: knowledgeMatchSchema.nullable().optional() }).superRefine((value, context) => {
 if (value.kind === 'memory' && value.memoryTextVersion && value.memoryTextVersion !== 'verified' && value.passage != null) {
  context.addIssue({ code:'custom', message:'Unverified historical memory cannot contain current text' });
 }
})).default([]), knowledgeSearches: z.array(knowledgeSearchSchema).max(100).default([]), draftLineage: z.array(z.object({ childId: z.string(), kind: z.string(), parent: z.object({ kind: z.enum(['board','selection']), id: z.string(), revision: z.number().int().positive() }), reason: z.string() })).default([]), preferenceProposals: z.array(z.object({ id: z.string(), statement: z.string(), applicability: z.string(), photoIds: z.array(z.string()), sources: z.array(z.object({ messageId: z.number().int(), text: z.string(), createdAt: z.string() })), state: z.enum(['pending','saved','removed','dismissed']), createdAt: z.string() })).default([]), inspectedPhotos: z.array(z.object({ photo_id: z.string(), visible_facts: z.array(z.string()), interpretation: z.string(), uncertainty: z.string(), imageVersion: z.object({ previewSha256: z.string().nullable().optional(), pixelVersion: z.enum(['verified','unrecorded','unreadable']).optional(), sourceMatches: z.boolean().nullable().optional() }).optional() })).default([]), conversation: z.boolean().default(false), messages: z.array(z.object({ id: z.number().int(), role: z.enum(['user', 'assistant']), content: z.string(), createdAt: z.string() })).default([]), draftEvidence: z.array(z.object({ draftId: z.string(), count: z.number().int().nonnegative(), observed: z.number().int().nonnegative(), unobserved: z.number().int().nonnegative(), pageTier: z.number().int().nonnegative(), detailTier: z.number().int().nonnegative(), otherVersion: z.number().int().nonnegative(), otherTool: z.number().int().nonnegative(), crossContractPairs: z.number().int().nonnegative(), omitted: z.number().int().nonnegative(), scanned: z.number().int().nonnegative(), draftRevision: z.number().int().positive().nullable(), perPhoto: z.array(z.object({ photoId: z.string(), contract: z.enum(['current_page_tier','current_detail_tier','other_prompt_version','other_tool']).nullable(), used: z.boolean(), pixelVersion: z.enum(['mismatch','unreadable','unrecorded']).nullable().optional(), originalStatus: z.enum(['missing','changed','matches','unchecked']).nullable().optional() })).max(100).default([]), basisSummary: z.boolean().optional() })).max(50).default([]), budgetIssue: z.object({ kind: z.enum(['images','cost','steps','time','rounds','context','tokens']), used: z.number().int().nonnegative(), requested: z.number().int().nonnegative(), limit: z.number().int().nonnegative(), stage: z.string(), requestNotSent: z.boolean() }).nullable().optional(), canvasTask: z.boolean().default(false), canvasSource: z.object({ boardId: z.string(), revision: z.number().int().positive(), selectedNodeIds: z.array(z.string()) }).nullable().default(null),
 boardDrafts: z.array(canvasBoardRecordSchema.transform(value => ({ id: value.id, projectId: value.project_id, title: value.title, state: value.state, owningTaskId: value.owning_task_id, revision: value.revision }))).default([]), id: z.string(), goal: z.string(), baseRevision: z.number().int(), providerId: z.string(), state: z.enum(['queued','planning','running','needs_input','review_ready','completed','paused','stale','failed','cancelled','interrupted']), reason: z.string(), budget, usage: z.object({ modelTurns: z.number().int(), toolCalls: z.number().int(), inputEstimate: z.number().int(), outputEstimate: z.number().int(), cloudImages: z.number().int().nonnegative(), costMinorUsd: z.number().int().nonnegative() }), elapsedMs: z.number(), question: z.string().nullable(), finalDraftIds: z.array(z.string()), steps: z.array(z.object({ id: z.number().int(), kind: z.string(), summary: z.string(), createdAt: z.string() })), createdAt: z.string() })),
 drafts: z.array(z.object({ id: z.string(), projectId: z.string(), taskId: z.string(), title: z.string(), baseRevision: z.number().int(), revision: z.number().int(), state: z.enum(['pending','applied','discarded']), stale: z.boolean(), summary: z.string(), items: z.array(z.object({ photo_id: z.string(), proposed_action: z.enum(['keep','reject','undecided']), position: z.number().int(), rationale: z.string() })), createdAt: z.string() })),
});
const reply = z.object({ ok: z.boolean(), data: z.object({ taskId: z.string() }).nullable(), error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean() }).nullable(), revision: z.number().nullable(), requestId: z.string() });
// 本地重算的确认：只有计数和版本号，观察正文一律不从这里走回界面。
const evidenceRecheck = z.object({ draftId: z.string().min(1), draftRevision: z.number().int().positive().nullable(), count: z.number().int().nonnegative(), observed: z.number().int().nonnegative(), unobserved: z.number().int().nonnegative(), omitted: z.number().int().nonnegative() });
const consentView = z.object({ id: z.number().int(), projectId: z.string(), mode: z.enum(['task', 'project', 'deny']), provider: z.string(), quota: z.number().int(), used: z.number().int(), grantedAt: z.string(), revokedAt: z.string().nullable() });
const aiConfigSchema = z.object({
 hostedAccount: z.object({visionDetail:z.object({policy:z.literal('preview1400-high-v1'),maxEdge:z.literal(1400),maxPhotosPerTask:z.literal(4)}).optional(),recovery:queuedRecoveryConfig.optional(),enabled:z.boolean(),verified:z.boolean(),policy:z.literal("deepseek-flash-v1"),provider:z.literal("DeepSeek"),model:z.literal("deepseek-flash"),unit:z.literal("tokens"),granted:z.number().int().nonnegative(),spent:z.number().int().nonnegative(),reserved:z.number().int().nonnegative(),available:z.number().int().nonnegative()}).optional(),
 hostedError: z.string().optional(),
 observation: z.object({ modelId: z.string(), capabilities: z.array(z.enum(['vision', 'tool_calling', 'structured_output', 'instruction_following'])), capabilitySource: z.enum(['official_documentation', 'local_protocol_smoke', 'operator_declaration']).nullable(), configurationError: z.string().nullable() }).nullable(),
 consentProvider: z.string().regex(/^openai@[a-f0-9]{64}$/).nullable(),
 providerId: z.string(), modelId: z.string(), ready: z.boolean(), apiKeyTail: z.string().nullable(), keySource: z.string().nullable(), baseUrl: z.string(), promptVersion: z.string(),
 capabilities: z.array(z.enum(['vision', 'tool_calling', 'structured_output', 'instruction_following'])), capabilitySource: z.enum(['official_documentation', 'local_protocol_smoke', 'operator_declaration']).nullable(), configurationError: z.string().nullable(),
 consent: consentView.nullable(),
});
export class TauriAgentRepository implements AgentRepository {
 async chat(turns: import('../domain/agent').ChatTurn[], requestId: string, onDelta?: (fragment: string) => void) {
  // 编号由调用方在发送前生成并留存：回复在路上丢了，才凭同一个编号向服务端只读核对。
  if (!onDelta) return z.string().parse(await invoke('agent_chat', { turns, requestId }));
  // 传了回调就走流式命令：Rust 每读到一段增量就经这条 channel 送回，channel 只活在这次 invoke 内。
  // 网关没有流式路由时 Rust 内部沿用同一编号退回提交路径，这里只会收到 0 段增量和同一个最终结果。
  const channel = new Channel<string>();
  channel.onmessage = onDelta;
  return z.string().parse(await invoke('agent_chat_stream', { turns, requestId, onDelta: channel }));
 }
 async chatJob(requestId: string) {
  return chatJob.parse(await invoke('agent_chat_job', { requestId }));
 }
 async selectionRevisionContext(projectId: string, draftId: string, offset = 0) {
  const result = selectionRevisionSchema.nullable().parse(await invoke('selection_revision_context', {projectId,draftId,offset}));
  if (result && (result.draftId !== draftId || result.parent.id === draftId || (result.nextOffset !== null && result.nextOffset !== offset+30) || result.items.length > result.totalItems || result.reviews.some(review => review.photoNames.length !== review.photoIds.length || (review.kind === 'apply' && review.mode === null)))) throw new Error('修订依据与方案不一致。');
  return result ? {...result,items:withLocalRevisionPreviews(result.items),observations:withLocalRevisionPreviews(result.observations)} : null;
 }

 async recheckEvidence(projectId: string, draftId: string) {
  const receipt = evidenceRecheck.parse(await invoke('recheck_draft_evidence', { projectId, draftId }));
  // 确认单点名的方案必须就是刚点的那一份：对不上等于把别的方案的口径当成自己的。
  if (receipt.draftId !== draftId) throw new Error('重算回执与方案不一致。');
  return receipt;
 }

 async queryVisionResult(projectId: string, taskId: string) { return visionRecovery.parse(await invoke('query_vision_result', { projectId, taskId })); }
 async query(projectId: string) { return projection.parse(await invoke('get_agent_projection', { projectId })); }
 async request(projectId: string, command: TaskCommand) {
  const request = { requestId: crypto.randomUUID(), projectId, command };
  let raw: unknown;
  try { raw = await invoke('agent_request', { request }); }
  catch (error) { if (typeof error === 'object' && error && 'code' in error) throw error; raw = await invoke('agent_request', { request }); }
  const parsed = reply.parse(raw);
  if (!parsed.ok || !parsed.data) throw Object.assign(new Error(parsed.error?.message ?? '任务请求失败'), { code: parsed.error?.code });
  return parsed.data.taskId;
 }
 async subscribe(projectId: string, onChange: () => void) {
  const handler = (payload: unknown) => { if (z.object({ projectId: z.string() }).safeParse(payload).data?.projectId === projectId) onChange(); };
  const first = await listen(AGENT_TASK_UPDATED_EVENT, event => handler(event.payload));
  try { const second = await listen(DRAFT_UPDATED_EVENT, event => handler(event.payload)); return () => { first(); second(); }; } catch (error) { first(); throw error; }
 }
 async aiConfig(projectId: string) { return aiConfigSchema.parse(await invoke('get_ai_config', { projectId })); }
 async setConsent(projectId: string, mode: 'task' | 'project' | 'deny', quota: number, recipient?: string | null) {
  if (mode !== 'deny' && !/^openai@[a-f0-9]{64}$/.test(recipient ?? '')) throw new Error('服务授权标识缺失，请刷新模型配置');
  const request = { requestId: crypto.randomUUID(), projectId, mode, provider: mode === 'deny' ? 'openai' : recipient!, quota };
  let raw: unknown;
  try { raw = await invoke('set_privacy_consent', { request }); }
  catch (error) { if (typeof error === 'object' && error && 'code' in error) throw Object.assign(new Error('message' in error ? String(error.message) : '授权失败'), { code: error.code }); raw = await invoke('set_privacy_consent', { request }); }
  const receipt = consentView.parse(raw);
  if (receipt.projectId !== projectId || receipt.provider !== request.provider || receipt.mode !== mode || receipt.quota !== quota) throw new Error('授权回执身份不匹配');
  return receipt;
 }
}
