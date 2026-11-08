import { z } from 'zod';
import { knowledgeMatchSchema } from './knowledgeMatchSchema';
export const photoJudgmentSchema = z.object({
 id:z.string(),photoId:z.string(),observationCallId:z.string(),
 judgment:z.string(),reasoning:z.string(),uncertainty:z.string(),observationUncertainty:z.string(),
 facts:z.array(z.object({index:z.number().int().min(0).max(2),text:z.string()})).min(1).max(3),
 methods:z.array(z.object({callId:z.string(),citationId:z.string(),claimIndex:z.number().int().nonnegative().optional(),
  title:z.string(),passage:z.string(),source:z.string().nullable().optional(),institution:z.string().nullable().optional(),
  author:z.string().nullable().optional(),license:z.string().nullable().optional(),version:z.union([z.string(),z.number()]).nullable().optional(),
  kind:z.enum(['original_method','retrieved_claim']),reviewStatus:z.string().nullable(),disputed:z.boolean(),excerpt:z.boolean(),matchReason:knowledgeMatchSchema.nullable().optional(),
 })).max(3),
 imageStatus:z.enum(['verified','mismatch','unreadable','unavailable','unrecorded']),inference:z.literal(true),
});
