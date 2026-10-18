// M4 云 Agent Worker 回归：独立模型发送授权、可恢复队列、取消/撤权/额度、
// 模型调用走本地 mock 服务器（无真实外发）。
import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { join } from 'node:path';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
import { runWorkerOnce } from '../../../workers/agent/src/index.ts';

let api, pool, base, owner, reviewer, viewer, workspace, project, manifest;
let modelServer, modelCalls = [];
const json = (method, token, body) => ({ method, headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), 'content-type': 'application/json' }, ...(body !== undefined ? { body: JSON.stringify(body) } : {}) });
async function call(path, method = 'GET', token = owner?.token, body, status = 200) {
  const response = await fetch(`${base}/api/v2${path}`, json(method, token, body)); const data = await response.json(); assert.equal(response.status, status, JSON.stringify(data)); return data;
}

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须由隔离 PostgreSQL runner 启动');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  // 本地 mock 模型：记录请求并返回固定汇总；不产生真实网络外发。
  modelServer = createServer((request, response) => {
    let raw = '';
    request.on('data', (chunk) => { raw += chunk; });
    request.on('end', () => {
      modelCalls.push({ url: request.url, auth: request.headers.authorization, body: raw });
      response.setHeader('content-type', 'application/json');
      response.end(JSON.stringify({
        model: 'mock-chat-1',
        choices: [{ message: { content: '整体倾向积极；主要关注开场节奏与人物视线；建议保留两版序列对比后定稿。' } }],
        usage: { prompt_tokens: 120, completion_tokens: 30 },
      }));
    });
  });
  await new Promise((resolve) => modelServer.listen(0, '127.0.0.1', resolve));
  const modelPort = modelServer.address().port;
  process.env.FOCALROOM_AGENT_MODEL_KEY = 'test-mock-key';
  process.env.FOCALROOM_AGENT_MODEL_BASE_URL = `http://127.0.0.1:${modelPort}/v1`;
  process.env.FOCALROOM_AGENT_MODEL_NAME = 'mock-chat';
  api = await createApi({ pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'previews-worker')), origin: 'http://127.0.0.1:1424' });
  await api.app.listen(0, '127.0.0.1'); base = await api.app.getUrl();
  const register = (email) => call('/auth/register', 'POST', null, { email, password: 'test-password-123', display_name: email });
  owner = await register('worker-owner@fr.test'); reviewer = await register('worker-reviewer@fr.test'); viewer = await register('worker-viewer@fr.test');
  workspace = (await call('/workspaces'))[0].id;
  const { randomUUID } = await import('node:crypto');
  manifest = { workspace_id: workspace, local_project_id: 'worker-demo', promotion_id: randomUUID(), title: 'Worker 评片', from_revision: 0,
    snapshot: { photos: [{ id: 'wp1', filename: 'wp1.jpg', source_sha256: 'a'.repeat(64), selection: 'UNDECIDED' }], sequences: [{ id: 'seq', title: '主序列', items: [] }] },
    media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
  const promotion = await call('/promotions', 'POST', owner.token, manifest);
  project = promotion.project_id;
  // 上传预览并 finalize：项目转 SHARED 后才能发送项目邀请。
  const jpeg = await sharp({ create: { width: 96, height: 64, channels: 3, background: '#3178c6' } }).jpeg().toBuffer();
  for (const photo of manifest.snapshot.photos) {
    const response = await fetch(`${base}/api/v2/projects/${project}/previews/${photo.id}`, { method: 'PUT', headers: { authorization: `Bearer ${owner.token}`, 'content-type': 'image/jpeg' }, body: jpeg });
    assert.equal(response.status, 200, await response.text());
  }
  await call(`/projects/${project}/finalize`, 'POST', owner.token, { manifest_digest: promotion.manifest_digest });
});

after(async () => { if (api) await api.close(); if (pool) await pool.end(); if (modelServer) await new Promise((resolve) => modelServer.close(resolve)); });

async function inviteOnce(account, role) {
  const link = await call(`/workspaces/${workspace}/invitations`, 'POST', owner.token, { email: account.user.email, role, project_id: project });
  await call('/invitations/accept', 'POST', account.token, { token: new URL(link.invite_url).hash.split('=')[1] });
}

test('M4 cookie-authenticated Agent GETs need no Origin/CSRF, writes remain protected', async () => {
  for (const suffix of ['model-consents', 'agent-jobs', 'agent-tasks']) {
    const response = await fetch(base + '/api/v2/projects/' + project + '/' + suffix, {
      headers: { cookie: 'fr_session=' + owner.token },
    });
    assert.equal(response.status, 200, suffix);
    await response.arrayBuffer();
    const anonymous = await fetch(base + '/api/v2/projects/' + project + '/' + suffix);
    assert.equal(anonymous.status, 401, suffix);
    await anonymous.arrayBuffer();
  }
  const response = await fetch(base + '/api/v2/projects/' + project + '/model-consents', {
    method: 'POST', headers: { cookie: 'fr_session=' + owner.token, 'content-type': 'application/json' },
    body: JSON.stringify({ scope: 'summarize_comments', max_tasks: 1 }),
  });
  assert.equal(response.status, 403);
  assert.equal((await response.json()).error.code, 'CSRF_REJECTED');
});

test('M4 无显式授权时入队被拒；授权/任务/取消/撤权全链路', async () => {
  await inviteOnce(reviewer, 'REVIEWER');
  await inviteOnce(viewer, 'VIEWER');
  // 先放两条人工评论供汇总。
  await call(`/projects/${project}/commands`, 'POST', reviewer.token, {
    version: '2.0', command_id: crypto.randomUUID(), workspace_id: workspace, project_id: project, expected_revision: 0, idempotency_key: crypto.randomUUID(),
    kind: 'comment.add', payload: { body: '开场节奏偏慢，建议第二张提前。', anchor: { type: 'photo', photo_id: 'wp1', x: 0.3, y: 0.4 } },
  });
  await call(`/projects/${project}/commands`, 'POST', owner.token, {
    version: '2.0', command_id: crypto.randomUUID(), workspace_id: workspace, project_id: project, expected_revision: 0, idempotency_key: crypto.randomUUID(),
    kind: 'comment.add', payload: { body: '开场保持克制更好，提前会破坏铺垫。', anchor: { type: 'photo', photo_id: 'wp1', x: 0.6, y: 0.2 } },
  });
  // 越权：REVIEWER 不能授权或入队。
  await call(`/projects/${project}/model-consents`, 'POST', reviewer.token, { scope: 'summarize_comments', max_tasks: 5 }, 403);
  // 无有效授权：入队返回 MODEL_CONSENT_REQUIRED。
  await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: 'missing' }, 403);
  // 授权：范围 + 额度；回显端点摘要。
  const consent = await call(`/projects/${project}/model-consents`, 'POST', owner.token, { scope: 'summarize_comments', max_tasks: 2 });
  assert.match(consent.endpoint_label, /mock-chat@127\.0\.0\.1/);
  // 入队（不入队 REVIEWER）。
  const job = await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: consent.consent_id });
  assert.equal(job.status, 'QUEUED');
  // Worker 执行：完成、结果落库、额度扣减。
  const run = await runWorkerOnce(pool);
  assert.equal(run.status, 'COMPLETED');
  assert.equal(modelCalls.length, 1);
  assert.equal(modelCalls[0].auth, 'Bearer test-mock-key');
  const consents = await call(`/projects/${project}/model-consents`, 'GET');
  assert.equal(consents.consents[0].used, 1);
  assert.equal(consents.consents[0].state, 'ACTIVE');
  const jobs = await call(`/projects/${project}/agent-jobs`, 'GET', owner.token);
  assert.equal(jobs.jobs[0].status, 'COMPLETED');
  const tasks = await call(`/projects/${project}/agent-tasks`, 'GET', owner.token);
  assert.equal(tasks.tasks[0].comment_count, 2);
  assert.ok(tasks.tasks[0].tokens_in > 0);
  // 取消：QUEUED 任务取消后 Worker 不执行。
  const second = await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: consent.consent_id });
  await call(`/projects/${project}/agent-jobs/${second.job_id}/cancel`, 'POST', owner.token);
  const idle = await runWorkerOnce(pool);
  assert.equal(idle.claimed, false, '取消的任务不应被领取');
  assert.equal(modelCalls.length, 1);
  // 额度：第二个授权 max_tasks=1，完成一次后用尽，再入队被拒。
  const small = await call(`/projects/${project}/model-consents`, 'POST', owner.token, { scope: 'summarize_comments', max_tasks: 1 });
  await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: small.consent_id });
  assert.equal((await runWorkerOnce(pool)).status, 'COMPLETED');
  await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: small.consent_id }, 403);
  // 撤权：未执行任务随撤回取消。
  const third = await call(`/projects/${project}/model-consents`, 'POST', owner.token, { scope: 'summarize_disagreement', max_tasks: 5 });
  const pending = await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_disagreement', consent_id: third.consent_id });
  const revoked = await call(`/projects/${project}/model-consents/${third.consent_id}`, 'DELETE', owner.token);
  assert.equal(revoked.cancelled_jobs, 1);
  const afterRevoke = await runWorkerOnce(pool);
  assert.equal(afterRevoke.claimed, false, '撤回的 QUEUED 任务已被取消');
  assert.equal(pending.status, 'QUEUED');
  assert.equal(modelCalls.length, 2, '分歧分析未执行，模型仍只被调过 2 次');
});

test('M4 崩溃恢复：租约过期的 RUNNING 任务被重新领取并完成', async () => {
  const consent = await call(`/projects/${project}/model-consents`, 'POST', owner.token, { scope: 'summarize_comments', max_tasks: 5 });
  const job = await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: consent.consent_id });
  // 模拟 Worker 崩溃：任务已执行过一次（attempts=1）后滞留 RUNNING 且租约已过期。
  await pool.query(`UPDATE fr_agent_jobs SET status='RUNNING', attempts=1, lease_expires=now()-interval '5 seconds' WHERE id=$1`, [job.job_id]);
  const run = await runWorkerOnce(pool);
  assert.equal(run.status, 'COMPLETED', '过期租约任务应被重新领取执行');
  const jobs = await call(`/projects/${project}/agent-jobs`, 'GET', owner.token);
  const recovered = jobs.jobs.find((item) => item.id === job.job_id);
  assert.equal(recovered.status, 'COMPLETED');
  assert.ok(recovered.attempts >= 2, '崩溃重试计入 attempts');
});

test('M4 模型失败有限重试后 FAILED 且不外发更多请求', async () => {
  const consent = await call(`/projects/${project}/model-consents`, 'POST', owner.token, { scope: 'summarize_comments', max_tasks: 10 });
  const job = await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: consent.consent_id });
  const before = modelCalls.length;
  // 让 mock 返回 500（仍计入调用次数）。
  const original = modelServer.listeners('request')[0];
  modelServer.removeAllListeners('request');
  modelServer.on('request', (request, response) => { modelCalls.push({ url: request.url, failed: true }); response.statusCode = 500; response.end('boom'); });
  const first = await runWorkerOnce(pool);
  assert.equal(first.status, 'REQUEUED', '首次失败重新排队');
  const second = await runWorkerOnce(pool);
  assert.equal(second.status, 'REQUEUED');
  const third = await runWorkerOnce(pool);
  assert.equal(third.status, 'FAILED', '第三次失败后终止');
  const idle = await runWorkerOnce(pool);
  assert.equal(idle.claimed, false);
  const jobs = await call(`/projects/${project}/agent-jobs`, 'GET', owner.token);
  const failed = jobs.jobs.find((item) => item.id === job.job_id);
  assert.equal(failed.status, 'FAILED');
  assert.match(failed.error, /500/);
  // 恢复 mock 供后续用例。
  modelServer.removeAllListeners('request');
  modelServer.on('request', original);
  assert.equal(modelCalls.length - before, 3, '恰好重试 3 次模型调用');
});
