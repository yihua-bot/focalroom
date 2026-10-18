import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate, assertSchema, digest } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

let pool, admin, api, base;
let available = true;
const mail = [];
const origin = 'https://review.fixture.test';
const address = () => `otp-${randomUUID()}@fr.test`;
async function call(path, body, headers = {}, status = 200) {
  const res = await fetch(base + '/api/v2' + path, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'content-type': 'application/json', ...headers },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const data = await res.json();
  assert.equal(res.status, status, data.error?.code);
  return { data, cookie: res.headers.get('set-cookie') };
}
async function send(email = address()) {
  await call('/auth/magic/request', { email });
  const delivered = mail.at(-1);
  assert.equal(delivered.email, email.trim().toLowerCase());
  return delivered;
}
const login = (email, code, status = 200) => call('/auth/magic/verify', { email, code }, {}, status);
before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DIRECTORY);
  const url = new URL(process.env.FOCALROOM_TEST_DATABASE_URL);
  assert.equal(url.hostname, '127.0.0.1');
  admin = new Pool({ connectionString: url.href });
  await admin.query('CREATE DATABASE fr_magic_login_fixture');
  url.pathname = '/fr_magic_login_fixture';
  pool = new Pool({ connectionString: url.href });
  // 覆盖曾手工应用 0009 草案、尚无迁移登记的兼容升级。
  await pool.query(await readFile(new URL('../migrations/0009_magic_login.sql', import.meta.url), 'utf8'));
  await migrate(pool);
  api = await createApi({
    pool, origin, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'magic-previews')),
    magicEmail: async (email, code, target) => {
      if (available) mail.push({ email, code, target });
      return available;
    },
  });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
});
after(async () => { await api?.close(); await pool?.end(); await admin?.end(); });

test('M4 OTP migration supports legacy draft tables and repeat explicit migration', async () => {
  await migrate(pool); await assertSchema(pool);
  assert.equal((await pool.query("SELECT count(*) FROM fr_migrations WHERE version='0012'")).rows[0].count, '1');
  await pool.query('SELECT attempts FROM fr_magic_login_codes LIMIT 0');
  assert.equal((await pool.query("SELECT to_regclass('public.sessions') AS name")).rows[0].name, null);
});

test('M4 OTP normalizes email and stores a challenge-bound hash, never the code', async () => {
  const email = address();
  const delivered = await send('  ' + email.toUpperCase() + '  ');
  assert.match(delivered.code, /^\d{6}$/); assert.equal(delivered.target, origin);
  const row = (await pool.query('SELECT * FROM fr_magic_login_codes WHERE email=$1', [email])).rows[0];
  assert.equal(row.code_hash, digest(`magic-v1:${row.digest}:${delivered.code}`));
  assert.notEqual(row.code_hash, digest(delivered.code));
  assert.equal(row.attempts, 0);
});

test('M4 OTP creates an authenticated cookie session, verified account and owner workspace', async () => {
  const { email, code } = await send();
  const { data, cookie } = await login(email, code);
  assert.equal(data.user.email, email); assert.equal(data.is_new_user, true);
  assert.match(cookie, /HttpOnly/); assert.match(cookie, /SameSite=Strict/); assert.match(cookie, /; Secure/);
  assert.match(cookie, /Path=\/api\/v2/);
  assert.equal(Object.hasOwn(data, 'token'), false, '原始会话令牌只通过 HttpOnly Cookie 返回');
  const cookieHeader = cookie.split(';')[0];
  assert.notEqual(cookieHeader.slice(11), data.csrf);
  const current = await call('/auth/me', undefined, { cookie: cookieHeader });
  assert.equal(current.data.id, data.user.id);
  assert.equal(current.data.csrf, data.csrf);
  const workspaces = await call('/workspaces', undefined, { cookie: cookieHeader });
  assert.equal(workspaces.data.length, 1); assert.equal(workspaces.data[0].role, 'OWNER');
  assert.ok((await pool.query('SELECT email_verified_at FROM fr_users WHERE id=$1', [data.user.id])).rows[0].email_verified_at);
  await call('/auth/logout', {}, { cookie: cookieHeader, origin }, 403);
  await call('/auth/logout', {}, { cookie: cookieHeader, origin, 'x-focalroom-csrf': data.csrf });
  await call('/auth/me', undefined, { cookie: cookieHeader }, 401);
  await login(email, code, 400);
});

test('M4 OTP reuses existing accounts without changing password or duplicating workspaces', async () => {
  const email = address(), password = 'preserved-password-123';
  const registered = await call('/auth/register', { email, password, display_name: 'Existing editor' });
  const old = (await pool.query('SELECT password_hash FROM fr_users WHERE email=$1', [email])).rows[0].password_hash;
  const { code } = await send(email);
  const result = await login(email, code);
  assert.equal(result.data.user.id, registered.data.user.id);
  assert.equal(result.data.is_new_user, false);
  assert.equal((await pool.query('SELECT count(*) FROM fr_workspaces WHERE owner_id=$1', [result.data.user.id])).rows[0].count, '1');
  assert.equal((await pool.query('SELECT password_hash FROM fr_users WHERE email=$1', [email])).rows[0].password_hash, old);
  await call('/auth/login', { email, password });
});

test('M4 OTP mail failure rejects the request and rolls back codes', async () => {
  const email = address(), count = mail.length;
  available = false;
  try {
    const result = await call('/auth/magic/request', { email }, {}, 503);
    assert.equal(result.data.error.code, 'EMAIL_UNAVAILABLE');
    assert.equal(mail.length, count);
    assert.equal((await pool.query('SELECT count(*) FROM fr_magic_login_codes WHERE email=$1', [email])).rows[0].count, '0');
  } finally { available = true; }
  const { code } = await send(email);
  await login(email, code);
});

test('M4 OTP expired codes and invalid inputs create no session', async () => {
  const { email, code } = await send();
  await pool.query("UPDATE fr_magic_login_codes SET expires_at=now()-interval '1 second' WHERE email=$1", [email]);
  await login(email, code, 400);
  await login(email, '12345', 400);
  await login('invalid', '123456', 400);
  await call('/auth/magic/request', { email: 'invalid' }, {}, 400);
  await call('/auth/magic/request', { email: address() }, { origin: 'https://foreign.fixture.test' }, 403);
  assert.equal((await pool.query('SELECT count(*) FROM fr_users WHERE email=$1', [email])).rows[0].count, '0');
});

test('M4 OTP wrong attempts persist and revoke the code after five failures', async () => {
  const { email, code } = await send();
  const wrong = code === '000000' ? '000001' : '000000';
  for (let i = 0; i < 5; i++) await login(email, wrong, 400);
  const row = (await pool.query('SELECT attempts,used_at FROM fr_magic_login_codes WHERE email=$1', [email])).rows[0];
  assert.equal(row.attempts, 5); assert.ok(row.used_at);
  await login(email, code, 400);
});

test('M4 OTP resend invalidates old challenges, with one shared email quota', async () => {
  const { email, code: old } = await send();
  const { code } = await send(email);
  const rows = (await pool.query('SELECT used_at FROM fr_magic_login_codes WHERE email=$1', [email])).rows;
  assert.equal(rows.filter(r => !r.used_at).length, 1);
  if (old !== code) await login(email, old, 400);
  await login(email, code);
  await send(email);
  assert.equal((await call('/auth/magic/request', { email }, {}, 429)).data.error.code, 'RATE_LIMITED');
});

test('M4 OTP concurrent verification succeeds once and creates one session', async () => {
  const { email, code } = await send();
  const responses = await Promise.all(Array.from({ length: 8 }, () => fetch(base + '/api/v2/auth/magic/verify', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, code }),
  })));
  assert.deepEqual(responses.map(r => r.status).sort(), [200, 400, 400, 400, 400, 400, 400, 400]);
  for (const response of responses) await response.arrayBuffer();
  const row = (await pool.query('SELECT id FROM fr_users WHERE email=$1', [email])).rows[0];
  assert.equal((await pool.query('SELECT count(*) FROM fr_sessions WHERE user_id=$1', [row.id])).rows[0].count, '1');
});

test('M4 OTP session failure rolls back code consumption and new account atomically', async () => {
  const { email, code } = await send();
  await pool.query(`CREATE FUNCTION otp_session_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture session failure'; END $$;
    CREATE TRIGGER otp_session_failure BEFORE INSERT ON fr_sessions FOR EACH ROW EXECUTE FUNCTION otp_session_failure()`);
  try {
    await login(email, code, 500);
    assert.equal((await pool.query('SELECT count(*) FROM fr_users WHERE email=$1', [email])).rows[0].count, '0');
    assert.equal((await pool.query('SELECT used_at FROM fr_magic_login_codes WHERE email=$1', [email])).rows[0].used_at, null);
  } finally { await pool.query('DROP TRIGGER otp_session_failure ON fr_sessions; DROP FUNCTION otp_session_failure()'); }
  await login(email, code);
});

test('M4 OTP failed resend preserves the previously delivered code', async () => {
  const { email, code } = await send();
  available = false;
  try { await call('/auth/magic/request', { email }, {}, 503); }
  finally { available = true; }
  await login(email, code);
});

test('M4 OTP concurrent resend cannot bypass the shared delivery quota', async () => {
  const email = address(), before = mail.length;
  const responses = await Promise.all(Array.from({ length: 8 }, () => fetch(base + '/api/v2/auth/magic/request', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email }),
  })));
  assert.deepEqual(responses.map(r => r.status).sort(), [200, 200, 200, 429, 429, 429, 429, 429]);
  for (const response of responses) await response.arrayBuffer();
  assert.equal(mail.length - before, 3);
  assert.equal((await pool.query('SELECT count(*) FROM fr_magic_login_codes WHERE email=$1 AND used_at IS NULL', [email])).rows[0].count, '1');
  await login(email, mail.at(-1).code);
});
