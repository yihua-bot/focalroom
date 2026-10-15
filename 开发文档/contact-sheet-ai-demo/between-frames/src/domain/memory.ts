export interface MemoryEvidence { id: number; projectId: string; photoId: string | null; kind: 'keep' | 'reject' | 'comparison' | 'draft_apply' | 'draft_reject' | 'note'; detail: Record<string, unknown>; createdAt: string; excluded: boolean }
export interface MemoryHypothesis { id: number; projectId: string; scope: 'project_local' | 'cross_project'; statement: string; evidenceIds: number[]; counterexampleIds: number[]; status: 'unconfirmed' | 'confirmed' | 'corrected' | 'retired'; confidence: 'tentative' | 'moderate'; userCorrection: string | null; disabled: boolean; createdAt: string; updatedAt: string }
export interface MemoryProjection { evidence: MemoryEvidence[]; hypotheses: MemoryHypothesis[]; notice: string }
export type MemoryCommand =
  | { type: 'SetEvidenceExcluded'; evidenceId: number; excluded: boolean }
  | { type: 'ConfirmHypothesis' | 'RetireHypothesis' | 'RestoreHypothesis'; hypothesisId: number }
  | { type: 'CorrectHypothesis'; hypothesisId: number; correction: string }
  | { type: 'SetHypothesisEnabled'; hypothesisId: number; enabled: boolean }
  | { type: 'DeleteHypothesis'; hypothesisId: number }
  | { type: 'DeleteEvidence'; evidenceId: number }
  | { type: 'PurgeObservationCache' };
export interface MemoryReceipt { requestId: string; changed: boolean; memoryRevision: number }
export interface MemoryRepository {
  query(projectId: string, query?: string): Promise<MemoryProjection>;
  request(projectId: string, command: MemoryCommand): Promise<MemoryReceipt>;
  correct(projectId: string, hypothesisId: number, correction: string): Promise<MemoryHypothesis>;
  setEnabled(projectId: string, hypothesisId: number, enabled: boolean): Promise<MemoryHypothesis>;
  deleteHypothesis(projectId: string, hypothesisId: number): Promise<void>;
  deleteEvidence(projectId: string, evidenceId: number): Promise<void>;
  purgeObservationCache(projectId: string): Promise<MemoryReceipt>;
}
