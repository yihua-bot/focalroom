import { z } from 'zod';

const failure = z.object({ code: z.string().min(1), message: z.string() });

// 结论与数字必须对得上：说"取到了"就不能是零条，说"没有资料"就不能是服务没答上来，
// 说"没跑成"就必须带着失败原因。对不上宁可整份拒收，也不让界面替回执圆场。
export const knowledgeSearchSchema = z.object({
  callId: z.string().min(1).max(160),
  tool: z.enum([
    'search_photography_knowledge',
    'search_photographers',
    'get_photobook_structure',
    'compare_photography_series',
  ]),
  query: z.string().max(200),
  returned: z.number().int().nonnegative(),
  localMatched: z.number().int().nonnegative().nullable().optional(),
  cloudAvailable: z.boolean().nullable().optional(),
  cloudError: failure.nullable().optional(),
  error: failure.nullable().optional(),
  outcome: z.enum(['found', 'no_match', 'unavailable', 'failed']),
}).superRefine((value, context) => {
  const reject = (message: string) => context.addIssue({ code: 'custom', message });
  if (value.outcome === 'found' && value.returned === 0) reject('Found outcome without any returned entry');
  if (value.outcome !== 'found' && value.returned > 0) reject('Empty outcome with returned entries');
  if (value.outcome === 'unavailable' && value.cloudAvailable !== false) reject('Unavailable outcome without a failed cloud call');
  if (value.outcome === 'no_match' && value.cloudAvailable === false) reject('No-match outcome hides an unanswered cloud call');
  if (value.outcome === 'failed' && value.error == null) reject('Failed outcome without a reason');
  if (value.outcome !== 'failed' && value.error != null) reject('Failure reason on a completed search');
});