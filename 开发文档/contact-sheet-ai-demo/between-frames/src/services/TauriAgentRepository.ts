import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { z } from 'zod';
import { canvasBoardRecordSchema } from '@betweenframes/contracts';
import { AGENT_TASK_UPDATED_EVENT, DRAFT_UPDATED_EVENT } from '../domain/commands.generated';
import type { AgentRepository, TaskCommand } from '../domain/agent';
const budget = z.object({ maxModelTurns: z.number().int(), maxToolCalls: z.number().int(), maxDurationSeconds: z.number().int(), maxCloudImages: z.number().int().min(0).max(100), maxCostMinorUsd: z.number().int(), maxContextTokens: z.number().int(), maxTotalTokens: z.number().int(), maxOutputTokens: z.number().int() });
const visionRecovery = z.object({ state: z.enum(['pending','completed','unknown','rejected','expired']), images: z.number().int().min(1).max(100) });
const queuedRecoveryConfig = z.discriminatedUnion('enabled', [z.object({enabled:z.literal(false)}),z.object({enabled:z.literal(true),policy:z.literal('encrypted-queue-v1'),maxQueueSeconds:z.literal(600)})]);
const projection = z.object({
 tasks: z.array(z.object({ queueCancellation: z.enum(['cancelled','already_sent','unconfirmed']).nullable().optional(), visionRecovery: visionRecovery.nullable().optional(), creativeReferences: z.array(z.object({ kind: z.enum(['knowledge','context']), label: z.string(), version: z.union([z.string(),z.number()]).nullable().optional(), citationId: z.string().optional(), passage: z.string().optional(), license: z.string().optional(), section: z.string().optional() })).default([]), draftLineage: z.array(z.object({ childId: z.string(), kind: z.string(), parent: z.object({ kind: z.enum(['board','selection']), id: z.string(), revision: z.number().int().positive() }), reason: z.string() })).default([]), preferenceProposals: z.array(z.object({ id: z.string(), statement: z.string(), applicability: z.string(), photoIds: z.array(z.string()), sources: z.array(z.object({ messageId: z.number().int(), text: z.string(), createdAt: z.string() })), state: z.enum(['pending','saved','removed','dismissed']), createdAt: z.string() })).default([]), inspectedPhotos: z.array(z.object({ photo_id: z.string(), visible_facts: z.array(z.string()), interpretation: z.string(), uncertainty: z.string() })).default([]), conversation: z.boolean().default(false), messages: z.array(z.object({ id: z.number().int(), role: z.enum(['user', 'assistant']), content: z.string(), createdAt: z.string() })).default([]), budgetIssue: z.object({ kind: z.enum(['images','cost','steps','time','rounds','context','tokens']), used: z.number().int().nonnegative(), requested: z.number().int().nonnegative(), limit: z.number().int().nonnegative(), stage: z.string(), requestNotSent: z.boolean() }).nullable().optional(), canvasTask: z.boolean().default(false), canvasSource: z.object({ boardId: z.string(), revision: z.number().int().positive(), selectedNodeIds: z.array(z.string()) }).nullable().default(null),
 boardDrafts: z.array(canvasBoardRecordSchema.transform(value => ({ id: value.id, projectId: value.project_id, title: value.title, state: value.state, owningTaskId: value.owning_task_id, revision: value.revision }))).default([]), id: z.string(), goal: z.string(), baseRevision: z.number().int(), providerId: z.string(), state: z.enum(['queued','planning','running','needs_input','review_ready','completed','paused','stale','failed','cancelled','interrupted']), reason: z.string(), budget, usage: z.object({ modelTurns: z.number().int(), toolCalls: z.number().int(), inputEstimate: z.number().int(), outputEstimate: z.number().int(), cloudImages: z.number().int().nonnegative(), costMinorUsd: z.number().int().nonnegative() }), elapsedMs: z.number(), question: z.string().nullable(), finalDraftIds: z.array(z.string()), steps: z.array(z.object({ id: z.number().int(), kind: z.string(), summary: z.string(), createdAt: z.string() })), createdAt: z.string() })),
 drafts: z.array(z.object({ id: z.string(), projectId: z.string(), taskId: z.string(), title: z.string(), baseRevision: z.number().int(), revision: z.number().int(), state: z.enum(['pending','applied','discarded']), stale: z.boolean(), summary: z.string(), items: z.array(z.object({ photo_id: z.string(), proposed_action: z.enum(['keep','reject','undecided']), position: z.number().int(), rationale: z.string() })), createdAt: z.string() })),
});
const reply = z.object({ ok: z.boolean(), data: z.object({ taskId: z.string() }).nullable(), error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean() }).nullable(), revision: z.number().nullable(), requestId: z.string() });
const consentView = z.object({ id: z.number().int(), projectId: z.string(), mode: z.enum(['task', 'project', 'deny']), provider: z.string(), quota: z.number().int(), used: z.number().int(), grantedAt: z.string(), revokedAt: z.string().nullable() });
const aiConfigSchema = z.object({
 hostedAccount: z.object({recovery:queuedRecoveryConfig.optional(),enabled:z.boolean(),verified:z.boolean(),policy:z.literal("deepseek-flash-v1"),provider:z.literal("DeepSeek"),model:z.literal("deepseek-flash"),unit:z.literal("tokens"),granted:z.number().int().nonnegative(),spent:z.number().int().nonnegative(),reserved:z.number().int().nonnegative(),available:z.number().int().nonnegative()}).optional(),
 hostedError: z.string().optional(),
 observation: z.object({ modelId: z.string(), capabilities: z.array(z.enum(['vision', 'tool_calling', 'structured_output', 'instruction_following'])), capabilitySource: z.enum(['official_documentation', 'local_protocol_smoke', 'operator_declaration']).nullable(), configurationError: z.string().nullable() }).nullable(),
 consentProvider: z.string().regex(/^openai@[a-f0-9]{64}$/).nullable(),
 providerId: z.string(), modelId: z.string(), ready: z.boolean(), apiKeyTail: z.string().nullable(), keySource: z.string().nullable(), baseUrl: z.string(), promptVersion: z.string(),
 capabilities: z.array(z.enum(['vision', 'tool_calling', 'structured_output', 'instruction_following'])), capabilitySource: z.enum(['official_documentation', 'local_protocol_smoke', 'operator_declaration']).nullable(), configurationError: z.string().nullable(),
 consent: consentView.nullable(),
});
export class TauriAgentRepository implements AgentRepository {
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
