import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
import { join } from 'node:path';

assert.ok(process.env.FOCALROOM_TEST_DIRECTORY);
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');

test('private sync: single cloud authority, owner-only ACL, explicit sharing, revocation and immutable receipts', async () => {
  const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  let api;
  try {
    await migrate(pool);
    api = await createApi({ pool, origin: 'http://127.0.0.1:1424', store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'private-previews')) });
    await api.app.listen(0, '127.0.0.1');
    const base = await api.app.getUrl();
    const call = async (path, token, body, status = 200, method = body === undefined ? 'GET' : 'POST') => {
      const response = await fetch(base + '/api/v2' + path, { method,
        headers: { 'content-type': 'application/json', ...(token ? { Authorization: 'Bearer ' + token } : {}) },
        ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
      const result = await response.json();
      assert.equal(response.status, status, JSON.stringify(result));
      return result;
    };
    const register = email => call('/auth/register', null, { email, password: 'private-sync-fixture-2026', display_name: email });
    const owner = await register('sync-owner@fr.test'), admin = await register('sync-admin@fr.test'), viewer = await register('sync-viewer@fr.test');
    const workspace = (await call('/workspaces', owner.token))[0].id;
    const adminInvite = await call(`/workspaces/${workspace}/invitations`, owner.token, { email: admin.user.email, role: 'ADMIN' });
    await call('/invitations/accept', admin.token, { token: new URL(adminInvite.invite_url).hash.split('=')[1] });
    const manifest = { workspace_id: workspace, local_project_id: randomUUID(), promotion_id: randomUUID(), title: '私有同步', from_revision: 0,
      snapshot: { photos: [], sequences: [{ id: 'private-sequence', title: '作品序列', items: [] }] }, media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
    const image = await sharp({ create: { width: 64, height: 48, channels: 3, background: '#aaccee' } }).jpeg().toBuffer();
    manifest.snapshot.photos.push({ id: 'private-photo', filename: 'fixture.jpg', source_sha256: 'a'.repeat(64), selection: 'UNDECIDED' });
    const start = await call('/private-promotions', owner.token, manifest);
    const id = start.project_id;
    assert.equal((await call(`/projects/${id}/sync-state`, owner.token)).mode, 'SYNC_ENROLLING');
    assert.equal((await call('/private-promotions', owner.token, manifest)).project_id, id);
    const uploaded = await fetch(`${base}/api/v2/projects/${id}/previews/private-photo`, { method: 'PUT', headers: { Authorization: 'Bearer ' + owner.token, 'content-type': 'image/jpeg' }, body: image });
    assert.equal(uploaded.status, 200);
    await call(`/projects/${id}/finalize`, owner.token, { manifest_digest: start.manifest_digest });
    assert.equal((await call(`/projects/${id}/sync-state`, owner.token)).mode, 'PRIVATE_SYNCED');
    assert.ok(!(await call('/projects', admin.token)).some(p => p.id === id), '工作区管理员不能发现私人项目');
    await call(`/projects/${id}/snapshot`, admin.token, undefined, 403);
    await call(`/workspaces/${workspace}/invitations`, owner.token, { email: viewer.user.email, role: 'VIEWER', project_id: id }, 409);
    const sharing = { command_id: randomUUID(), expected_version: 0, mode: 'SHARED' };
    const shared = await call(`/projects/${id}/sync-state`, owner.token, sharing);
    assert.equal(shared.version, 1);
    assert.deepEqual(await call(`/projects/${id}/sync-state`, owner.token, sharing), shared);
    assert.ok((await call('/projects', admin.token)).some(p => p.id === id));
    const invitation = await call(`/workspaces/${workspace}/invitations`, owner.token, { email: viewer.user.email, role: 'VIEWER', project_id: id });
    await call('/invitations/accept', viewer.token, { token: new URL(invitation.invite_url).hash.split('=')[1] });
    const ticket = await call(`/projects/${id}/previews/private-photo/ticket`, viewer.token, {});
    await call(`/projects/${id}/sync-state`, viewer.token, { command_id: randomUUID(), expected_version: 1, mode: 'PRIVATE_SYNCED' }, 403);
    const privateAgain = await call(`/projects/${id}/sync-state`, owner.token, { command_id: randomUUID(), expected_version: 1, mode: 'PRIVATE_SYNCED' });
    assert.equal(privateAgain.version, 2);
    await call(`/projects/${id}/snapshot`, viewer.token, undefined, 403);
    await call(`/projects/${id}/snapshot`, admin.token, undefined, 403);
    const deniedTicket = await fetch(base + ticket.url);
    assert.equal(deniedTicket.status, 403, '旧媒体票据不绕过收回共享');
    assert.equal((await pool.query('SELECT active FROM fr_grants WHERE project_id=$1', [id])).rows[0].active, false);
    assert.deepEqual(await call(`/projects/${id}/sync-state`, owner.token, sharing), shared, '旧编号只返回旧回执，不能再次开启共享');
    assert.equal((await call(`/projects/${id}/sync-state`, owner.token)).mode, 'PRIVATE_SYNCED');
    await call(`/projects/${id}/sync-state`, owner.token, { command_id: randomUUID(), expected_version: 0, mode: 'SHARED' }, 409);
    assert.equal((await pool.query('SELECT revision FROM fr_projects WHERE id=$1', [id])).rows[0].revision, 0, '模式变化不伪造照片版本');
    assert.equal((await pool.query('SELECT count(*)::int AS n FROM fr_sync_commands WHERE project_id=$1', [id])).rows[0].n, 2);
    const legacy = await call('/promotions', owner.token, { ...manifest, promotion_id: randomUUID() });
    await call(`/projects/${legacy.project_id}/sync-state`, admin.token, undefined, 403);
  } finally { await api?.close(); await pool.end(); }
});
