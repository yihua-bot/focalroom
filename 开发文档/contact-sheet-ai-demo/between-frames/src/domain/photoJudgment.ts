export interface JudgmentMethod {
 callId: string; citationId: string; claimIndex?: number; title: string; passage: string;
 source?: string | null; institution?: string | null; author?: string | null;
 license?: string | null; version?: string | number | null;
 kind: 'original_method' | 'retrieved_claim'; reviewStatus: string | null;
 disputed: boolean; excerpt: boolean; matchReason?: import('./knowledgeMatch').KnowledgeMatch | null;
}
export interface PhotoJudgment {
 id: string; photoId: string; observationCallId: string;
 judgment: string; reasoning: string; uncertainty: string; observationUncertainty: string;
 facts: {index: number; text: string}[]; methods: JudgmentMethod[];
 imageStatus: 'verified' | 'mismatch' | 'unreadable' | 'unavailable' | 'unrecorded'; inference: true;
}
