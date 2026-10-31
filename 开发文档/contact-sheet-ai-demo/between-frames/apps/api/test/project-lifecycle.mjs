// 项目归档与删除生命周期：归档隐藏、恢复、所有者权限、级联删除、STAGING 保护。
import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

let pool, api, base, owner, other, workspaceId, JPEG;

async function call(path, method = 'GET', account = owner, body, status = 200) {
  const response = await fetch(base + '/api/v2' + path, {
    method,
    headers: { 'content-type': 'application/json', ...(account ? { authorization: 'Bearer ' + account.token } : {}) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const value = await response.json();
  assert.equal(response.status, status, JSON.stringify(value));
  return value;
}

const JPEG_BUFFER_PROMISE = sharp({ create: { width: 8, height: 8, channels: 3, background: '#596b51' } }).jpeg().toBuffer();

async function createProject(title, finalize = true) {
  const manifest = {
    workspace_id: workspaceId,
    local_project_id: 'local-' + randomUUID(),
    promotion_id: randomUUID(),
    title,
    from_revision: 0,
    snapshot: {
      photos: [{ id: 'p1', filename: 'f1.jpg', source_sha256: 'a'.repeat(64), selection: 'KEEP' }],
      sequences: [{ id: 'seq', title: '主序列', items: [{ id: 'i1', photo_id: 'p1' }] }],
    },
    media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false },
  };
  const promotion = await call('/promotions', 'POST', owner, manifest);
  if (!finalize) return promotion.project_id;
  const put = await fetch(base + '/api/v2/projects/' + promotion.project_id + '/previews/p1', {
    method: 'PUT', headers: { authorization: 'Bearer ' + owner.token, 'content-type': 'image/jpeg' }, body: JPEG,
  });
  assert.equal(put.status, 200);
  await call('/projects/' + promotion.project_id + '/finalize', 'POST', owner, { manifest_digest: promotion.manifest_digest });
  return promotion.project_id;
}

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须使用隔离 PostgreSQL runner');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  api = await createApi({ pool, origin: 'http://127.0.0.1:1424',
    store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'lifecycle-previews')) });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
  const register = email => call('/auth/register', 'POST', null, { email, password: 'lifecycle-test-password-123', display_name: email });
  owner = await register('lifecycle-owner@fr.test');
  other = await register('lifecycle-other@fr.test');
  const workspaces = await call('/workspaces');
  workspaceId = workspaces[0].id;
  JPEG = await JPEG_BUFFER_PROMISE;
});

after(async () => { await api?.close(); await pool?.end(); });

test('归档后默认列表隐藏，include=archived 可见并带 is_owner', async () => {
  const id = await createProject('归档演练');
  await call('/projects/' + id + '/archive', 'POST');
  const active = await call('/projects');
  assert.ok(!active.some(p => p.id === id), '默认列表不应包含已归档项目');
  const withArchived = await call('/projects?include=archived');
  const archived = withArchived.find(p => p.id === id);
  assert.ok(archived, 'include=archived 应返回归档项目');
  assert.equal(archived.state, 'ARCHIVED');
  assert.equal(archived.is_owner, true);
  assert.ok(archived.archived_at, '应记录归档时间');
});

test('恢复后回到 SHARED 并重新出现在默认列表', async () => {
  const id = await createProject('恢复演练');
  await call('/projects/' + id + '/archive', 'POST');
  await call('/projects/' + id + '/unarchive', 'POST');
  const active = await call('/projects');
  const restored = active.find(p => p.id === id);
  assert.ok(restored, '恢复后应出现在默认列表');
  assert.equal(restored.state, 'SHARED');
});

test('已归档项目不能再次归档', async () => {
  const id = await createProject('重复归档演练');
  await call('/projects/' + id + '/archive', 'POST');
  await call('/projects/' + id + '/archive', 'POST', owner, undefined, 409);
});

test('STAGING 项目不能归档', async () => {
  const id = await createProject('暂存演练', false);
  await call('/projects/' + id + '/archive', 'POST', owner, undefined, 409);
});

test('非所有者不能归档或删除', async () => {
  const id = await createProject('权限演练');
  await call('/projects/' + id + '/archive', 'POST', other, undefined, 403);
  await call('/projects/' + id, 'DELETE', other, undefined, 403);
});

test('所有者删除：快照 404、实时数据清空、审计行按隐私架构保留', async () => {
  const id = await createProject('删除演练');
  const result = await call('/projects/' + id, 'DELETE');
  assert.equal(result.deleted, true);
  await call('/projects/' + id + '/snapshot', 'GET', owner, undefined, 404);
  const row = await pool.query('SELECT state, deleted_at FROM fr_projects WHERE id=$1', [id]);
  assert.equal(row.rows[0].state, 'DELETED', '项目行应转为 DELETED 态');
  assert.ok(row.rows[0].deleted_at, '应记录删除时间');
  for (const table of ['fr_previews', 'fr_grants', 'fr_comments']) {
    const left = await pool.query(`SELECT count(*)::int AS n FROM ${table} WHERE project_id=$1`, [id]);
    assert.equal(left.rows[0].n, 0, table + ' 应清空');
  }
  for (const table of ['fr_versions', 'fr_events']) {
    const kept = await pool.query(`SELECT count(*)::int AS n FROM ${table} WHERE project_id=$1`, [id]);
    assert.ok(kept.rows[0].n >= 1, table + ' 审计行应保留（0033 不可变架构）');
  }
  const active = await call('/projects');
  assert.ok(!active.some(p => p.id === id), '默认列表不应出现已删除项目');
  const withArchived = await call('/projects?include=archived');
  assert.ok(!withArchived.some(p => p.id === id), '归档列表也不应出现已删除项目');
  await call('/projects/' + id, 'DELETE', owner, undefined, 404);
});
