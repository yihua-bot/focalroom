// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createBlindPack, evaluate, genres, phases, snapshotDigest, type EvaluationInput } from './evaluation.ts';

const sha = (value: string) => createHash('sha256').update(value).digest('hex');
function corpus(): EvaluationInput {
  const tasks = Array.from({ length: 30 }, (_, index) => ({
    id: `task-${index}`, creatorId: 'photographer', goal: `editing goal ${index}`,
    genre: genres[index % genres.length], phase: phases[index % phases.length],
    targetCount: 2, provenance: 'photographer' as const,
    photos: Array.from({ length: 80 }, (_, n) => ({ id: `photo-${index}-${n}`, sha256: sha(`${index}:${n}`) })),
    authorization: { holderId: 'photographer', evidenceRef: 'local consent record', photoIds: Array.from({ length: 80 }, (_, n) => `photo-${index}-${n}`), cloudPreviewProviders: ['deepseek'] },
    feedbackEvidenceRef: 'human command evidence',
  }));
  const runs = tasks.flatMap(task => (['between_frames', 'human_baseline', 'competitor'] as const).map(system => ({
    id: `${task.id}-${system}`, taskId: task.id, system, version: 'test-version',
    model: 'test-model', promptVersion: 'test-prompt', state: 'review_ready' as const,
    orderedPhotoIds: task.photos.slice(0, 2).map(p => p.id), withinBudget: true,
    elapsedMs: 100, actualTokens: 20, estimatedCostMinorUsd: 1, billedCostMinorUsd: null,
    sourceEvidenceRef: 'test-only trace', explanation: 'Candidate sequence rationale', followUp: null,
    trace: [{ callId: 'call-1', tool: 'list_photos', photoIds: task.photos.slice(0, 2).map(p => p.id), evidenceRef: 'test-only call receipt' }],
    audit: { evidenceRef: 'test-only state audit', formalBeforeSha256: sha('formal'), formalAfterSha256: sha('formal'), originalsBeforeSha256: sha('originals'), originalsAfterSha256: sha('originals'), illegalWriteCount: 0 },
    cloudPreviewProvider: 'deepseek',
  })));
  const input: EvaluationInput = { protocolVersion: 'bf-m3-eval-1', datasetId: 'unit-fixture-not-real-evidence', registeredRunIds: runs.map(r => r.id), tasks, runs, reviews: [] };
  input.reviews = createBlindPack(input, 'fixed-test-seed').privateKey.flatMap(k => ['photographer', 'independent'].map(reviewerId => ({
    caseId: k.caseId, candidateId: k.candidateId, reviewerId,
    scores: { visual_facts: 4, creative_intent: 3, sequence_rhythm: 2, explanation: 5, collaboration: 4 },
    reason: 'Test-only verifiable observation; not a real review.', factCheck: { checkedClaims: 2, incorrectClaims: 1 },
  })));
  return input;
}
const codes = (input: EvaluationInput) => evaluate(input, 'fixed-test-seed').report.issues.map(i => i.code);

describe('M3 evidence evaluation', () => {
  it('reports absent evidence as partial with null rates, not 0/0 success', () => {
    const input = corpus(); input.tasks = []; input.runs = []; input.registeredRunIds = []; input.reviews = [];
    const result = evaluate(input, 'fixed-test-seed');
    expect(result.report.status).toBe('partial');
    expect(result.report.completion.rate).toBeNull();
    expect(result.report.dimensions.visual_facts.mean).toBeNull();
    expect(result.report.issues.some(i => i.code === 'NEED_30_REAL_TASKS')).toBe(true);
  });
  it('keeps five dimensions, denominators and estimated costs separate from bills', () => {
    const report = evaluate(corpus(), 'fixed-test-seed').report;
    expect(report.status).toBe('ready_for_human_release_review');
    expect(report.completion).toEqual({ completed: 30, attempts: 30, rate: 1 });
    expect(report.dimensions.sequence_rhythm).toEqual({ mean: 2, count: 180 });
    expect(report.costs.billedMinorUsd).toEqual({ total: 0, knownRuns: 0, totalRuns: 30 });
    expect(report.costs.estimatedMinorUsd.total).toBe(30);
    expect(report.scoreByRun).toHaveLength(90);
    expect(report.completionByConfiguration).toHaveLength(2);
    expect(report.visualFactError).toEqual({ checkedClaims: 360, incorrectClaims: 180, rate: 0.5 });
    expect(report.followUp.acceptanceRate).toBeNull();
  });
  it('does not allow IDs or repeated files to manufacture 30 tasks', () => {
    const input = corpus(); input.tasks[1] = { ...input.tasks[0], id: input.tasks[1].id };
    input.tasks[2].photos[1].sha256 = input.tasks[2].photos[0].sha256;
    input.tasks[3].provenance = 'synthetic'; input.tasks[4].photos = input.tasks[4].photos.slice(0, 79);
    expect(codes(input)).toEqual(expect.arrayContaining(['DUPLICATE_TASK_CONTENT', 'DUPLICATE_SOURCE_PHOTO', 'SYNTHETIC_NOT_QUALITY_EVIDENCE', 'INVALID_TASK_SIZE']));
  });
  it('rejects missing authorization, extra scope and unconsented providers', () => {
    const input = corpus(); input.tasks[0].authorization = null;
    input.tasks[1].authorization!.photoIds.push('foreign'); input.runs[6].cloudPreviewProvider = 'other';
    expect(codes(input)).toEqual(expect.arrayContaining(['AUTHORIZATION_SCOPE_MISSING_OR_MISMATCH', 'CLOUD_PREVIEW_NOT_AUTHORIZED']));
    expect(evaluate(input, 'fixed-test-seed').report.completion.completed).toBe(27);
    const padded = corpus(); padded.runs[0].orderedPhotoIds[0] += ' ';
    expect(() => evaluate(padded, 'fixed-test-seed')).toThrow();
  });
  it('retains failed attempts and rejects a dropped registered attempt', () => {
    const input = corpus(); input.runs[0].state = 'paused';
    expect(evaluate(input, 'fixed-test-seed').report.completion).toEqual({ completed: 29, attempts: 30, rate: 29 / 30 });
    input.runs.shift();
    expect(codes(input)).toContain('REGISTERED_RUNS_MISSING_OR_EXTRA');
  });
  it('does not average away illegal writes, fabricated refs, missing audits or budget failures', () => {
    const input = corpus();
    input.runs[0].audit!.formalAfterSha256 = sha('changed');
    input.runs[3].trace[0].photoIds.push('foreign'); input.runs[6].audit = null;
    input.runs[9].withinBudget = false;
    const report = evaluate(input, 'fixed-test-seed').report;
    expect(report.status).toBe('partial'); expect(report.completion.completed).toBe(26);
    expect(report.hardGates).toEqual({ illegalWriteOrHashChangeCount: 1, hallucinatedPhotoReferenceCount: 1, auditedAgentRuns: 29, agentAttempts: 30, missingAudits: 1 });
  });
  it('requires human author plus independent reviews and rejects duplicate or unknown reviews', () => {
    const input = corpus(); input.reviews = input.reviews.filter(r => r.reviewerId === 'photographer');
    input.reviews.push(input.reviews[0], { ...input.reviews[0], candidateId: 'unknown' });
    expect(codes(input)).toEqual(expect.arrayContaining(['NEED_AUTHOR_AND_INDEPENDENT_REVIEW', 'DUPLICATE_REVIEW', 'UNKNOWN_REVIEW_CANDIDATE']));
    input.reviews[0].reason = ' ';
    expect(() => evaluate(input, 'fixed-test-seed')).toThrow();
  });
  it('hides product/model identities and uses reproducible blinded mappings', () => {
    const input = corpus(); const pack = createBlindPack(input, 'fixed-test-seed');
    expect(pack).toEqual(createBlindPack(input, 'fixed-test-seed'));
    expect(pack.privateKey).not.toEqual(createBlindPack(input, 'different-seed').privateKey);
    const publicText = JSON.stringify(pack.publicPack);
    for (const secret of ['between_frames', 'human_baseline', 'competitor', 'test-model', 'photographer', 'test-version']) expect(publicText).not.toContain(secret);
    const digest = snapshotDigest(input); input.reviews = [];
    expect(snapshotDigest(input)).toBe(digest);
    input.runs[0].state = 'failed'; expect(snapshotDigest(input)).not.toBe(digest);
  });
  it('CLI persists partial reports, never overwrites outputs, and freezes review inputs', () => {
    const directory = mkdtempSync(join(tmpdir(), 'bf-eval-test-'));
    const source = join(directory, 'input.json'); const output = join(directory, 'results');
    const input = corpus(); input.reviews = [];
    writeFileSync(source, JSON.stringify(input));
    const cli = resolve('tools/agent-eval/cli.ts');
    const run = (...args: string[]) => spawnSync(process.execPath, [cli, ...args], { encoding: 'utf8' });
    expect(run(source, output).status).toBe(2);
    expect(run(source, output).status).toBe(1);
    const key = join(output, 'private-key.json');
    const saved = JSON.parse(readFileSync(key, 'utf8')) as { seed: string };
    const pack = createBlindPack(input, saved.seed);
    input.reviews = pack.privateKey.flatMap(k => ['photographer', 'independent'].map(reviewerId => ({ caseId: k.caseId, candidateId: k.candidateId, reviewerId, scores: { visual_facts: 4, creative_intent: 3, sequence_rhythm: 2, explanation: 5, collaboration: 4 }, reason: 'test observation', factCheck: { checkedClaims: 0, incorrectClaims: 0 } })));
    writeFileSync(source, JSON.stringify(input));
    expect(run(source, join(directory, 'without-key')).status).toBe(1);
    execFileSync(process.execPath, [cli, source, join(directory, 'reviewed'), key]);
    input.runs.pop(); writeFileSync(source, JSON.stringify(input));
    expect(run(source, join(directory, 'changed'), key).status).toBe(1);
    expect(JSON.parse(readFileSync(join(output, 'report.json'), 'utf8')).status).toBe('partial');
  });
});
