import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { randomBytes, randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { assertSchema, digest, migrate } from '../src/database.ts';
import { checkRateLimit } from '../src/rate-limit.ts';
import { LocalPreviewStore } from '../src/storage.ts';
import { accountEmailUrl } from '../src/email.ts';

let pool, second, admin, api, base;
const deliveries = [];
let mailAvailable = true;
const password = 'initial-password-123';
const token = () => randomBytes(32).toString('hex');
const headers = account => ({ authorization: `Bearer ${account.token}` });
async function call(path, body, account, expected = 200) {
  const response = await fetch(base + '/api/v2' + path, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'content-type': 'application/json', ...(account ? headers(account) : {}) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const result = await response.json();
  assert.equal(response.status, expected, JSON.stringify(result));
  return result;
}
async function register() {
  const email = `account-${randomUUID()}@fr.test`;
  return call('/auth/register', { email, password, display_name: '隔离认证演练' });
}
async function insertToken(table, user, raw = token(), expired = false) {
  await pool.query(`INSERT INTO ${table}(digest,user_id,expires_at) VALUES($1,$2,now()+interval '${expired ? '-1 second' : '30 minutes'}')`, [digest(raw), user.user.id]);
  return raw;
}
before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DIRECTORY, '必须由隔离 PostgreSQL runner 启动');
  const url = new URL(process.env.FOCALROOM_TEST_DATABASE_URL);
  assert.equal(url.hostname, '127.0.0.1');
  admin = new Pool({ connectionString: url.href });
  await admin.query('CREATE DATABASE fr_account_security_fixture');
  url.pathname = '/fr_account_security_fixture';
  pool = new Pool({ connectionString: url.href });
  second = new Pool({ connectionString: url.href });
  await migrate(pool);
  const delivery = kind => async (email, raw, origin) => {
    if (mailAvailable) deliveries.push({ kind, email, raw, origin });
    return mailAvailable;
  };
  api = await createApi({ pool, origin: 'http://127.0.0.1:1424', store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'account-previews')), email: { reset: delivery('reset'), verification: delivery('verification') } });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
});
after(async () => { await api?.close(); await second?.end(); await pool?.end(); await admin?.end(); });

test('M4 explicit migrations create account/limiter tables and remain idempotent', async () => {
  await migrate(pool); await assertSchema(pool);
  assert.deepEqual((await pool.query('SELECT version FROM fr_migrations ORDER BY version')).rows.map(r => r.version), ['0001', '0002', '0006', '0008', '0009', '0010', '0011', '0012', '0013', '0014', '0015', '0017', '0018', '0019', '0020', '0021', '0022', '0023', '0024', '0025']);
  assert.deepEqual((await pool.query("SELECT column_name FROM information_schema.columns WHERE table_name='fr_ai_requests' AND column_name IN ('result_json','error_code') ORDER BY column_name")).rows.map(r => r.column_name), ['error_code', 'result_json']);
  for (const table of ['fr_source_assets', 'fr_asset_aliases', 'fr_edit_variants', 'fr_renditions', 'fr_boards', 'fr_canvas_nodes', 'fr_board_annotations', 'fr_author_hypotheses', 'fr_billing_refunds', 'fr_canvas_commands']) {
    assert.equal((await pool.query('SELECT to_regclass($1) AS name', [table])).rows[0].name, table);
  }
  const foreignKeys = (await pool.query("SELECT column_name,data_type FROM information_schema.columns WHERE table_name='fr_password_reset_tokens' AND column_name='user_id'")).rows;
  assert.equal(foreignKeys[0].data_type, 'text');
  await pool.query('SELECT * FROM fr_agent_task_results LIMIT 0');
});

test('M4 independent database clients enforce one shared hourly quota under concurrency', async () => {
  const key = `concurrent:${randomUUID()}`;
  const results = await Promise.all(Array.from({ length: 20 }, (_, i) => checkRateLimit(i % 2 ? pool : second, key, 3, 3_600_000)));
  assert.equal(results.filter(r => r.allowed).length, 3);
  const row = (await pool.query('SELECT count,window_start FROM fr_rate_limits WHERE key=$1', [`3600000:${key}`])).rows[0];
  assert.equal(row.count, 4, '拒绝计数饱和，不能溢出');
  assert.equal(row.window_start.getUTCMinutes(), 0);
  assert.equal(results[0].resetAt.getTime() - row.window_start.getTime(), 3_600_000);
  assert.equal((await checkRateLimit(pool, key, 1)).allowed, true, '不同窗口策略不混用');
  await assert.rejects(checkRateLimit(pool, key, 3, 0), /无效/);
});

test('M4 password reset hashes match login, revoke sessions and consume every outstanding token', async () => {
  const user = await register();
  const raw = await insertToken('fr_password_reset_tokens', user);
  const sibling = await insertToken('fr_password_reset_tokens', user);
  await call('/auth/reset-password', { token: raw, password: 'recovered-password-123' });
  await call('/auth/me', undefined, user, 401);
  await call('/auth/login', { email: user.user.email, password }, undefined, 401);
  await call('/auth/login', { email: user.user.email, password: 'recovered-password-123' });
  await call('/auth/reset-password', { token: raw, password }, undefined, 400);
  await call('/auth/reset-password', { token: sibling, password }, undefined, 400);
  assert.equal((await pool.query('SELECT count(*) FROM fr_password_reset_tokens WHERE user_id=$1 AND used_at IS NULL', [user.user.id])).rows[0].count, '0');
});

test('M4 concurrent resets with different tokens have exactly one winner', async () => {
  const user = await register();
  const tokens = await Promise.all([insertToken('fr_password_reset_tokens', user), insertToken('fr_password_reset_tokens', user)]);
  const replies = await Promise.all(tokens.map(raw => fetch(base + '/api/v2/auth/reset-password', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ token: raw, password: 'concurrent-reset-123' }) })));
  assert.deepEqual(replies.map(r => r.status).sort(), [200, 400]);
});

test('M4 invalid, expired and oversized reset inputs do not change the account', async () => {
  const user = await register(), raw = await insertToken('fr_password_reset_tokens', user);
  for (const value of ['', 'short', 'a'.repeat(1025), '海'.repeat(342)])
    await call('/auth/reset-password', { token: raw, password: value }, undefined, 400);
  await call('/auth/reset-password', { token: 'g'.repeat(64), password }, undefined, 400);
  await call('/auth/reset-password', { token: await insertToken('fr_password_reset_tokens', user, token(), true), password }, undefined, 400);
  await call('/auth/login', { email: user.user.email, password });
  await call('/auth/reset-password', { token: raw, password: 'valid-after-errors' });
});

test('M4 forgot password normalizes email and hides account existence and email quota', async () => {
  const user = await register(), before = deliveries.length;
  const unknown = `absent-${randomUUID()}@fr.test`;
  for (let i = 0; i < 5; i++) {
    assert.deepEqual(await call('/auth/forgot-password', { email: ` ${user.user.email.toUpperCase()} ` }), { sent: true });
    assert.deepEqual(await call('/auth/forgot-password', { email: unknown }), { sent: true });
  }
  assert.equal(deliveries.length - before, 3);
  const { raw } = deliveries.at(-1);
  assert.equal((await pool.query('SELECT count(*) FROM fr_password_reset_tokens WHERE digest=$1', [raw])).rows[0].count, '0');
  await call('/auth/reset-password', { token: raw, password: 'mail-reset-password-123' });
  await call('/auth/login', { email: user.user.email, password: 'mail-reset-password-123' });
});

test('M4 verification enforces session/CSRF and concurrent ten-minute delivery quota', async () => {
  const user = await register(), before = deliveries.length;
  await call('/auth/resend-verification', {}, undefined, 401);
  const csrf = await fetch(base + '/api/v2/auth/resend-verification', { method: 'POST', headers: { cookie: `fr_session=${user.token}`, origin: 'http://127.0.0.1:1424', 'content-type': 'application/json' }, body: '{}' });
  assert.equal(csrf.status, 403);
  const replies = await Promise.all(Array.from({ length: 5 }, () => fetch(base + '/api/v2/auth/resend-verification', { method: 'POST', headers: { ...headers(user), 'content-type': 'application/json' }, body: '{}' })));
  assert.deepEqual(replies.map(r => r.status).sort(), [200, 200, 200, 429, 429]);
  assert.equal(deliveries.length - before, 3);
  const raw = deliveries.at(-1).raw;
  await call('/auth/verify-email', { token: raw });
  await call('/auth/verify-email', { token: raw }, undefined, 400);
  await call('/auth/verify-email', { token: deliveries.at(-2).raw }, undefined, 400);
  assert.deepEqual(await call('/auth/resend-verification', {}, user), { already_verified: true });
});

test('M4 unavailable email rolls back tokens without claiming verification delivery', async () => {
  const user = await register();
  mailAvailable = false;
  try {
    assert.deepEqual(await call('/auth/forgot-password', { email: user.user.email }), { sent: true });
    const error = await call('/auth/resend-verification', {}, user, 503);
    assert.equal(error.error.code, 'EMAIL_UNAVAILABLE');
    for (const table of ['fr_password_reset_tokens', 'fr_email_verify_tokens'])
      assert.equal((await pool.query(`SELECT count(*) FROM ${table} WHERE user_id=$1`, [user.user.id])).rows[0].count, '0');
  } finally { mailAvailable = true; }
});
test('M4 account mail links use fragments, reject public HTTP and support the Web route', () => {
  const raw = token();
  const url = new URL(accountEmailUrl('https://review.example.test', 'reset-password', raw));
  assert.equal(url.pathname, '/shared'); assert.equal(url.search, '');
  assert.equal(url.hash, `#reset-password=${raw}`);
  assert.ok(accountEmailUrl('http://127.0.0.1:1430', 'verify-email', raw).includes('#verify-email='));
  assert.throws(() => accountEmailUrl('http://public.example.test', 'verify-email', raw));
  assert.throws(() => accountEmailUrl('https://user:secret@review.example.test', 'verify-email', raw));
});
test('M4 quota cleanup removes expired rows without erasing longer live windows', async t => {
  const key = `cleanup:${randomUUID()}`;
  await pool.query("INSERT INTO fr_rate_limits VALUES($1,now()-interval '3 days',1),($2,to_timestamp(floor(extract(epoch FROM now())/86400)*86400),3)", [`expired:${key}`, `86400000:${key}`]);
  t.mock.method(Math, 'random', () => 0);
  assert.equal((await checkRateLimit(pool, key, 3, 86_400_000)).allowed, false);
  assert.equal((await pool.query('SELECT count(*) FROM fr_rate_limits WHERE key=$1', [`expired:${key}`])).rows[0].count, '0');
  assert.equal((await checkRateLimit(second, key, 3, 86_400_000)).allowed, false);
});
