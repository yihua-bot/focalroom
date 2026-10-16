import { createHash, createHmac } from 'node:crypto';
import { z } from 'zod';

const id = z.string().min(1).max(160).regex(/^\S+$/u);
const text = z.string().trim().min(1).max(4000);
const count = z.number().int().nonnegative().safe();
const hash = z.string().regex(/^[a-f0-9]{64}$/);
export const genres = ['documentary', 'family', 'street', 'photobook', 'long_project', 'collaborative'] as const;
export const phases = ['first_curation', 'third_after_feedback'] as const;
export const dimensions = ['visual_facts', 'creative_intent', 'sequence_rhythm', 'explanation', 'collaboration'] as const;

const taskSchema = z.strictObject({
  id, creatorId: id, goal: text, genre: z.enum(genres), phase: z.enum(phases),
  targetCount: count.positive(), provenance: z.enum(['photographer', 'synthetic']),
  photos: z.array(z.strictObject({ id, sha256: hash })).min(1).max(300),
  authorization: z.strictObject({
    holderId: id, evidenceRef: text, photoIds: z.array(id).min(1),
    cloudPreviewProviders: z.array(id),
  }).nullable(),
  feedbackEvidenceRef: text.nullable(),
});
const runSchema = z.strictObject({
  id, taskId: id, system: z.enum(['between_frames', 'human_baseline', 'competitor']),
  version: text, model: text.nullable(), promptVersion: text.nullable(),
  state: z.enum(['review_ready', 'failed', 'paused', 'needs_input', 'cancelled']),
  orderedPhotoIds: z.array(id), withinBudget: z.boolean(),
  elapsedMs: count, actualTokens: count.nullable(),
  estimatedCostMinorUsd: count.nullable(), billedCostMinorUsd: count.nullable(),
  sourceEvidenceRef: text, explanation: text,
  followUp: z.strictObject({ humanModificationCount: count, reviewedChanges: count, acceptedChanges: count, evidenceRef: text }).refine(v => v.acceptedChanges <= v.reviewedChanges).nullable(),
  trace: z.array(z.strictObject({ callId: id, tool: id, photoIds: z.array(id), evidenceRef: text })),
  audit: z.strictObject({
    evidenceRef: text, formalBeforeSha256: hash, formalAfterSha256: hash,
    originalsBeforeSha256: hash, originalsAfterSha256: hash, illegalWriteCount: count,
  }).nullable(),
  cloudPreviewProvider: id.nullable(),
});
const score = z.number().int().min(1).max(5);
const reviewSchema = z.strictObject({
  caseId: id, candidateId: id, reviewerId: id,
  packetId: hash.nullable().default(null),
  // 旧记录未标来源时保持 unverified，不把角色名称当作真人身份凭据。
  reviewerKind: z.enum(['human', 'simulated_ai', 'unverified']).default('unverified'),
  reviewerModel: text.nullable().default(null),
  scores: z.strictObject({ visual_facts: score, creative_intent: score, sequence_rhythm: score, explanation: score, collaboration: score }),
  reason: text,
  factCheck: z.strictObject({ checkedClaims: count, incorrectClaims: count }).refine(v => v.incorrectClaims <= v.checkedClaims),
}).refine(review => review.reviewerKind === 'simulated_ai' ? review.reviewerModel !== null : review.reviewerModel === null,
  { message: 'AI 模拟须注明模型；真人/未核实记录不能附带模拟模型身份' });
export const inputSchema = z.strictObject({
  protocolVersion: z.literal('bf-m3-eval-1'), datasetId: id,
  // 在运行前登记所有 attempt ID；失败记录不能在汇总时静默移除。
  registeredRunIds: z.array(id), tasks: z.array(taskSchema), runs: z.array(runSchema),
  reviews: z.array(reviewSchema),
});
export type EvaluationInput = z.infer<typeof inputSchema>;
export type BlindPack = ReturnType<typeof createBlindPack>;

export function snapshotDigest(input: EvaluationInput) {
  const frozen = { ...input, reviews: [] };
  return createHash('sha256').update(JSON.stringify(frozen)).digest('hex');
}

function duplicate(values: string[]) { return new Set(values).size !== values.length; }
function sorted(values: string[]) { return [...values].sort(); }
function sameSet(a: string[], b: string[]) { return JSON.stringify(sorted(a)) === JSON.stringify(sorted(b)); }

export function createBlindPack(input: EvaluationInput, seed: string) {
  const key = (value: string) => createHmac('sha256', seed).update(value).digest('hex');
  const privateKey: { caseId: string; taskId: string; candidateId: string; runId: string }[] = [];
  const cases = [...input.tasks].sort((a, b) => key(`task:${a.id}`).localeCompare(key(`task:${b.id}`)))
    .map((task, index) => {
      const caseId = `case-${index + 1}`;
      const candidates = input.runs.filter(r => r.taskId === task.id && r.state === 'review_ready')
        .sort((a, b) => key(`run:${a.id}`).localeCompare(key(`run:${b.id}`)))
        .map((run, candidateIndex) => {
          const candidateId = `candidate-${candidateIndex + 1}`;
          privateKey.push({ caseId, taskId: task.id, candidateId, runId: run.id });
          return { candidateId, orderedPhotoIds: run.orderedPhotoIds, explanation: run.explanation };
        });
      return { caseId, goal: task.goal, targetCount: task.targetCount,
        photos: task.photos.map(p => ({ photoId: p.id, previewFilename: `${p.sha256}-1400.jpg` })), candidates };
    });
  return { publicPack: { protocolVersion: input.protocolVersion, packetId: key(`packet:${snapshotDigest(input)}`), cases }, privateKey };
}

export function evaluate(raw: unknown, seed: string) {
  const input = inputSchema.parse(raw);
  const pack = createBlindPack(input, seed);
  const issues: { code: string; reference: string }[] = [];
  const add = (code: string, reference: string) => issues.push({ code, reference });
  if (duplicate(input.tasks.map(t => t.id))) add('DUPLICATE_TASK_ID', input.datasetId);
  if (duplicate(input.runs.map(r => r.id)) || duplicate(input.registeredRunIds)) add('DUPLICATE_RUN_ID', input.datasetId);
  if (!sameSet(input.registeredRunIds, input.runs.map(r => r.id))) add('REGISTERED_RUNS_MISSING_OR_EXTRA', input.datasetId);
  if (input.tasks.length < 30) add('NEED_30_REAL_TASKS', input.datasetId);
  for (const genre of genres) if (!input.tasks.some(t => t.genre === genre)) add('MISSING_GENRE', genre);
  for (const phase of phases) if (!input.tasks.some(t => t.phase === phase)) add('MISSING_PHASE', phase);
  const fingerprints = new Set<string>();
  for (const task of input.tasks) {
    if (task.provenance !== 'photographer') add('SYNTHETIC_NOT_QUALITY_EVIDENCE', task.id);
    if (task.photos.length < 80 || task.targetCount > task.photos.length) add('INVALID_TASK_SIZE', task.id);
    if (duplicate(task.photos.map(p => p.id))) add('DUPLICATE_PHOTO_ID', task.id);
    if (duplicate(task.photos.map(p => p.sha256))) add('DUPLICATE_SOURCE_PHOTO', task.id);
    const fingerprint = JSON.stringify([task.goal.trim(), sorted(task.photos.map(p => p.sha256)), task.phase]);
    if (fingerprints.has(fingerprint)) add('DUPLICATE_TASK_CONTENT', task.id);
    fingerprints.add(fingerprint);
    if (!task.authorization || !sameSet(task.authorization.photoIds, task.photos.map(p => p.id))) add('AUTHORIZATION_SCOPE_MISSING_OR_MISMATCH', task.id);
    if (task.phase === 'third_after_feedback' && !task.feedbackEvidenceRef) add('MISSING_FEEDBACK_EVIDENCE', task.id);
    for (const system of ['between_frames', 'human_baseline', 'competitor'] as const) {
      if (!input.runs.some(r => r.taskId === task.id && r.system === system)) add('MISSING_COMPARISON_RUN', `${task.id}:${system}`);
    }
  }
  let illegalWrites = 0, invalidRefs = 0, completed = 0;
  const completions = new Map<string, boolean>();
  for (const run of input.runs) {
    const task = input.tasks.find(t => t.id === run.taskId);
    if (!task) { add('UNKNOWN_RUN_TASK', run.id); continue; }
    const allowed = new Set(task.photos.map(p => p.id));
    const refs = [...run.orderedPhotoIds, ...run.trace.flatMap(t => t.photoIds)];
    const invalid = refs.filter(photoId => !allowed.has(photoId)).length;
    invalidRefs += invalid;
    if (invalid) add('HALLUCINATED_OR_UNAUTHORIZED_PHOTO_REF', run.id);
    if (duplicate(run.trace.map(t => t.callId))) add('DUPLICATE_TOOL_CALL_ID', run.id);
    const cloudAuthorized = !run.cloudPreviewProvider || !!task.authorization?.cloudPreviewProviders.includes(run.cloudPreviewProvider);
    if (!cloudAuthorized) add('CLOUD_PREVIEW_NOT_AUTHORIZED', run.id);
    const audit = run.audit;
    const writes = audit ? audit.illegalWriteCount + Number(audit.formalBeforeSha256 !== audit.formalAfterSha256) + Number(audit.originalsBeforeSha256 !== audit.originalsAfterSha256) : 0;
    const isAgent = run.system === 'between_frames';
    const executionEvidence = !!run.model && !!run.promptVersion && run.trace.length > 0 && run.actualTokens !== null;
    if (isAgent) {
      illegalWrites += writes;
      if (!audit) add('MISSING_STATE_AUDIT', run.id);
      if (writes) add('ILLEGAL_WRITE_OR_HASH_CHANGE', run.id);
      if (!executionEvidence) add('MISSING_MODEL_EXECUTION_EVIDENCE', run.id);
    }
    const complete = run.state === 'review_ready' && run.withinBudget && run.orderedPhotoIds.length === task.targetCount
      && !duplicate(run.orderedPhotoIds) && invalid === 0 && !!task.authorization && cloudAuthorized
      && sameSet(task.authorization.photoIds, task.photos.map(p => p.id))
      && (!isAgent || (!!audit && writes === 0 && executionEvidence));
    completions.set(run.id, complete);
    if (run.system === 'between_frames' && complete) completed++;
    if (run.state === 'review_ready' && !complete) add('INVALID_REVIEWABLE_RESULT', run.id);
  }
  const reviewKeys = new Set<string>();
  const validReviews: typeof input.reviews = [];
  for (const review of input.reviews) {
    if (review.packetId !== pack.publicPack.packetId) { add('REVIEW_BATCH_MISSING_OR_MISMATCH', review.reviewerId); continue; }
    const mapping = pack.privateKey.find(k => k.caseId === review.caseId && k.candidateId === review.candidateId);
    const reviewKey = JSON.stringify([review.caseId, review.candidateId, review.reviewerId]);
    if (!mapping) { add('UNKNOWN_REVIEW_CANDIDATE', reviewKey); continue; }
    if (reviewKeys.has(reviewKey)) { add('DUPLICATE_REVIEW', reviewKey); continue; }
    reviewKeys.add(reviewKey);
    validReviews.push(review);
  }
  const identities = new Map<string, Set<string>>();
  for (const review of validReviews) {
    const identity = JSON.stringify([review.reviewerKind, review.reviewerModel]);
    identities.set(review.reviewerId, new Set([...(identities.get(review.reviewerId) ?? []), identity]));
  }
  const conflicted = new Set([...identities].filter(([, sources]) => sources.size > 1).map(([reviewerId]) => reviewerId));
  for (const reviewerId of conflicted) add('REVIEWER_PROVENANCE_CONFLICT', reviewerId);
  const humanReviews = validReviews.filter(r => r.reviewerKind === 'human' && !conflicted.has(r.reviewerId));
  const simulatedReviews = validReviews.filter(r => r.reviewerKind === 'simulated_ai' && !conflicted.has(r.reviewerId));
  const unverifiedReviews = validReviews.filter(r => r.reviewerKind === 'unverified' && !conflicted.has(r.reviewerId));
  for (const review of unverifiedReviews) add('REVIEW_SOURCE_UNVERIFIED', review.reviewerId);
  for (const mapping of pack.privateKey) {
    const task = input.tasks.find(t => t.id === mapping.taskId)!;
    const reviews = humanReviews.filter(r => r.caseId === mapping.caseId && r.candidateId === mapping.candidateId);
    if (!reviews.some(r => r.reviewerId === task.creatorId) || !reviews.some(r => r.reviewerId !== task.creatorId)) add('NEED_AUTHOR_AND_INDEPENDENT_REVIEW', mapping.runId);
  }
  const agentRuns = input.runs.filter(r => r.system === 'between_frames');
  const sumKnown = (field: 'billedCostMinorUsd' | 'estimatedCostMinorUsd') => {
    const known = agentRuns.filter(r => r[field] !== null);
    return { total: known.reduce((sum, r) => sum + r[field]!, 0), knownRuns: known.length, totalRuns: agentRuns.length };
  };
  const summarizeDimensions = (reviews: typeof input.reviews) => Object.fromEntries(dimensions.map(dimension => [dimension, {
    mean: reviews.length ? reviews.reduce((sum, r) => sum + r.scores[dimension], 0) / reviews.length : null,
    count: reviews.length,
  }]));
  const summarizeFacts = (reviews: typeof input.reviews) => {
    const checkedClaims = reviews.reduce((sum, r) => sum + r.factCheck.checkedClaims, 0);
    const incorrectClaims = reviews.reduce((sum, r) => sum + r.factCheck.incorrectClaims, 0);
    return { checkedClaims, incorrectClaims, rate: checkedClaims ? incorrectClaims / checkedClaims : null };
  };
  // 来源、版本、阶段分开汇总；跨系统平均分不用于宣称优劣。
  const scoreByRun = (source: typeof input.reviews) => pack.privateKey.map(mapping => {
    const run = input.runs.find(r => r.id === mapping.runId)!;
    const reviews = source.filter(r => r.caseId === mapping.caseId && r.candidateId === mapping.candidateId);
    return { runId: run.id, taskId: run.taskId, system: run.system, version: run.version, model: run.model, promptVersion: run.promptVersion,
      phase: input.tasks.find(t => t.id === run.taskId)!.phase,
      dimensions: summarizeDimensions(reviews) };
  });
  const followedUp = agentRuns.flatMap(r => r.followUp ? [r.followUp] : []);
  const reviewedChanges = followedUp.reduce((sum, f) => sum + f.reviewedChanges, 0);
  const acceptedChanges = followedUp.reduce((sum, f) => sum + f.acceptedChanges, 0);
  const configurations = new Map<string, typeof agentRuns>();
  for (const run of agentRuns) {
    const phase = input.tasks.find(t => t.id === run.taskId)?.phase ?? null;
    const key = JSON.stringify([run.version, run.model, run.promptVersion, phase]);
    configurations.set(key, [...(configurations.get(key) ?? []), run]);
  }
  return {
    input, pack,
    report: {
      protocolVersion: input.protocolVersion, datasetId: input.datasetId,
      status: issues.length ? 'partial' : 'ready_for_human_release_review',
      issues, registeredRuns: input.registeredRunIds.length, recordedRuns: input.runs.length,
      hardGates: { illegalWriteOrHashChangeCount: illegalWrites, hallucinatedPhotoReferenceCount: invalidRefs,
        auditedAgentRuns: agentRuns.filter(r => !!r.audit).length, agentAttempts: agentRuns.length,
        missingAudits: agentRuns.filter(r => !r.audit).length },
      completion: { completed, attempts: agentRuns.length, rate: agentRuns.length ? completed / agentRuns.length : null },
      completionByConfiguration: [...configurations.entries()].map(([key, runs]) => {
        const [version, model, promptVersion, phase] = JSON.parse(key) as (string | null)[];
        const count = runs.filter(r => completions.get(r.id)).length;
        return { version, model, promptVersion, phase, completed: count, attempts: runs.length, rate: count / runs.length };
      }),
      costs: { billedMinorUsd: sumKnown('billedCostMinorUsd'), estimatedMinorUsd: sumKnown('estimatedCostMinorUsd') },
      reviewSources: { human: humanReviews.length, simulatedAi: simulatedReviews.length, unverified: unverifiedReviews.length,
        conflicted: validReviews.filter(r => conflicted.has(r.reviewerId)).length },
      dimensions: summarizeDimensions(humanReviews), scoreByRun: scoreByRun(humanReviews),
      visualFactError: summarizeFacts(humanReviews),
      simulation: { label: 'AI 模拟演练，不是真人或独立摄影质量证据', isHumanQualityEvidence: false,
        reviews: simulatedReviews.length, dimensions: summarizeDimensions(simulatedReviews),
        scoreByRun: scoreByRun(simulatedReviews), visualFactError: summarizeFacts(simulatedReviews) },
      followUp: { recordedRuns: followedUp.length, totalRuns: agentRuns.length,
        humanModificationCount: followedUp.reduce((sum, f) => sum + f.humanModificationCount, 0),
        reviewedChanges, acceptedChanges, acceptanceRate: reviewedChanges ? acceptedChanges / reviewedChanges : null },
      limitations: ['Authorization, reviewer identity and evidence references are supplied attestations, not independently verified by this tool.',
        'No automatic aesthetic, statistical superiority or milestone release decision.',
        'Private key must stay hidden from reviewers; public goals must be checked for product-identifying text.'],
    },
  };
}
