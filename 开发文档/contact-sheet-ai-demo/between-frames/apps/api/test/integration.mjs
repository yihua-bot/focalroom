import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import sharp from 'sharp';
import { WebSocket } from 'ws';
import * as Y from 'yjs';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
let api, pool, base, owner, reviewer, viewer, outsider, workspace, project, manifest, receipt;
const json = (method, token, body) => ({ method, headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), 'content-type': 'application/json' }, ...(body !== undefined ? { body: JSON.stringify(body) } : {}) });
async function call(path, method = 'GET', token = owner?.token, body, status = 200) {
  const response = await fetch(`${base}/api/v2${path}`, json(method, token, body)); const data = await response.json(); assert.equal(response.status, status, JSON.stringify(data)); return data;
}
const command = (kind, payload, revision = 0) => { const id = randomUUID(); return { version: '2.0', command_id: id, workspace_id: workspace, project_id: project, expected_revision: revision, idempotency_key: id, kind, payload }; };
const execute = (value, token = owner.token, status = 200) => call(`/projects/${project}/commands`, 'POST', token, value, status);
async function invite(account, role = 'REVIEWER') {
  const link = await call(`/workspaces/${workspace}/invitations`, 'POST', owner.token, { email: account.user.email, role, project_id: project });
  const token = new URL(link.invite_url).hash.split('=')[1]; await call('/invitations/accept', 'POST', account.token, { token }); return { link, token };
}
before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须由隔离 PostgreSQL runner 启动');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL }); await migrate(pool); await migrate(pool);
  api = await createApi({ pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'previews')), origin: 'http://127.0.0.1:1424' });
  await api.app.listen(0, '127.0.0.1'); base = await api.app.getUrl();
  const register = (email) => call('/auth/register', 'POST', null, { email, password: 'test-password-123', display_name: email });
  owner = await register('owner@fr.test'); reviewer = await register('reviewer@fr.test'); viewer = await register('viewer@fr.test'); outsider = await register('outsider@fr.test');
  workspace = (await call('/workspaces'))[0].id;
  manifest = { workspace_id: workspace, local_project_id: 'local-review', promotion_id: randomUUID(), title: 'FocalRoom 多人评片', from_revision: 0, snapshot: { photos: ['p1', 'p2', 'p3'].map(id => ({ id, filename: `${id}.jpg`, source_sha256: 'a'.repeat(64), selection: 'UNDECIDED' })), sequences: [{ id: 'sequence', title: '主序列', items: [{ id: 'item-p1', photo_id: 'p1' }] }] }, media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
});
after(async () => { if (api) await api.close(); if (pool) await pool.end(); });
test('M4 promotion verifies media, strips EXIF, atomically commits and replays', async () => {
  const start = await call('/promotions', 'POST', owner.token, manifest); project = start.project_id;
  assert.deepEqual(await call('/promotions', 'POST', owner.token, manifest), start);
  await call(`/projects/${project}/finalize`, 'POST', owner.token, { manifest_digest: start.manifest_digest }, 409);
  const jpeg = await sharp({ create: { width: 96, height: 64, channels: 3, background: '#2398ba' } }).jpeg().withMetadata({ exif: { IFD0: { Artist: 'private metadata' } } }).toBuffer();
  for (const photo of manifest.snapshot.photos) {
    const response = await fetch(`${base}/api/v2/projects/${project}/previews/${photo.id}`, { method: 'PUT', headers: { authorization: `Bearer ${owner.token}`, 'content-type': 'image/jpeg' }, body: jpeg }); assert.equal(response.status, 200, await response.text());
  }
  receipt = await call(`/projects/${project}/finalize`, 'POST', owner.token, { manifest_digest: start.manifest_digest });
  assert.deepEqual(await call(`/projects/${project}/finalize`, 'POST', owner.token, { manifest_digest: start.manifest_digest }), receipt);
  const ticket = await call(`/projects/${project}/previews/p1/ticket`, 'POST');
  const media = await fetch(`${base}${ticket.url}`); assert.equal(media.status, 200); const meta = await sharp(Buffer.from(await media.arrayBuffer())).metadata(); assert.equal(meta.exif, undefined); assert.equal(meta.width, 96);
  assert.equal((await fetch(`${base}${ticket.url}`)).status, 401, '单次票据不得重放');
  assert.equal(Number((await pool.query('SELECT count(*) FROM fr_versions')).rows[0].count), 1);
});
test('M4 project-only invitation, email binding, one-time acceptance and isolation', async () => {
  const invited = await invite(reviewer); await invite(viewer, 'VIEWER');
  await call('/invitations/accept', 'POST', reviewer.token, { token: invited.token }, 403);
  assert.equal((await call('/workspaces', 'GET', reviewer.token)).length, 1, '项目邀请不创建工作区成员');
  assert.equal((await call('/projects', 'GET', reviewer.token)).length, 1);
  await call(`/projects/${project}/snapshot`, 'GET', outsider.token, undefined, 403);
  const bad = await call(`/workspaces/${workspace}/invitations`, 'POST', owner.token, { email: 'unregistered@fr.test', role: 'VIEWER', project_id: project });
  await call('/invitations/accept', 'POST', outsider.token, { token: new URL(bad.invite_url).hash.split('=')[1] }, 403);
  await call(`/workspaces/${workspace}/invitations/${bad.id}`, 'DELETE');
  assert.ok(!(await pool.query('SELECT digest FROM fr_invitations LIMIT 1')).rows[0].digest.includes(invited.token));
});
test('M4 cookie CSRF, session hashes, unknown fields, no model authorization', async () => {
  const headers = { cookie: `fr_session=${owner.token}`, origin: 'http://127.0.0.1:1424', 'content-type': 'application/json' };
  const res = await fetch(`${base}/api/v2/projects/${project}/ws-ticket`, { method: 'POST', headers }); assert.equal(res.status, 403);
  headers['x-focalroom-csrf'] = owner.csrf;
  assert.equal((await fetch(`${base}/api/v2/projects/${project}/ws-ticket`, { method: 'POST', headers })).status, 200);
  assert.equal((await pool.query('SELECT count(*) FROM fr_sessions WHERE digest=$1', [owner.token])).rows[0].count, '0');
  const invalid = command('photo.set_selection', { photo_ids: ['p1'], selection_state: 'KEEP', unsafe: true }); await execute(invalid, owner.token, 400);
  const comments = await call(`/projects/${project}/ai-comments`, 'GET', reviewer.token); assert.equal(comments.grants_model_upload, false); assert.equal(comments.creates_author_memory, false);
});
test('M5 comment anchors, parent project isolation, provenance and Viewer denial', async () => {
  const note = command('comment.add', { body: '主体空间可再调整', anchor: { type: 'photo', photo_id: 'p1', x: 0.4, y: 0.6 } }); await execute(note, reviewer.token);
  await execute(command('comment.add', { body: 'forbidden', anchor: { type: 'photo', photo_id: 'p1', x: 0, y: 0 } }), viewer.token, 403);
  await execute(command('comment.add', { body: 'wrong', anchor: { type: 'photo', photo_id: 'foreign', x: 0, y: 0 } }), reviewer.token, 400);
  await call(`/projects/${project}/comments/${note.command_id}/replies`, 'POST', owner.token, { body: '收到，保留这个建议' });
  await call(`/projects/${project}/comments/foreign-thread/replies`, 'POST', reviewer.token, { body: 'wrong project' }, 400);
  const comments = await call(`/projects/${project}/ai-comments`); assert.equal(comments.comments[0].author_id, reviewer.user.id); assert.equal(comments.comments[0].reviewer, 'reviewer@fr.test');
});
test('M5 two concurrent proposals: one atomic formal apply, other remains submitted', async () => {
  const proposal = async (photo, position) => {
    const cmd = command('proposal.create', { title: `加入 ${photo}`, base_revision: 0, operations: [{ op: 'add', operation_id: `op-${photo}`, photo_id: photo, to_index: position }] });
    await execute(cmd, reviewer.token); await execute(command('proposal.submit', { proposal_id: cmd.command_id, expected_proposal_revision: 0 }), reviewer.token); return cmd.command_id;
  };
  const a = await proposal('p2', 1), b = await proposal('p3', 1);
  const state = await call(`/projects/${project}/snapshot`);
  const make = id => { const p = state.proposals.find(p => p.id === id); return command('proposal.apply', { proposal_id: id, expected_proposal_revision: p.revision, accepted_operation_ids: p.operations.map(o => o.operation_id), approval_nonce: p.nonce }); };
  await execute(make(a), reviewer.token, 403);
  const commands = [make(a), make(b)];
  const replies = await Promise.all(commands.map(c => fetch(`${base}/api/v2/projects/${project}/commands`, json('POST', owner.token, c))));
  assert.deepEqual(replies.map(r => r.status).sort(), [200, 409]);
  const winner = commands[replies.findIndex(r => r.status === 200)]; const loser = commands[replies.findIndex(r => r.status === 409)];
  const next = await call(`/projects/${project}/snapshot`); assert.equal(next.revision, 1); assert.equal(next.snapshot.sequences[0].items.length, 2); assert.equal(next.proposals.find(p => p.id === loser.payload.proposal_id).state, 'SUBMITTED');
  const replay = await execute(winner); assert.equal(replay.new_revision, 1);
  await execute({ ...winner, payload: { ...winner.payload, accepted_operation_ids: [] } }, owner.token, 409);
  assert.equal((await pool.query('SELECT count(*) FROM fr_versions WHERE project_id=$1', [project])).rows[0].count, '2');
  await assert.rejects(pool.query('UPDATE fr_versions SET revision=revision+1 WHERE project_id=$1', [project]), /immutable/);
});
async function websocket(account) {
  const ticket = await call(`/projects/${project}/ws-ticket`, 'POST', account.token);
  const ws = new WebSocket(base.replace('http', 'ws') + '/api/v2/realtime', { origin: 'http://127.0.0.1:1424' });
  const messages = []; ws.on('message', raw => messages.push(JSON.parse(raw.toString())));
  await new Promise((resolve, reject) => { ws.once('open', resolve); ws.once('error', reject); }); ws.send(JSON.stringify({ type: 'authenticate', ticket: ticket.token }));
  await until(() => messages.some(m => m.type === 'sync')); return { ws, messages };
}
async function until(predicate) { const end = Date.now() + 6000; while (!predicate()) { assert.ok(Date.now() < end, '等待实时消息超时'); await new Promise(r => setTimeout(r, 25)); } }
test('M5 authenticated WS, private Yjs, presence, snapshot reconnect and revoke', async () => {
  const a = await websocket(owner), b = await websocket(reviewer);
  a.ws.send(JSON.stringify({ type: 'presence', photo_id: 'p1' })); await until(() => b.messages.some(m => m.type === 'presence' && m.users.some(u => u.id === owner.user.id && u.photo_id === 'p1')));
  const doc = new Y.Doc(); doc.getText('note').insert(0, '私人草稿，不改正式序列'); a.ws.send(JSON.stringify({ type: 'draft', update: Buffer.from(Y.encodeStateAsUpdate(doc)).toString('base64') }));
  await until(() => a.messages.some(m => m.type === 'draft')); assert.ok(!b.messages.some(m => m.type === 'draft'));
  const mutate = command('photo.set_selection', { photo_ids: ['p1'], selection_state: 'KEEP' }, 1); await execute(mutate); await until(() => b.messages.some(m => m.type === 'events' && m.events.some(e => e.revision === 2)));
  a.ws.close(); await new Promise(resolve => a.ws.once('close', resolve));
  const reopened = await websocket(owner); const sync = reopened.messages.find(m => m.type === 'sync'); assert.equal(sync.revision, 2); const restored = new Y.Doc(); Y.applyUpdate(restored, Buffer.from(sync.draft, 'base64')); assert.equal(restored.getText('note').toString(), '私人草稿，不改正式序列');
  const ticket = await call(`/projects/${project}/previews/p1/ticket`, 'POST', reviewer.token);
  await call(`/projects/${project}/grants/${reviewer.user.id}`, 'DELETE');
  await until(() => b.ws.readyState === WebSocket.CLOSED);
  assert.equal((await fetch(`${base}${ticket.url}`)).status, 403, '未使用的旧媒体票据也必须拒绝');
  await call(`/projects/${project}/snapshot`, 'GET', reviewer.token, undefined, 403);
  assert.equal((await call(`/projects/${project}/snapshot`)).revision, 2);
  reopened.ws.close(); doc.destroy(); restored.destroy();
});
