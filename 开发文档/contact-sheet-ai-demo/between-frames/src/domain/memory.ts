export interface MemoryEvidence { id: number; projectId: string; photoId: string | null; kind: 'keep' | 'reject' | 'comparison' | 'draft_apply' | 'draft_reject' | 'note'; detail: Record<string, unknown>; createdAt: string }
export interface MemoryHypothesis { id: number; projectId: string; scope: 'project_local' | 'cross_project'; statement: string; evidenceIds: number[]; counterexampleIds: number[]; status: 'unconfirmed' | 'confirmed' | 'corrected' | 'retired'; confidence: 'tentative' | 'moderate'; userCorrection: string | null; disabled: boolean; createdAt: string; updatedAt: string }
export interface MemoryProjection { evidence: MemoryEvidence[]; hypotheses: MemoryHypothesis[]; notice: string }
export interface MemoryRepository {
  query(projectId: string): Promise<MemoryProjection>;
  correct(projectId: string, hypothesisId: number, correction: string): Promise<MemoryHypothesis>;
  setEnabled(projectId: string, hypothesisId: number, enabled: boolean): Promise<MemoryHypothesis>;
}
