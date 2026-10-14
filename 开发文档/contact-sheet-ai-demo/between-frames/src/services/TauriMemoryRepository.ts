import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { MemoryRepository } from '../domain/memory';

const evidence = z.object({
 id: z.number().int(), projectId: z.string(), photoId: z.string().nullable(),
 kind: z.enum(['keep', 'reject', 'comparison', 'draft_apply', 'draft_reject', 'note']),
 detail: z.record(z.string(), z.unknown()), createdAt: z.string(),
});
const hypothesis = z.object({
 id: z.number().int(), projectId: z.string(), scope: z.enum(['project_local', 'cross_project']),
 statement: z.string(), evidenceIds: z.array(z.number().int()), counterexampleIds: z.array(z.number().int()),
 status: z.enum(['unconfirmed', 'confirmed', 'corrected', 'retired']), confidence: z.enum(['tentative', 'moderate']),
 userCorrection: z.string().nullable(), disabled: z.boolean(), createdAt: z.string(), updatedAt: z.string(),
});
const projection = z.object({ evidence: z.array(evidence), hypotheses: z.array(hypothesis), notice: z.string() });

export class TauriMemoryRepository implements MemoryRepository {
 async query(projectId: string) { return projection.parse(await invoke('get_memory_projection', { projectId })); }
 async correct(projectId: string, hypothesisId: number, correction: string) {
  return hypothesis.parse(await invoke('correct_hypothesis', { projectId, hypothesisId, correction }));
 }
 async setEnabled(projectId: string, hypothesisId: number, enabled: boolean) {
  return hypothesis.parse(await invoke('set_hypothesis_enabled', { projectId, hypothesisId, enabled }));
 }
}
