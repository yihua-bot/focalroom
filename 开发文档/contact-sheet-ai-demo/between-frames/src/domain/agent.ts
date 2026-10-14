import type { Project } from './models';
export interface AgentBudget { maxModelTurns: number; maxToolCalls: number; maxDurationSeconds: number; maxCloudImages: number; maxCostMinorUsd: number; maxContextTokens: number; maxTotalTokens: number; maxOutputTokens: number }
export const defaultAgentBudget: AgentBudget = { maxModelTurns: 16, maxToolCalls: 40, maxDurationSeconds: 480, maxCloudImages: 0, maxCostMinorUsd: 100, maxContextTokens: 48000, maxTotalTokens: 500000, maxOutputTokens: 8192 };
export type TaskState = 'queued' | 'planning' | 'running' | 'needs_input' | 'review_ready' | 'completed' | 'paused' | 'stale' | 'failed' | 'cancelled' | 'interrupted';
export interface AgentTask { id: string; goal: string; baseRevision: number; state: TaskState; reason: string; budget: AgentBudget; usage: { modelTurns: number; toolCalls: number; inputEstimate: number; outputEstimate: number; cloudImages: number; costMinorUsd: number }; elapsedMs: number; question: string | null; finalDraftIds: string[]; steps: { id: number; kind: string; summary: string; createdAt: string }[]; createdAt: string }
export interface DraftItem { photo_id: string; proposed_action: 'keep' | 'reject' | 'undecided'; position: number; rationale: string }
export interface SelectionDraft { id: string; projectId: string; taskId: string; title: string; baseRevision: number; revision: number; state: 'pending' | 'applied' | 'discarded'; stale: boolean; summary: string; items: DraftItem[]; createdAt: string }
export interface AgentProjection { tasks: AgentTask[]; drafts: SelectionDraft[] }
export type TaskCommand =
 | { type: 'StartTask'; goal: string; selectedPhotoIds: string[]; targetCount: number; budget: AgentBudget }
 | { type: 'PauseTask' | 'CancelTask' | 'CloseTask'; taskId: string }
 | { type: 'ResumeTask'; taskId: string; budget: AgentBudget }
 | { type: 'SubmitFeedback'; taskId: string; answer: string };
export interface AgentRepository { query(projectId: string): Promise<AgentProjection>; request(projectId: string, command: TaskCommand): Promise<string>; subscribe(projectId: string, onChange: () => void): Promise<() => void> }
export interface DraftRepository { agent: AgentRepository; applyDraft(projectId: string, draft: SelectionDraft, approvedPhotoIds: string[], mode: 'partial' | 'full'): Promise<Project>; rebaseDraft(projectId: string, draft: SelectionDraft): Promise<Project>; updateDraft(projectId: string, draft: SelectionDraft, orderedPhotoIds: string[]): Promise<Project>; discardDraft(projectId: string, draft: SelectionDraft): Promise<Project> }
