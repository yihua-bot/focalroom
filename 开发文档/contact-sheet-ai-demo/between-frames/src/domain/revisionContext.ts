export interface HistoricalPhotoObservation {
  photoId: string; photoHash: string; photoName: string; thumbnailPath: string | null;
  modelId: string; promptVersion: string; taskId: string; callId: string;
  visibleFacts: string[]; interpretation: string; uncertainty: string; currentHashMatches: boolean;
}
export interface SelectionRevisionContext {
  draftId: string; parent: { kind: 'selection'; id: string; revision: number };
  parentTitle: string; parentSummary: string; parentState: 'pending'|'applied'|'discarded'; parentBaseRevision: number;
  items: Array<{ photoId: string; photoHash: string; photoName: string; position: number; action: 'keep'|'reject'|'undecided'; rationale: string; currentHashMatches: boolean; thumbnailPath: string | null }>;
  feedback: Array<{ messageId: number; role: 'user'|'assistant'; content: string; createdAt: string }>;
  reviews: Array<{ commandId: string; kind: 'apply'|'reorder'|'rebase'|'discard'; createdAt: string; mode: 'partial'|'full'|null; photoIds: string[]; photoNames: string[]; partial: boolean; undoneAtDelegation: boolean | null }>;
  observations: HistoricalPhotoObservation[];
  totalItems: number; totalFeedback: number; totalReviews: number; totalObservations: number; nextOffset: number | null; hasEarlier: boolean;
}
