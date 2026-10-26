import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { MemoryCommand, MemoryRepository } from '../domain/memory';

const evidence = z.object({
 id: z.number().int(), projectId: z.string(), photoId: z.string().nullable(),
 kind: z.enum(['keep', 'reject', 'comparison', 'draft_apply', 'draft_reject', 'note']),
 detail: z.record(z.string(), z.unknown()), createdAt: z.string(), excluded: z.boolean(),
});
const hypothesis = z.object({
 id: z.number().int(), projectId: z.string(), scope: z.enum(['project_local', 'cross_project']),
 statement: z.string(), evidenceIds: z.array(z.number().int()), counterexampleIds: z.array(z.number().int()),
 status: z.enum(['unconfirmed', 'confirmed', 'corrected', 'retired']), confidence: z.enum(['tentative', 'moderate']),
 userCorrection: z.string().nullable(), disabled: z.boolean(), createdAt: z.string(), updatedAt: z.string(),
});
const cloudAuthorMemory = z.object({
 id: z.string(), statement: z.string(), status: z.enum(['unconfirmed', 'confirmed', 'corrected', 'retired']),
 confidence: z.enum(['tentative', 'moderate']), project_id: z.string(), project_title: z.string(),
 created_at: z.string(), updated_at: z.string(),
}).transform(value => ({ id: value.id, statement: value.statement, status: value.status, confidence: value.confidence, projectId: value.project_id, projectTitle: value.project_title, createdAt: value.created_at, updatedAt: value.updated_at }));
const projection = z.object({ evidence: z.array(evidence), hypotheses: z.array(hypothesis), notice: z.string(), crossProjectEnabled: z.boolean().default(false), authorHypotheses: z.array(hypothesis).default([]), authorMemoryNotice: z.string().default('') });

const reply = z.object({ ok: z.boolean(), requestId: z.string(), data: z.object({ requestId: z.string(), changed: z.boolean(), memoryRevision: z.number().int().nonnegative() }).nullable(), error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean() }).nullable() });
export class TauriMemoryRepository implements MemoryRepository {
 async query(projectId: string, query = '') {
  const parsed = projection.parse(await invoke('get_memory_projection', { projectId, query }));
  if (parsed.evidence.some(e => e.projectId !== projectId) || parsed.hypotheses.some(h => h.projectId !== projectId || h.scope !== 'project_local')
    || parsed.authorHypotheses.some(h => h.projectId === projectId || h.scope !== 'cross_project' || h.status !== 'confirmed' || h.disabled || !h.evidenceIds.length)
    || (!parsed.crossProjectEnabled && parsed.authorHypotheses.length)) throw new Error('记忆投影范围不匹配');
  return parsed;
 }
 async request(projectId: string, command: MemoryCommand) {
  const request = { requestId: crypto.randomUUID(), projectId, command };
  let raw: unknown;
  try { raw = await invoke('execute_memory_command', { request }); }
  catch (error) { if (typeof error === 'object' && error && 'code' in error) throw error; raw = await invoke('execute_memory_command', { request }); }
  const parsed = reply.parse(raw);
  if (!parsed.ok || !parsed.data) throw Object.assign(new Error(parsed.error?.message ?? '记忆操作失败'), { code: parsed.error?.code });
  if (parsed.requestId !== request.requestId || parsed.data.requestId !== request.requestId) throw new Error('记忆回执身份不匹配');
  return parsed.data;
 }
 async correct(projectId: string, hypothesisId: number, correction: string) {
  await this.request(projectId, { type: 'CorrectHypothesis', hypothesisId, correction });
  return this.find(projectId, hypothesisId);
 }
 async setEnabled(projectId: string, hypothesisId: number, enabled: boolean) {
  await this.request(projectId, { type: 'SetHypothesisEnabled', hypothesisId, enabled });
  return this.find(projectId, hypothesisId);
 }
 async deleteHypothesis(projectId: string, hypothesisId: number) {
  await this.request(projectId, { type: 'DeleteHypothesis', hypothesisId });
 }
 async deleteEvidence(projectId: string, evidenceId: number) {
  await this.request(projectId, { type: 'DeleteEvidence', evidenceId });
 }
 async purgeObservationCache(projectId: string) {
  return this.request(projectId, { type: 'PurgeObservationCache' });
 }
 async cloudAuthorMemories(projectId: string) {
  return z.array(cloudAuthorMemory).parse(await invoke('cloud_author_memories', { projectId }));
 }
 private async find(projectId: string, id: number) {
  const item = (await this.query(projectId)).hypotheses.find(h => h.id === id);
  if (!item) throw new Error('假设不存在');
  return item;
 }
}
