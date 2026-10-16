import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { z } from 'zod';
import { AGENT_TASK_UPDATED_EVENT, DRAFT_UPDATED_EVENT } from '../domain/commands.generated';
import type { AgentRepository, TaskCommand } from '../domain/agent';
const budget = z.object({ maxModelTurns: z.number().int(), maxToolCalls: z.number().int(), maxDurationSeconds: z.number().int(), maxCloudImages: z.number().int().min(0).max(100), maxCostMinorUsd: z.number().int(), maxContextTokens: z.number().int(), maxTotalTokens: z.number().int(), maxOutputTokens: z.number().int() });
const projection = z.object({
 tasks: z.array(z.object({ id: z.string(), goal: z.string(), baseRevision: z.number().int(), providerId: z.string(), state: z.enum(['queued','planning','running','needs_input','review_ready','completed','paused','stale','failed','cancelled','interrupted']), reason: z.string(), budget, usage: z.object({ modelTurns: z.number().int(), toolCalls: z.number().int(), inputEstimate: z.number().int(), outputEstimate: z.number().int(), cloudImages: z.number().int().nonnegative(), costMinorUsd: z.number().int().nonnegative() }), elapsedMs: z.number(), question: z.string().nullable(), finalDraftIds: z.array(z.string()), steps: z.array(z.object({ id: z.number().int(), kind: z.string(), summary: z.string(), createdAt: z.string() })), createdAt: z.string() })),
 drafts: z.array(z.object({ id: z.string(), projectId: z.string(), taskId: z.string(), title: z.string(), baseRevision: z.number().int(), revision: z.number().int(), state: z.enum(['pending','applied','discarded']), stale: z.boolean(), summary: z.string(), items: z.array(z.object({ photo_id: z.string(), proposed_action: z.enum(['keep','reject','undecided']), position: z.number().int(), rationale: z.string() })), createdAt: z.string() })),
});
const reply = z.object({ ok: z.boolean(), data: z.object({ taskId: z.string() }).nullable(), error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean() }).nullable(), revision: z.number().nullable(), requestId: z.string() });
const consentView = z.object({ id: z.number().int(), projectId: z.string(), mode: z.enum(['task', 'project', 'deny']), provider: z.string(), quota: z.number().int(), used: z.number().int(), grantedAt: z.string(), revokedAt: z.string().nullable() });
const aiConfigSchema = z.object({
 providerId: z.string(), modelId: z.string(), ready: z.boolean(), apiKeyTail: z.string().nullable(), keySource: z.string().nullable(), baseUrl: z.string(), promptVersion: z.string(),
 capabilities: z.array(z.enum(['vision', 'tool_calling', 'structured_output', 'instruction_following'])), capabilitySource: z.enum(['official_documentation', 'local_protocol_smoke', 'operator_declaration']).nullable(), configurationError: z.string().nullable(),
 consent: consentView.nullable(),
});
export class TauriAgentRepository implements AgentRepository {
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
 async setConsent(projectId: string, mode: 'task' | 'project' | 'deny', quota: number) {
  return consentView.parse(await invoke('set_privacy_consent', { request: { requestId: crypto.randomUUID(), projectId, mode, provider: 'openai', quota } }));
 }
}
