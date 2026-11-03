// 管理员统计接口的权限边界：只有 fr_users.role='admin' 能读到注册量与下载量，
// 匿名与普通账户一律拒绝，且响应里不得出现任何统计字段。
import test from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

assert.ok(process.env.FOCALROOM_TEST_DIRECTORY);
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');

test('产品统计只对管理员开放', async () => {
  const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  let api;
  try {
    await migrate(pool);
    api = await createApi({
      pool,
      store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'admin-stats')),
      origin: 'http://127.0.0.1:1424',
    });
    await api.app.listen(0, '127.0.0.1');
    const base = await api.app.getUrl();
    const request = async (path, token, body) => {
      const response = await fetch(base + '/api/v2' + path, {
        method: body === undefined ? 'GET' : 'POST',
        headers: { 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}) },
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      });
      return { status: response.status, body: await response.json() };
    };
    const password = 'admin-stats-fixture-2026';
    const member = await request('/auth/register', null, { email: 'stats-member@fr.test', password, display_name: '普通用户' });
    assert.equal(member.status, 200, JSON.stringify(member.body));
    const token = member.body.token;

    const anonymous = await request('/stats/overview');
    assert.equal(anonymous.status, 401);
    assert.equal(anonymous.body.registrations, undefined);

    const plain = await request('/stats/overview', token);
    assert.equal(plain.status, 403, JSON.stringify(plain.body));
    assert.equal(plain.body.registrations, undefined);
    assert.equal(plain.body.downloads, undefined);
    assert.equal(plain.body.downloadsTotal, undefined);

    await pool.query("UPDATE fr_users SET role='admin' WHERE email=$1", ['stats-member@fr.test']);
    const admin = await request('/stats/overview', token);
    assert.equal(admin.status, 200, JSON.stringify(admin.body));
    assert.ok(admin.body.registrations.total >= 1);
    assert.equal(typeof admin.body.downloadsTotal, 'number');
  } finally {
    if (api) await api.close();
    await pool.end();
  }
});
