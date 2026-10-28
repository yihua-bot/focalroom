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
  const corrupt = await fetch(base + '/api/v2/projects/' + project + '/previews/p1', {method:'PUT',headers:{authorization:'Bearer '+owner.token,'content-type':'image/jpeg'},body:Buffer.from('broken jpeg')}); assert.equal(corrupt.status,400);
  const jpeg = await sharp({ create: { width: 96, height: 64, channels: 3, background: '#2398ba' } }).jpeg().withMetadata({ exif: { IFD0: { Artist: 'private metadata' } } }).toBuffer();
  for (const photo of manifest.snapshot.photos) {
    const response = await fetch(`${base}/api/v2/projects/${project}/previews/${photo.id}`, { method: 'PUT', headers: { authorization: `Bearer ${owner.token}`, 'content-type': 'image/jpeg' }, body: jpeg }); assert.equal(response.status, 200, await response.text());
  }
  receipt = await call(`/projects/${project}/finalize`, 'POST', owner.token, { manifest_digest: start.manifest_digest });
  assert.deepEqual(await call(`/projects/${project}/finalize`, 'POST', owner.token, { manifest_digest: start.manifest_digest }), receipt);
  const ticket = await call(`/projects/${project}/previews/p1/ticket`, 'POST');
  const media = await fetch(`${base}${ticket.url}`); assert.equal(media.status, 200); const meta = await sharp(Buffer.from(await media.arrayBuffer())).metadata(); assert.equal(meta.exif, undefined); assert.equal(meta.width, 96);
  assert.equal((await fetch(`${base}${ticket.url}`)).status, 401, '单次票据不得重放');
  assert.equal(Number((await pool.query('SELECT count(*) FROM fr_versions WHERE project_id=$1', [project])).rows[0].count), 1);
});
test('M4 non-UUID photo IDs retain stable asset aliases after preview upload', async () => {
  const rows = (await pool.query(
    'SELECT a.photo_id,a.project_id,a.asset_id,s.id,s.photo_id AS source_photo_id FROM fr_asset_aliases a JOIN fr_source_assets s ON s.id=a.asset_id WHERE a.project_id=$1 ORDER BY a.photo_id',
    [project],
  )).rows;
  assert.equal(rows.length, 3, '不能吞掉非 UUID 照片的资产映射失败');
  assert.deepEqual(rows.map(row => row.photo_id), ['p1', 'p2', 'p3']);
  for (const row of rows) {
    assert.equal(row.project_id, project);
    assert.equal(row.asset_id, row.id);
    assert.equal(row.photo_id, row.source_photo_id);
    assert.match(row.asset_id, /^[a-f0-9-]{36}$/);
  }
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
  await execute(command('comment.add', { body: '开场这张放第一位更稳', anchor: { type: 'sequence', sequence_item_id: 'item-p1', revision_id: '0' } }), reviewer.token);
  await execute(command('comment.add', { body: '这一版整体节奏偏快', anchor: { type: 'revision', revision_id: '0' } }), reviewer.token);
  await execute(command('comment.add', { body: 'wrong item', anchor: { type: 'sequence', sequence_item_id: 'item-foreign', revision_id: '0' } }), reviewer.token, 400);
  await execute(command('comment.add', { body: 'wrong revision', anchor: { type: 'revision', revision_id: '9' } }), reviewer.token, 400);
  await call(`/projects/${project}/comments/${note.command_id}/replies`, 'POST', owner.token, { body: '收到，保留这个建议', idempotency_key: randomUUID() });
  await call(`/projects/${project}/comments/foreign-thread/replies`, 'POST', reviewer.token, { body: 'wrong project', idempotency_key: randomUUID() }, 400);
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
  // 断言失败也必须关闭连接：残留 WS 会挂住整个套件进程。
  let reopened; const doc = new Y.Doc(); let restored;
  try {
    a.ws.send(JSON.stringify({ type: 'presence', photo_id: 'p1' })); await until(() => b.messages.some(m => m.type === 'presence' && m.users.some(u => u.id === owner.user.id && u.photo_id === 'p1')));
    doc.getText('note').insert(0, '私人草稿，不改正式序列'); a.ws.send(JSON.stringify({ type: 'draft', update: Buffer.from(Y.encodeStateAsUpdate(doc)).toString('base64') }));
    await until(() => a.messages.some(m => m.type === 'draft')); assert.ok(!b.messages.some(m => m.type === 'draft'));
    const mutate = command('photo.set_selection', { photo_ids: ['p1'], selection_state: 'KEEP' }, 1); await execute(mutate); await until(() => b.messages.some(m => m.type === 'events' && m.events.some(e => e.revision === 2)));
    a.ws.close(); await new Promise(resolve => a.ws.once('close', resolve));
    reopened = await websocket(owner); const sync = reopened.messages.find(m => m.type === 'sync'); assert.equal(sync.revision, 2); restored = new Y.Doc(); Y.applyUpdate(restored, Buffer.from(sync.draft, 'base64')); assert.equal(restored.getText('note').toString(), '私人草稿，不改正式序列');
    const ticket = await call(`/projects/${project}/previews/p1/ticket`, 'POST', reviewer.token);
    await call(`/projects/${project}/grants/${reviewer.user.id}`, 'DELETE');
    // 撤权后的新图片票据请求与命令提交必须立即阻断（G4 完整回归）。
    await call(`/projects/${project}/previews/p1/ticket`, 'POST', reviewer.token, undefined, 403);
    await execute(command('photo.set_selection', { photo_ids: ['p1'], selection_state: 'KEEP' }, 2), reviewer.token, 403);
    await until(() => b.ws.readyState === WebSocket.CLOSED);
    assert.equal((await fetch(`${base}${ticket.url}`)).status, 403, '未使用的旧媒体票据也必须拒绝');
    await call(`/projects/${project}/snapshot`, 'GET', reviewer.token, undefined, 403);
    assert.equal((await call(`/projects/${project}/snapshot`)).revision, 2);
  } finally {
    a.ws.close(); b.ws.close(); reopened?.ws.close(); doc.destroy(); restored?.destroy();
  }
});
test('M5 cross-instance presence syncs through the PostgreSQL bus', async () => {
  const pool2 = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  const second = await createApi({ pool: pool2, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'previews-second')), origin: 'http://127.0.0.1:1424' });
  await second.app.listen(0, '127.0.0.1'); const base2 = await second.app.getUrl();
  const connectAt = async (target, account) => {
    const ticket = await call(`/projects/${project}/ws-ticket`, 'POST', account.token);
    const ws = new WebSocket(target.replace('http', 'ws') + '/api/v2/realtime', { origin: 'http://127.0.0.1:1424' });
    const messages = []; ws.on('message', raw => messages.push(JSON.parse(raw.toString())));
    await new Promise((resolve, reject) => { ws.once('open', resolve); ws.once('error', reject); }); ws.send(JSON.stringify({ type: 'authenticate', ticket: ticket.token }));
    await until(() => messages.some(m => m.type === 'sync')); return { ws, messages };
  };
  const onA = await connectAt(base, owner);
  const onB = await connectAt(base2, viewer);
  onB.ws.send(JSON.stringify({ type: 'presence', photo_id: 'p1' }));
  await until(() => onA.messages.some(m => m.type === 'presence' && m.users.some(u => u.id === viewer.user.id && u.photo_id === 'p1')), '实例一应看到实例二的在线状态');
  await until(() => onB.messages.some(m => m.type === 'presence' && m.users.some(u => u.id === owner.user.id)), '实例二应看到实例一的在线状态');
  onB.ws.close(); await new Promise(resolve => onB.ws.once('close', resolve));
  await until(() => onA.messages.some(m => m.type === 'presence' && !m.users.some(u => u.id === viewer.user.id)), '断开后应从广播中消失');
  onA.ws.close(); await second.close(); await pool2.end();
});
test('M5 short task principals are project scoped, read-only, expiring and revocable', async () => {
  const scoped = await call(`/projects/${project}/agent-scopes`, 'POST', viewer.token);
  const read = async (token, status = 200) => { const response = await fetch(`${base}/api/v2/agent/comments`, { headers: { authorization: `Task ${token}` } }); const value = await response.json(); assert.equal(response.status, status, JSON.stringify(value)); return value; };
  const result = await read(scoped.task_token); assert.equal(result.project_id, project); assert.ok(result.comments.every(c => c.author_id && c.id && c.reviewer)); assert.equal(result.permits_model_upload, false);
  const wrongWrite = await fetch(`${base}/api/v2/projects/${project}/commands`, { method: 'POST', headers: { authorization: `Task ${scoped.task_token}`, 'content-type': 'application/json' }, body: JSON.stringify(command('photo.set_selection', { photo_ids: ['p1'], selection_state: 'REJECT' }, 2)) }); assert.equal(wrongWrite.status, 401);
  const { readScopedComments } = await import('../../../workers/agent/src/comments.ts'); assert.equal((await readScopedComments(base, scoped.task_token, project)).comments.length, result.comments.length); await assert.rejects(readScopedComments(base, scoped.task_token, 'other-project'));
  await call(`/projects/${project}/agent-scopes/${scoped.id}`, 'DELETE', viewer.token); await read(scoped.task_token, 401);
  const expires = await call(`/projects/${project}/agent-scopes`, 'POST', viewer.token); await pool.query("UPDATE fr_agent_scopes SET expires_at=now()-interval '1 second' WHERE id=$1", [expires.id]); await read(expires.task_token, 401);
  const revoked = await call(`/projects/${project}/agent-scopes`, 'POST', viewer.token); await call(`/projects/${project}/grants/${viewer.user.id}`, 'DELETE'); await read(revoked.task_token, 403);
});
test('M4 malformed and oversized requests return safe private JSON errors', async () => {
  for (const [body, status] of [['{broken', 400], [JSON.stringify({ text: 'x'.repeat(2100000) }), 413]]) { const response = await fetch(`${base}/api/v2/projects`, { method: 'POST', headers: { 'content-type': 'application/json' }, body }); assert.equal(response.status, status); assert.equal(response.headers.get('cache-control'), 'private, no-store'); const value = await response.json(); assert.equal(value.error.code, 'VALIDATION_ERROR'); assert.ok(!JSON.stringify(value).includes('stack')); }
});
test('M4 invitations expire and membership never grants ordinary project access', async () => {
  const invitation = await call(`/workspaces/${workspace}/invitations`, 'POST', owner.token, { email: outsider.user.email, role: 'EDITOR' }); const token = new URL(invitation.invite_url).hash.split('=')[1]; await call('/invitations/accept', 'POST', outsider.token, { token });
  assert.equal((await call('/workspaces', 'GET', outsider.token)).length, 2); assert.equal((await call('/projects', 'GET', outsider.token)).length, 0); await call(`/projects/${project}/snapshot`, 'GET', outsider.token, undefined, 403);
  const expired = await call(`/workspaces/${workspace}/invitations`, 'POST', owner.token, { email: outsider.user.email, role: 'VIEWER', project_id: project }); await pool.query("UPDATE fr_invitations SET expires_at=now()-interval '1 second' WHERE id=$1", [expired.id]); await call('/invitations/accept', 'POST', outsider.token, { token: new URL(expired.invite_url).hash.split('=')[1] }, 403);
});
test('M5 Yjs rejects formal state fields and viewer replay cannot bypass downgraded ACL', async () => {
  const viewerInvite = await invite(viewer, 'EDITOR'); assert.ok(viewerInvite.token); await call(`/projects/${project}/grants/${viewer.user.id}`, 'PATCH', owner.token, { role: 'EDITOR', can_approve: false, can_edit_formal: true });
  const edit = command('photo.set_selection', { photo_ids: ['p2'], selection_state: 'KEEP' }, 2); await execute(edit, viewer.token); await call(`/projects/${project}/grants/${viewer.user.id}`, 'PATCH', owner.token, { role: 'VIEWER', can_approve: false, can_edit_formal: false }); await execute(edit, viewer.token, 403);
  const connection = await websocket(owner); const bad = new Y.Doc(); bad.getMap('sequence').set('order', ['p3', 'p2', 'p1']); connection.ws.send(JSON.stringify({ type: 'draft', update: Buffer.from(Y.encodeStateAsUpdate(bad)).toString('base64') })); await until(() => connection.ws.readyState === WebSocket.CLOSED); assert.equal((await call(`/projects/${project}/snapshot`)).revision, 3); bad.destroy();
});
test('M4/M5 API restart preserves session, authoritative versions and comments', async () => {
  await api.close(); api = await createApi({ pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'previews')), origin: 'http://127.0.0.1:1424' }); await api.app.listen(0, '127.0.0.1'); base = await api.app.getUrl();
  const snapshot = await call(`/projects/${project}/snapshot`); assert.equal(snapshot.revision, 3); assert.ok(snapshot.comments.length >= 2); assert.equal((await call(`/projects/${project}/history`)).length, 4);
  await assert.rejects(pool.query('UPDATE fr_acl_audit SET allowed=false'), /immutable/);
});
test('M4 workspace member removal preserves independent project grants and protects owner', async () => {
  const list = await call(`/workspaces/${workspace}/members`); assert.ok(list.some(m => m.user_id === outsider.user.id));
  await call(`/workspaces/${workspace}/members/${owner.user.id}`, 'DELETE', owner.token, undefined, 403);
  await call(`/workspaces/${workspace}/members`, 'GET', outsider.token, undefined, 403);
  const grantBefore = (await pool.query('SELECT count(*) FROM fr_grants WHERE project_id=$1', [project])).rows[0].count;
  await call(`/workspaces/${workspace}/members/${outsider.user.id}`, 'DELETE'); assert.equal((await call('/workspaces', 'GET', outsider.token)).length, 1); assert.equal((await pool.query('SELECT count(*) FROM fr_grants WHERE project_id=$1', [project])).rows[0].count, grantBefore);
});
test('M5 draft revisions are private, owned, idempotent and partially approved with complete audit', async () => {
  await invite(reviewer);
  const baseline = await call(`/projects/${project}/snapshot`); assert.equal(baseline.revision, 3);
  const first = baseline.snapshot.sequences[0].items[0].photo_id, other = baseline.snapshot.sequences[0].items[1].photo_id;
  const operations = [{ op: 'move', operation_id: 'move-first', photo_id: other, to_index: 0 }, { op: 'remove', operation_id: 'remove-original', photo_id: first }];
  const draft = command('proposal.create', { title: '可修订私人提案', base_revision: 3, operations }, 3); await execute(draft, reviewer.token);
  assert.ok(!(await call(`/projects/${project}/snapshot`)).proposals.some(p => p.id === draft.command_id), '未提交草案不对作者以外成员开放');
  const input = { idempotency_key: randomUUID(), expected_proposal_revision: 0, base_revision: 3, title: '修订后的提案', operations };
  await call(`/projects/${project}/proposals/${draft.command_id}`, 'PATCH', owner.token, input, 403);
  const revision = await call(`/projects/${project}/proposals/${draft.command_id}`, 'PATCH', reviewer.token, input); assert.equal(revision.proposal_revision, 1); assert.deepEqual(await call(`/projects/${project}/proposals/${draft.command_id}`, 'PATCH', reviewer.token, input), revision);
  await call(`/projects/${project}/proposals/${draft.command_id}`, 'PATCH', reviewer.token, { ...input, idempotency_key: randomUUID() }, 409);
  await call(`/projects/${project}/proposals/${draft.command_id}`, 'PATCH', reviewer.token, { ...input, expected_proposal_revision: 1, idempotency_key: randomUUID(), operations: [operations[0], operations[0]] }, 400);
  await execute(command('proposal.submit', { proposal_id: draft.command_id, expected_proposal_revision: 1 }, 3), reviewer.token);
  await call(`/projects/${project}/proposals/${draft.command_id}`, 'PATCH', reviewer.token, { ...input, expected_proposal_revision: 2, idempotency_key: randomUUID() }, 409);
  const proposal = (await call(`/projects/${project}/snapshot`)).proposals.find(p => p.id === draft.command_id);
  const approval = command('proposal.apply', { proposal_id: proposal.id, expected_proposal_revision: proposal.revision, approval_nonce: proposal.nonce, accepted_operation_ids: ['move-first'] }, 3); await execute(approval);
  const after = await call(`/projects/${project}/snapshot`); assert.equal(after.revision, 4); assert.equal(after.snapshot.sequences[0].items[0].photo_id, other); assert.ok(after.snapshot.sequences[0].items.some(i => i.photo_id === first), '未接受的移除不得发生');
  const audit = (await pool.query('SELECT envelope FROM fr_commands WHERE project_id=$1 AND command_id=$2', [project, approval.command_id])).rows[0].envelope; assert.deepEqual(audit, approval);
  const historical = await call(`/projects/${project}/versions/3`); assert.deepEqual(historical.snapshot, baseline.snapshot); assert.equal((await call(`/projects/${project}/snapshot`)).revision, 4);
  await call(`/projects/${project}/versions/3`, 'GET', outsider.token, undefined, 403);
  const rejected = command('proposal.create', { title: '空选驳回', base_revision: 4, operations: [{ op: 'remove', operation_id: 'remove', photo_id: first }] }, 4); await execute(rejected, reviewer.token); await execute(command('proposal.submit', { proposal_id: rejected.command_id, expected_proposal_revision: 0 }, 4), reviewer.token);
  const rejectProposal = (await call(`/projects/${project}/snapshot`)).proposals.find(p => p.id === rejected.command_id); await execute(command('proposal.apply', { proposal_id: rejectProposal.id, expected_proposal_revision: rejectProposal.revision, approval_nonce: rejectProposal.nonce, accepted_operation_ids: [] }, 4)); assert.equal((await call(`/projects/${project}/snapshot`)).revision, 4); assert.equal((await call(`/projects/${project}/history`)).length, 5);
});
test('M5 thread replies replay exactly and preserve an immutable original payload', async () => {
  const baseline = await call(`/projects/${project}/snapshot`), thread = baseline.comments.find(c => !c.parent_id).id;
  const input = { body: '幂等回复内容', idempotency_key: randomUUID() }; const path = `/projects/${project}/comments/${thread}/replies`;
  const reply = await call(path, 'POST', reviewer.token, input); assert.deepEqual(await call(path, 'POST', reviewer.token, input), reply); await call(path, 'POST', reviewer.token, { ...input, body: '不同内容' }, 409);
  assert.equal((await call(`/projects/${project}/snapshot`)).comments.length, baseline.comments.length + 1); const record = (await pool.query('SELECT envelope FROM fr_commands WHERE command_id=$1', [reply.id])).rows[0].envelope; assert.equal(record.thread_id, thread); assert.equal(record.body, input.body);
});
test('M4 append-only migration upgrades 0001 without rewriting legacy receipts', async () => {
  const { readFile } = await import('node:fs/promises'); const { digest, assertSchema } = await import('../src/database.ts');
  await pool.query('CREATE DATABASE fr_upgrade_fixture'); const url = new URL(process.env.FOCALROOM_TEST_DATABASE_URL); url.pathname = '/fr_upgrade_fixture'; const legacy = new Pool({ connectionString: url.href });
  try {
    const sql = await readFile(new URL('../migrations/0001_shared_authority.sql', import.meta.url), 'utf8'); await legacy.query(sql); await legacy.query('INSERT INTO fr_migrations VALUES($1,$2,now())', ['0001', digest(sql)]);
    await legacy.query("INSERT INTO fr_users(id,email,display_name,password_hash) VALUES('u','u@upgrade.test','legacy','not-an-account')"); await legacy.query("INSERT INTO fr_workspaces VALUES('w','legacy','u')"); await legacy.query("INSERT INTO fr_projects(id,workspace_id,owner_id,title,state,revision,snapshot,promotion_key,manifest_digest,initial_revision) VALUES('p','w','u','legacy','SHARED',0,'{}','key','hash',0)"); await legacy.query("INSERT INTO fr_commands(project_id,command_id,actor_id,idempotency_key,payload_digest,reply) VALUES('p','old','u','old','legacy',$1)", [JSON.stringify({ok:true})]);
    await assert.rejects(assertSchema(legacy), /显式执行迁移/); await migrate(legacy); await migrate(legacy); await assertSchema(legacy);
    const record = (await legacy.query("SELECT envelope,reply FROM fr_commands WHERE command_id='old'")).rows[0]; assert.equal(record.envelope, null); assert.deepEqual(record.reply, { ok: true });
    await assert.rejects(legacy.query("INSERT INTO fr_commands(project_id,command_id,actor_id,idempotency_key,payload_digest,reply) VALUES('p','new','u','new','hash','{}')"), /requires envelope/); assert.equal((await legacy.query("SELECT sha256 FROM fr_migrations WHERE version='0001'")).rows[0].sha256, digest(sql));
  } finally { await legacy.end(); }
});
test('M4 project authority does not authorize cloud model transmission', async () => {
  const before = (await pool.query('SELECT count(*) FROM fr_agent_task_results')).rows[0].count;
  // 无效授权引用：项目权限与成员身份都不构成向模型发送评论的同意。
  const result = await call(`/projects/${project}/agent-tasks/summarize`, 'POST', owner.token, { kind: 'summarize_comments', consent_id: 'no-such-consent' }, 403);
  assert.equal(result.error.code, 'MODEL_CONSENT_REQUIRED');
  await call(`/projects/${project}/agent-tasks/summarize`, 'POST', outsider.token, { kind: 'summarize_comments', consent_id: 'no-such-consent' }, 403);
  assert.equal((await pool.query('SELECT count(*) FROM fr_agent_task_results')).rows[0].count, before);
});
test('G3 revision diff compares immutable snapshots: field-level changes, boundaries and scoping', async () => {
  // 独立项目 + 新观察者：两轮选片命令制造已知字段级差异。
  const diffViewer = await call('/auth/register', 'POST', null, { email: 'diff-viewer@fr.test', password: 'test-password-123', display_name: 'diff-viewer@fr.test' });
  const manifestDiff = { workspace_id: workspace, local_project_id: 'diff-fixture', promotion_id: randomUUID(), title: '版本比较项目', from_revision: 0,
    snapshot: { photos: ['d1', 'd2'].map(id => ({ id, filename: `${id}.jpg`, source_sha256: 'd'.repeat(64), selection: 'UNDECIDED' })),
      sequences: [{ id: 'sequence', title: '主序列', items: [] }] },
    media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
  const start = await call('/promotions', 'POST', owner.token, manifestDiff);
  const diffProject = start.project_id;
  const jpeg = await sharp({ create: { width: 96, height: 64, channels: 3, background: '#98ba23' } }).jpeg().toBuffer();
  for (const photo of manifestDiff.snapshot.photos) {
    const response = await fetch(`${base}/api/v2/projects/${diffProject}/previews/${photo.id}`, { method: 'PUT', headers: { authorization: `Bearer ${owner.token}`, 'content-type': 'image/jpeg' }, body: jpeg });
    assert.equal(response.status, 200);
  }
  await call(`/projects/${diffProject}/finalize`, 'POST', owner.token, { manifest_digest: start.manifest_digest });
  const issue = (kind, payload, revision) => call(`/projects/${diffProject}/commands`, 'POST', owner.token,
    { ...command(kind, payload), project_id: diffProject, expected_revision: revision });
  await issue('photo.set_selection', { photo_ids: ['d1'], selection_state: 'KEEP' }, 0);
  await issue('photo.set_selection', { photo_ids: ['d2'], selection_state: 'KEEP' }, 1);
  const inviteDiff = await call(`/workspaces/${workspace}/invitations`, 'POST', owner.token, { email: diffViewer.user.email, role: 'VIEWER', project_id: diffProject });
  await call('/invitations/accept', 'POST', diffViewer.token, { token: new URL(inviteDiff.invite_url).hash.split('=')[1] });

  // 字段级差异：d2 的 selection 变化，d1 稳定，序列无差异。
  const diff = await call(`/projects/${diffProject}/versions/1/diff/2`, 'GET', owner.token);
  assert.equal(diff.from.revision, 1); assert.equal(diff.to.revision, 2);
  assert.deepEqual(diff.photos.added, []); assert.deepEqual(diff.photos.removed, []);
  assert.equal(diff.photos.changed.length, 1);
  assert.equal(diff.photos.changed[0].id, 'd2');
  assert.deepEqual(diff.photos.changed[0].fields.selection, { before: 'UNDECIDED', after: 'KEEP' });
  assert.deepEqual(diff.sequences, { added: [], removed: [], changed: [] });

  // 跨多版本：d1 与 d2 同时出现在 0→2 差异中。
  const wide = await call(`/projects/${diffProject}/versions/0/diff/2`, 'GET', owner.token);
  assert.deepEqual(wide.photos.changed.map(change => change.id).sort(), ['d1', 'd2']);

  // 边界与权限：同版本 400、缺失版本 404、未授权 403、被授权观察者可读。
  await call(`/projects/${diffProject}/versions/2/diff/2`, 'GET', owner.token, undefined, 400);
  await call(`/projects/${diffProject}/versions/1/diff/99`, 'GET', owner.token, undefined, 404);
  await call(`/projects/${diffProject}/versions/1/diff/2`, 'GET', outsider.token, undefined, 403);
  assert.equal((await call(`/projects/${diffProject}/versions/1/diff/2`, 'GET', diffViewer.token)).photos.changed.length, 1);
});
test('I1 research references are project-scoped auxiliary material: permission-gated CRUD, identity-separated from formal assets', async () => {
  const revisionBefore = (await call(`/projects/${project}/snapshot`)).revision;
  const assetsBefore = Number((await pool.query('SELECT count(*)::int AS n FROM fr_source_assets')).rows[0].n);

  // 笔记与链接两类参考；作者/来源/许可元数据随行。
  const note = await call(`/projects/${project}/research`, 'POST', owner.token,
    { kind: 'note', title: '寇德卡画册编排笔记', body: '盲页开场，章节间留白节奏参考《Exiles》。', author: '内部整理', source: '画册《Exiles》', license: '内部参考' });
  assert.ok(note.id);
  const link = await call(`/projects/${project}/research`, 'POST', reviewer.token,
    { kind: 'link', title: 'Magnum 展览页', url: 'https://www.magnumphotos.com/exhibition' });
  assert.ok(link.id);

  const list = await call(`/projects/${project}/research`, 'GET', viewer.token);
  assert.equal(list.length, 2);
  assert.equal(list.filter(item => item.kind === 'note').length, 1);
  assert.equal(list[0].created_by_name, 'owner@fr.test');

  // 校验：未知 kind、空正文笔记、坏 URL 链接全部 400。
  await call(`/projects/${project}/research`, 'POST', owner.token, { kind: 'image', title: 'x' }, 400);
  await call(`/projects/${project}/research`, 'POST', owner.token, { kind: 'note', title: 'x', body: '' }, 400);
  await call(`/projects/${project}/research`, 'POST', owner.token, { kind: 'link', title: 'x', url: 'not-a-url' }, 400);

  // 权限：观察者只读，未授权不可见。
  await call(`/projects/${project}/research`, 'POST', viewer.token, { kind: 'note', title: 'x', body: 'y' }, 403);
  await call(`/projects/${project}/research`, 'GET', outsider.token, undefined, 403);

  // 修订：作者改自己的；不能改别人的；项目所有者可改任何人的。
  await call(`/projects/${project}/research/${link.id}`, 'PATCH', reviewer.token, { title: 'Magnum 展览页（更新）' });
  await call(`/projects/${project}/research/${note.id}`, 'PATCH', reviewer.token, { title: '越权' }, 403);
  await call(`/projects/${project}/research/${link.id}`, 'PATCH', owner.token, { source: 'Magnum Photos' });
  await call(`/projects/${project}/research/${randomUUID()}`, 'PATCH', owner.token, { title: 'x' }, 404);
  await call(`/projects/${project}/research/${link.id}`, 'PATCH', owner.token, {}, 400);

  // 删除：作者删自己的；缺失 404。
  await call(`/projects/${project}/research/${link.id}`, 'DELETE', reviewer.token);
  await call(`/projects/${project}/research/${link.id}`, 'DELETE', reviewer.token, undefined, 404);

  // 参考是辅助资料：不推进正式修订，不进入资产身份域。
  assert.equal((await call(`/projects/${project}/snapshot`)).revision, revisionBefore);
  assert.equal(Number((await pool.query('SELECT count(*)::int AS n FROM fr_source_assets')).rows[0].n), assetsBefore);
  assert.equal(Number((await pool.query('SELECT count(*)::int AS n FROM fr_research_references')).rows[0].n), 1);
});
