import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID, randomBytes } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../src/server.ts';
import { migrate, digest } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
import { HostedAiService, HOSTED_AI_POLICY } from '../src/hosted-ai.ts';

let pool, api, base, calls = 0, seen, mode = 'ok', release;
const origin = 'http://127.0.0.1:1424';
const secret = 'isolated-async-transport-not-a-provider-key';
const endpoint = '/api/v2/ai/deepseek-flash-v1/responses/async';
const body = () => ({ model: 'deepseek-flash', input: [{ role: 'user', content: [{ type: 'input_text', text: 'fixture' }] }], max_output_tokens: 128, store: false });
const auth = u => ({ authorization: `Bearer ${u.token}`, origin });
const headers = (u, id) => ({ ...auth(u), 'content-type': 'application/json', 'x-focalroom-ai-policy': HOSTED_AI_POLICY, 'x-focalroom-request-id': id });
const reply = () => Response.json({ id: 'isolated-result', status: 'completed', output: [{ type: 'message', content: [{ type: 'output_text', text: 'fixture' }] }], usage: { input_tokens: 20, output_tokens: 10, total_tokens: 30 } });
async function transport(input, key) {
  assert.equal(key, secret); calls++; seen = input;
  if (mode === 'blocked') await new Promise(resolve => { release = resolve; });
  if (mode === 'network') throw new Error(secret);
  if (mode === 'reject') return new Response(secret, { status: 401 });
  if (mode === 'server') return new Response(secret, { status: 503 });
  if (mode === 'meter') return Response.json({ status: 'completed', output: [], usage: { total_tokens: 30 } });
  if (mode === 'overflow') return Response.json({ status: 'completed', output: [], usage: { input_tokens: 1, output_tokens: 999999, total_tokens: 1000000 } });
  return reply();
}
async function request(path, options) { const r = await fetch(base + path, options); return { status: r.status, body: await r.json() }; }
const submit = (u, input = body(), id = randomUUID(), extra = {}) => request(endpoint, { method: 'POST', headers: { ...headers(u, id), ...extra }, body: JSON.stringify(input) });
const poll = (u, id) => request(endpoint + '/' + id, { headers: auth(u) });
const quota = u => request('/api/v2/ai/account', { headers: auth(u) });
async function user(verified = true) {
  const u = await api.service.auth.account({ email: `${randomUUID()}@fixture.test`, password: 'fixture-password', display_name: 'Fixture' }, true, randomUUID());
  if (verified) await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1', [u.user.id]);
  return u;
}
async function finished(u, id) {
  for (let i = 0; i < 50; i++) {
    const result = await poll(u, id);
    assert.equal(result.status, 200);
    if (result.body.state !== 'PENDING') return result.body;
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  assert.fail('isolated transport failed to settle');
}
before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL }); await migrate(pool);
  api = await createApi({ pool, origin, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'async-previews')), hostedAi: { key: secret, initialTokens: 500000 }, hostedAiTransport: transport });
  await api.app.listen(0, '127.0.0.1'); base = await api.app.getUrl();
});
after(async () => { release?.(); await api?.close(); await pool?.end(); });

test('NestJS accepts async work without waiting; concurrent instances and retries call upstream once', async () => {
  const u = await user(), id = randomUUID(), start = calls;
  mode = 'blocked';
  try {
    const initial = await Promise.all([submit(u, body(), id), submit(u, body(), id)]);
    const submitted = initial[0]; assert.equal(initial[1].status, 202);
    assert.equal(submitted.status, 202); assert.equal(submitted.body.jobId, id); assert.equal(submitted.body.state, 'PENDING');
    assert.equal((await poll(u, id)).body.result, null);
    const peer = new HostedAiService(pool, api.service.auth, { key: secret, initialTokens: 500000 }, transport);
    const duplicates = await Promise.all([submit(u, body(), id), peer.submit(headers(u, id), body())]);
    assert.equal(duplicates[0].status, 202); assert.equal(duplicates[1].jobId, id); assert.equal(calls, start + 1);
    assert.equal((await submit(u, { ...body(), instructions: 'different' }, id)).status, 409);
    const reserved = (await quota(u)).body.reserved; assert.ok(reserved > 0);
    assert.equal((await pool.query('SELECT body_hash,policy,state,reserved FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2', [u.user.id, id])).rows[0].policy, HOSTED_AI_POLICY);
    release(); const result = await finished(u, id);
    assert.equal(result.state, 'COMPLETED'); assert.equal(result.result.usage.total_tokens, 30);
    assert.equal((await quota(u)).body.reserved, 0); assert.equal((await quota(u)).body.spent, 30);
    assert.deepEqual(await peer.job(auth(u), id), result, 'stored result survives service replacement');
    assert.equal((await submit(u, body(), id)).status, 200); assert.equal(calls, start + 1);
    assert.ok(!JSON.stringify(result).includes(secret));
  } finally { release?.(); mode = 'ok'; }
});

test('async validation preserves policy, verified account, CSRF, model, request ID and image consent', async () => {
  const u = await user(), unverified = await user(false), start = calls;
  assert.equal((await submit(unverified)).status, 403);
  assert.equal((await submit(u, body(), randomUUID(), { 'x-focalroom-ai-policy': 'other' })).status, 409);
  assert.equal((await submit(u, body(), 'bad-id')).status, 400);
  assert.equal((await submit(u, { ...body(), store: true })).status, 400);
  assert.equal((await submit(u, { ...body(), model: 'other' })).status, 400);
  assert.equal((await submit(u, body(), randomUUID(), { authorization: '', cookie: `fr_session=${u.token}` })).status, 403);
  const bytes = await sharp({ create: { width: 64, height: 32, channels: 3, background: '#369' } }).jpeg().withMetadata().toBuffer();
  const image = { ...body(), input: [{ role: 'user', content: [{ type: 'input_image', image_url: 'data:image/jpeg;base64,' + bytes.toString('base64') }] }] };
  assert.equal((await submit(u, image)).status, 403);
  assert.equal((await submit(u, { ...body(), tools: [{ type: 'web_search' }] })).status, 400);
  assert.equal(calls, start);
  const r = await submit(u, image, randomUUID(), { 'x-focalroom-image-consent': 'granted' });
  assert.ok([200, 202].includes(r.status)); assert.equal((await finished(u, r.body.jobId)).state, 'COMPLETED');
  assert.deepEqual(seen.reasoning, { effort: 'none' });
  assert.equal((await sharp(Buffer.from(seen.input[0].content[0].image_url.split(',')[1], 'base64')).metadata()).exif, undefined);
});

test('async quotas are atomic across concurrent jobs; account and session isolation applies to polling', async () => {
  const u = await user(), stranger = await user(), id = randomUUID(), start = calls;
  await quota(u); await pool.query('UPDATE fr_ai_accounts SET granted=10000 WHERE user_id=$1', [u.user.id]);
  mode = 'blocked';
  try {
    assert.equal((await submit(u, body(), id)).status, 202);
    assert.equal((await submit(u)).status, 402); assert.equal(calls, start + 1);
    assert.equal((await poll(stranger, id)).status, 404); assert.equal((await poll(u, 'bad-id')).status, 400);
    await pool.query('DELETE FROM fr_sessions WHERE digest=$1', [digest(u.token)]);
    assert.equal((await poll(u, id)).status, 401);
    release(); await api.service.ai.drain();
    assert.equal((await pool.query('SELECT spent,reserved FROM fr_ai_accounts WHERE user_id=$1', [u.user.id])).rows[0].spent, '30');
  } finally { release?.(); mode = 'ok'; }
});

test('only confirmed rejection releases async quota; network, 5xx and invalid usage remain UNKNOWN', async () => {
  for (const failure of ['reject', 'network', 'server', 'meter', 'overflow']) {
    const u = await user(), id = randomUUID(), start = calls; mode = failure;
    const r = await submit(u, body(), id); assert.ok([200, 202].includes(r.status));
    const result = await finished(u, id), q = (await quota(u)).body;
    assert.equal(result.state, failure === 'reject' ? 'REJECTED' : 'UNKNOWN');
    assert.equal(result.error, failure === 'reject' ? 'AI_PROVIDER_REJECTED' : 'AI_RESULT_UNKNOWN');
    assert.equal(q.spent, 0); assert.equal(q.reserved === 0, failure === 'reject');
    assert.equal(result.result, null); assert.ok(!JSON.stringify(result).includes(secret));
    await submit(u, body(), id); assert.equal(calls, start + 1);
  }
  mode = 'ok';
});

test('restarting with an abandoned PENDING job exposes UNKNOWN without replay or quota release', async () => {
  const u = await user(), id = randomUUID(), start = calls; await quota(u);
  await pool.query("INSERT INTO fr_ai_requests(user_id,request_id,body_hash,policy,reserved,state,created_at) VALUES($1,$2,'fixture',$3,100,'PENDING',now()-interval '7 minutes')", [u.user.id, id, HOSTED_AI_POLICY]);
  await pool.query('UPDATE fr_ai_accounts SET reserved=reserved+100 WHERE user_id=$1', [u.user.id]);
  assert.equal((await poll(u, id)).body.state, 'UNKNOWN');
  assert.equal((await quota(u)).body.reserved, 100); assert.equal(calls, start);
});

test('identical job IDs in two accounts settle only their own ledger and result', async () => {
  const a = await user(), b = await user(), id = randomUUID(), start = calls;
  const results = await Promise.all([submit(a, body(), id), submit(b, body(), id)]);
  assert.ok(results.every(result => [200, 202].includes(result.status)));
  await Promise.all([finished(a, id), finished(b, id)]);
  for (const u of [a, b]) {
    const q = (await quota(u)).body; assert.equal(q.spent, 30); assert.equal(q.reserved, 0);
  }
  assert.equal(calls, start + 2);
});

test('expired async results are removed without refunding or replaying a completed request', async () => {
  const u = await user(), id = randomUUID(), start = calls;
  await submit(u, body(), id); await finished(u, id);
  await pool.query("UPDATE fr_ai_requests SET finished_at=now()-interval '25 hours' WHERE user_id=$1 AND request_id=$2", [u.user.id, id]);
  const result = (await poll(u, id)).body;
  assert.equal(result.state, 'COMPLETED'); assert.equal(result.error, 'AI_RESULT_EXPIRED'); assert.equal(result.result, null);
  assert.equal((await pool.query('SELECT result_json FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2', [u.user.id, id])).rows[0].result_json, null);
  await submit(u, body(), id); assert.equal(calls, start + 1);
  assert.equal((await quota(u)).body.spent, 30); assert.equal((await quota(u)).body.reserved, 0);
});

test('async JSON payload above 2.5 MB traverses the real local HTTP parser and remains bounded', async () => {
  const u = await user();
  const bytes = await sharp(randomBytes(900 * 900 * 3), { raw: { width: 900, height: 900, channels: 3 } }).jpeg({ quality: 95 }).toBuffer();
  const image = { type: 'input_image', image_url: 'data:image/jpeg;base64,' + bytes.toString('base64') };
  const input = { ...body(), input: [{ role: 'user', content: [image, image, image] }] };
  assert.ok(Buffer.byteLength(JSON.stringify(input)) > 2500000);
  const r = await submit(u, input, randomUUID(), { 'x-focalroom-image-consent': 'granted' });
  assert.ok([200, 202].includes(r.status)); assert.equal((await finished(u, r.body.jobId)).state, 'COMPLETED');
  const invalid = { ...body(), instructions: 'x'.repeat(17 * 1024 * 1024) };
  assert.equal((await submit(u, invalid)).status, 413);
});
