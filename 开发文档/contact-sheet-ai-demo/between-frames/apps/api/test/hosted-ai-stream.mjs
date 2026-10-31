import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate, assertSchema } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
import { HOSTED_AI_POLICY } from '../src/hosted-ai.ts';

// 流式提交契约：鉴权/额度/幂等与 async 一致；增量经 SSE 转发；完成后回执仍可轮询。
let pool, admin, api, base;
const secret = 'isolated-stream-transport-not-a-provider-key';
const origin = 'https://stream.fixture.test';
const body = () => ({ model: 'deepseek-flash', input: [{ role: 'user', content: [{ type: 'input_text', text: '流式夹具' }] }], max_output_tokens: 128, store: false });
const usage = { input_tokens: 20, output_tokens: 12, total_tokens: 32 };
const output = [{ type: 'message', content: [{ type: 'output_text', text: '你好，流式' }] }];
let mode = 'ok';
const encode = value => new TextEncoder().encode(`data: ${JSON.stringify(value)}\n\n`);
async function transport(input, key) {
  assert.equal(key, secret);
  if (mode === 'reject') return new Response(secret, { status: 401 });
  if (mode === 'broken') return new Response('not-json', { status: 200 });
  const events = mode === 'badmeter'
    ? [{ type: 'response.output_text.delta', delta: '错' }, { type: 'response.completed', response: { status: 'completed', output, usage: { input_tokens: 1, output_tokens: 999999, total_tokens: 1000000 } } }]
    : [{ type: 'response.output_text.delta', delta: '你好，' }, { type: 'response.output_text.delta', delta: '流式' }, { type: 'response.completed', response: { id: 'stream-result', status: 'completed', output, usage } }];
  return new Response(new ReadableStream({ start(controller) { for (const event of events) controller.enqueue(encode(event)); controller.close(); } }), { status: 200 });
}
async function register() {
  const email = `stream-${randomUUID()}@fr.test`;
  const r = await fetch(base + '/api/v2/auth/register', { method: 'POST', headers: { 'content-type': 'application/json', origin }, body: JSON.stringify({ email, password: 'stream-password-1', display_name: '流式' }) });
  const value = await r.json();
  await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1', [value.user.id]);
  return value;
}
async function streamPost(user, requestId, payload = body()) {
  return fetch(base + '/api/v2/ai/deepseek-flash-v1/responses/stream', {
    method: 'POST',
    headers: { 'content-type': 'application/json', origin, authorization: `Bearer ${user.token}`, 'x-focalroom-ai-policy': HOSTED_AI_POLICY, 'x-focalroom-request-id': requestId },
    body: JSON.stringify(payload),
  });
}
async function readSSE(response) {
  assert.equal(response.status, 200);
  assert.match(response.headers.get('content-type') ?? '', /text\/event-stream/);
  const text = await response.text();
  return text.split('\n\n').filter(line => line.startsWith('data:')).map(line => JSON.parse(line.slice(5).trim()));
}
async function account(user) {
  const row = (await pool.query('SELECT granted,spent,reserved FROM fr_ai_accounts WHERE user_id=$1', [user.user.id])).rows[0];
  return { spent: Number(row.spent), reserved: Number(row.reserved) };
}

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DIRECTORY);
  const url = new URL(process.env.FOCALROOM_TEST_DATABASE_URL);
  assert.equal(url.hostname, '127.0.0.1');
  admin = new Pool({ connectionString: url.href });
  await admin.query('CREATE DATABASE fr_hosted_ai_stream_fixture');
  url.pathname = '/fr_hosted_ai_stream_fixture';
  pool = new Pool({ connectionString: url.href });
  await migrate(pool);
  api = await createApi({ pool, origin, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'stream-previews')), hostedAi: { key: secret, initialTokens: 50000 }, hostedAiTransport: transport });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
});
after(async () => { await api?.close(); await pool?.end(); await admin?.end(); });

test('stream forwards deltas, settles metering and leaves a pollable receipt', async () => {
  await migrate(pool); await assertSchema(pool);
  const user = await register();
  const id = randomUUID();
  const events = await readSSE(await streamPost(user, id));
  assert.deepEqual(events.filter(e => e.type === 'delta').map(e => e.text), ['你好，', '流式']);
  const done = events.find(e => e.type === 'done');
  assert.equal(done.reply.usage.total_tokens, 32);
  assert.equal(events[0].type, 'start');
  const after = await account(user);
  assert.equal(after.spent, 32, '流式完成后按真实用量结算');
  assert.equal(after.reserved, 0);
  const receipt = await (await fetch(base + `/api/v2/ai/deepseek-flash-v1/responses/async/${id}`, { headers: { authorization: `Bearer ${user.token}` } })).json();
  assert.equal(receipt.state, 'COMPLETED', '流式结果仍可凭编号轮询（恢复路径兼容）');
  // 同号流式重放：内容已产生，返回冲突而不是再次调用模型。
  const replay = await streamPost(user, id);
  assert.equal(replay.status, 409);
});

test('stream rejects replayed ids, provider rejections and unverified metering', async () => {
  const user = await register();
  mode = 'reject';
  const rejected = await streamPost(user, randomUUID());
  assert.equal(rejected.status, 502);
  const afterReject = await account(user);
  assert.equal(afterReject.spent, 0, '供应商拒绝应释放额度不计费');
  mode = 'badmeter';
  const events = await readSSE(await streamPost(user, randomUUID()));
  assert.equal(events.at(-1).type, 'error');
  assert.equal(events.at(-1).code, 'AI_RESULT_UNKNOWN');
  mode = 'broken';
  const broken = await readSSE(await streamPost(user, randomUUID()));
  assert.equal(broken.at(-1).type, 'error');
  mode = 'ok';
  const quota = await fetch(base + '/api/v2/ai/account', { headers: { authorization: `Bearer ${user.token}` } });
  assert.equal((await quota.json()).available > 0, true);
});

// 本地化冒烟：同一错误按 Accept-Language 返回对应语言文案。
test('error messages localize by Accept-Language', async () => {
  for (const [lang, expected] of [['en', 'Please sign in first'], ['ja', '先にログインしてください'], ['ko', '먼저 로그인하세요']]) {
    const r = await fetch(base + '/api/v2/auth/me', { headers: { 'accept-language': lang } });
    const body = await r.json();
    assert.equal(r.status, 401);
    assert.equal(body.error.message, expected, `${lang} 应返回本地化文案`);
  }
  const zh = await (await fetch(base + '/api/v2/auth/me', { headers: { 'accept-language': 'zh-Hans' } })).json();
  assert.equal(zh.error.message, '请先登录');
  const fallback = await (await fetch(base + '/api/v2/auth/me')).json();
  assert.equal(fallback.error.message, '请先登录', '无语言头回落中文');
});
