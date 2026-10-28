import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { Pool } from 'pg';
import sharp from 'sharp';
import { join } from 'node:path';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { createSession } from '../src/auth.ts';
import { LocalPreviewStore } from '../src/storage.ts';

assert.ok(process.env.FOCALROOM_TEST_DIRECTORY);
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');

test('account data: scoped export, reauthentication, closure, private erasure and shared object retention', async () => {
  const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  const store = new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'erasure-media'));
  let removalFailure = true;
  let api;
  try {
    await migrate(pool);
    api = await createApi({ pool, store: {
      put: (key, bytes) => store.put(key, bytes), get: key => store.get(key),
      remove: async key => {
        if (removalFailure) { removalFailure = false; throw new Error('fixture: media temporarily unavailable'); }
        await store.remove(key);
      },
    }, origin: 'http://127.0.0.1:1424' });
    const erase = api.service.data.erase.bind(api.service.data);
    api.service.data.erase = async (...args) => {
      try { return await erase(...args); }
      catch (error) {
        if (!error.status) console.error('隔离测试数据库错误', error.code, error.message, error.constraint);
        throw error;
      }
    };
    await api.app.listen(0, '127.0.0.1');
    const base = await api.app.getUrl();
    const call = async (path, token, body, status = 200) => {
      const response = await fetch(base + '/api/v2' + path, { method: body === undefined ? 'GET' : 'POST',
        headers: { 'content-type': 'application/json', ...(token ? { Authorization: 'Bearer ' + token } : {}) },
        ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
      const result = await response.json();
      assert.equal(response.status, status, JSON.stringify(result));
      return result;
    };
    const password = 'account-erasure-fixture-2026';
    const owner = await call('/auth/register', null, { email: 'erase-owner@fr.test', password, display_name: '待删除' });
    const viewer = await call('/auth/register', null, { email: 'erase-viewer@fr.test', password, display_name: '协作者' });
    const workspace = (await call('/workspaces', owner.token))[0].id;
    const createProject = async (privateSync, background) => {
      const manifest = { workspace_id: workspace, local_project_id: randomUUID(), promotion_id: randomUUID(), title: '私有资料', from_revision: 0,
        snapshot: { photos: [{ id: 'p1', filename: 'fixture.jpg', source_sha256: 'b'.repeat(64), selection: 'KEEP' }],
          sequences: [{ id: 's1', title: '序列', items: [{ id: 'i1', photo_id: 'p1' }] }] },
        media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
      const result = await call(privateSync ? '/private-promotions' : '/promotions', owner.token, manifest);
      const image = await sharp({ create: { width: 64, height: 48, channels: 3, background } }).jpeg().toBuffer();
      const uploaded = await fetch(`${base}/api/v2/projects/${result.project_id}/previews/p1`, { method: 'PUT', headers: { Authorization: 'Bearer ' + owner.token, 'content-type': 'image/jpeg' }, body: image });
      assert.equal(uploaded.status, 200);
      const media = await uploaded.json();
      await call(`/projects/${result.project_id}/finalize`, owner.token, { manifest_digest: result.manifest_digest });
      return { id: result.project_id, key: media.sha256 + '.jpg' };
    };
    const first = await createProject(true, '#aabbcc');
    const unique = await createProject(true, '#cc8844');
    const shared = await createProject(false, '#aabbcc');
    assert.equal(first.key, shared.key, '跨项目内容去重夹具');
    const invitation = await call(`/workspaces/${workspace}/invitations`, owner.token, { email: viewer.user.email, role: 'VIEWER', project_id: shared.id });
    await call('/invitations/accept', viewer.token, { token: new URL(invitation.invite_url).hash.split('=')[1] });
    await call('/boards', owner.token, { projectId: unique.id, title: '待清理画布' });
    await call(`/projects/${unique.id}/research`, owner.token, { kind: 'note', title: '私密笔记', body: '应当清除的正文' });
    await call('/account/data-export', null, {}, 401);
    await call('/account/data-export', viewer.token, { user_id: owner.user.id }, 400);
    const exported = await call('/account/data-export', owner.token, {});
    assert.equal(exported.user_id, owner.user.id);
    assert.equal(exported.owned_projects.length, 3);
    assert.equal(exported.research[0].body, '应当清除的正文');
    const serialized = JSON.stringify(exported);
    for (const forbidden of [owner.token, password, 'password_hash', 'csrf', 'device_key', 'customer_id']) assert.ok(!serialized.includes(forbidden));
    const otherExport = await call('/account/data-export', viewer.token, {});
    assert.equal(otherExport.owned_projects.length, 0);
    const deletion = { password, confirm_email: owner.user.email, understand_retention: true };
    await call('/account/erase', owner.token, { ...deletion, password: 'wrong-password-value' }, 403);
    await call('/account/erase', owner.token, { ...deletion, confirm_email: viewer.user.email }, 403);
    await pool.query('INSERT INTO fr_ai_accounts(user_id,granted,reserved) VALUES($1,1000,100)', [owner.user.id]);
    await pool.query("INSERT INTO fr_ai_requests(user_id,request_id,body_hash,policy,reserved,state) VALUES($1,$2,'fixture','deepseek-flash-v1',100,'PENDING')", [owner.user.id, randomUUID()]);
    await call('/account/erase', owner.token, deletion, 409);
    await pool.query("UPDATE fr_ai_requests SET state='REJECTED',charged=0 WHERE user_id=$1", [owner.user.id]);
    await pool.query('UPDATE fr_ai_accounts SET reserved=0 WHERE user_id=$1', [owner.user.id]);
    const receipt = await call('/account/erase', owner.token, deletion);
    assert.equal(receipt.closed, true);
    assert.equal(receipt.erased_private_projects, 2);
    assert.equal(receipt.media_cleanup, 'scheduled');
    assert.ok((await pool.query('SELECT count(*)::int AS n FROM fr_erasure_objects')).rows[0].n > 0, '介质故障保留持久重试项');
    await api.service.data.clean();
    await call('/auth/me', owner.token, undefined, 401);
    await call('/auth/login', null, { email: owner.user.email, password }, 401);
    await assert.rejects(createSession(pool, { id: owner.user.id, email: owner.user.email, display_name: '旧登录并发结果' }), error => error.code === 'AUTH_REVOKED');
    await assert.rejects(api.service.updateProfile({ id: owner.user.id }, { display_name: '迟到的旧资料请求' }), error => error.code === 'AUTH_REVOKED');
    const erased = (await pool.query('SELECT email,display_name,avatar_url FROM fr_users WHERE id=$1', [owner.user.id])).rows[0];
    assert.ok(erased.email.endsWith('@deleted.invalid'));
    assert.equal(erased.display_name, '已删除账户');
    assert.equal((await pool.query('SELECT count(*)::int AS n FROM fr_previews WHERE project_id=ANY($1)', [[first.id, unique.id]])).rows[0].n, 0);
    assert.equal((await pool.query('SELECT count(*)::int AS n FROM fr_research_references WHERE project_id=$1', [unique.id])).rows[0].n, 0);
    const retained = await call(`/projects/${shared.id}/snapshot`, viewer.token);
    assert.equal(retained.snapshot.photos.length, 1, '协作者已获授权的作品保留');
    assert.ok((await store.get(shared.key)).length > 0, '共享对象仍有引用时禁止物理删除');
    await assert.rejects(store.get(unique.key), error => error.code === 'ENOENT');
    assert.equal((await pool.query('SELECT count(*)::int AS n FROM fr_erasure_objects')).rows[0].n, 0);
    const erasedHistory = (await pool.query('SELECT snapshot FROM fr_versions WHERE project_id=$1', [unique.id])).rows;
    assert.ok(erasedHistory.every(row => row.snapshot.photos.length === 0));
    await assert.rejects(pool.query("UPDATE fr_versions SET snapshot='{\"photos\":[1],\"sequences\":[]}' WHERE project_id=$1", [unique.id]), /immutable audit record/);
    await assert.rejects(pool.query('UPDATE fr_versions SET actor_id=$2 WHERE project_id=$1', [unique.id, viewer.user.id]), /immutable audit record/);
    await assert.rejects(pool.query('DELETE FROM fr_erasure_audit WHERE project_id=$1', [unique.id]), /immutable audit record/);
    await assert.rejects(pool.query("UPDATE fr_versions SET snapshot='{\"photos\":[],\"sequences\":[]}' WHERE project_id=$1", [shared.id]), /immutable audit record/);
  } finally { await api?.close(); await pool.end(); }
});
