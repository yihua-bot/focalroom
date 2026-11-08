import { z } from 'zod';

// 命中词最多列 8 个，但命中词数是真的总数；带回条数不可能超过命中总数。
// 这两条对不上就说明回执自相矛盾，宁可整份拒收也不让界面替它圆场。
export const knowledgeMatchSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('local_terms'),
    termCount: z.number().int().positive(),
    matchedTerms: z.array(z.string().min(1)).max(8),
    fields: z.array(z.enum(['title', 'tags', 'text'])).min(1).max(3),
    matched: z.number().int().positive().optional(),
    returned: z.number().int().positive().optional(),
  }).superRefine((value, context) => {
    if (value.matchedTerms.length > value.termCount) {
      context.addIssue({ code: 'custom', message: 'Listed match terms exceed the reported total' });
    }
    if (value.matched != null && value.returned != null && value.returned > value.matched) {
      context.addIssue({ code: 'custom', message: 'Returned entries exceed matched entries' });
    }
  }),
  z.object({ kind: z.literal('builtin_method') }),
  z.object({ kind: z.literal('cloud_search'), relevance: z.number().finite().nullable() }),
]);