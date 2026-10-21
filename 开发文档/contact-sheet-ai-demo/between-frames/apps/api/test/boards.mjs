import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

let pool, api, base, owner, reviewer, viewer, outsider, workspace, project, board, comment;
async function call(path, method = 'GET', account = owner, body, status = 200, headers) {
  const response = await fetch(base + '/api/v2' + path, {
    method,
    headers: { 'content-type': 'application/json', ...(headers ?? (account ? { authorization: 'Bearer ' + account.token } : {})) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const value = await response.json();
  assert.equal(response.status, status, JSON.stringify(value));
  return value;
}
async function invite(account, role) {
  const link = await call('/workspaces/' + workspace + '/invitations', 'POST', owner, {
    email: account.user.email, role, project_id: project,
  });
  await call('/invitations/accept', 'POST', account, { token: new URL(link.invite_url).hash.split('=')[1] });
}
before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须使用隔离 PostgreSQL runner');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  api = await createApi({ pool, origin: 'http://127.0.0.1:1424',
    store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'board-previews')) });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
  const register = email => call('/auth/register', 'POST', null, { email, password: 'board-test-password-123', display_name: email });
  owner = await register('board-owner@fr.test');
  reviewer = await register('board-reviewer@fr.test');
  viewer = await register('board-viewer@fr.test');
  outsider = await register('board-outsider@fr.test');
  workspace = (await call('/workspaces'))[0].id;
  const manifest = { workspace_id: workspace, local_project_id: 'board-fixture', promotion_id: randomUUID(),
    title: '画布权限项目', from_revision: 0,
    snapshot: { photos: [{ id: 'p1', filename: 'p1.jpg', source_sha256: 'a'.repeat(64), selection: 'UNDECIDED' }],
      sequences: [{ id: 'sequence', title: '主序列', items: [] }] },
    media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
  const promotion = await call('/promotions', 'POST', owner, manifest);
  project = promotion.project_id;
  await call('/boards?project=' + project, 'GET', owner, undefined, 409);
  const jpeg = await sharp({ create: { width: 96, height: 64, channels: 3, background: '#2398ba' } }).jpeg().toBuffer();
  const upload = await fetch(base + '/api/v2/projects/' + project + '/previews/p1', {
    method: 'PUT', headers: { authorization: 'Bearer ' + owner.token, 'content-type': 'image/jpeg' }, body: jpeg,
  });
  assert.equal(upload.status, 200);
  await call('/projects/' + project + '/finalize', 'POST', owner, { manifest_digest: promotion.manifest_digest });
  await invite(reviewer, 'REVIEWER');
  await invite(viewer, 'VIEWER');
});
after(async () => { await api?.close(); await pool?.end(); });

test('G6 board discovery and creation enforce project grants, not just authentication', async () => {
  await call('/boards?project=' + project, 'GET', null, undefined, 401);
  await call('/boards?project=' + project, 'GET', outsider, undefined, 403);
  await call('/boards', 'POST', outsider, { projectId: project, title: '越权' }, 403);
  await call('/boards', 'POST', viewer, { projectId: project, title: '只读' }, 403);
  board = await call('/boards', 'POST', owner, { projectId: project, title: '  叙事初稿  ' });
  assert.equal(board.title, '叙事初稿');
  assert.equal(board.state, 'draft');
  assert.deepEqual((await call('/boards?project=' + project, 'GET', viewer)).map(b => b.id), [board.id]);
  await call('/boards', 'POST', reviewer, { projectId: project, title: '评审草稿' });
  for (const value of [{ projectId: project, title: '' }, { projectId: project, title: 'x'.repeat(201) },
    { projectId: project, title: '绕过', state: 'official' }, { title: '缺少项目' }]) {
    await call('/boards', 'POST', owner, value, 400);
  }
  await call('/boards', 'GET', owner, undefined, 400);
});

test('G6 node and comment reads are scoped; hidden nodes and missing resources behave correctly', async () => {
  const nodeId = randomUUID();
  await pool.query("INSERT INTO fr_canvas_nodes(id,board_id,node_type,asset_id,rotation,locked,children_json) VALUES($1,$2,'photo','p1',15,true,'[]')", [nodeId, board.id]);
  await pool.query("INSERT INTO fr_canvas_nodes(board_id,node_type,visible) VALUES($1,'sticky',false)", [board.id]);
  const nodes = await call('/boards/' + board.id + '/nodes', 'GET', viewer);
  assert.equal(nodes.length, 1);
  assert.equal(nodes[0].id, nodeId);
  assert.equal(nodes[0].rotation, 15);
  assert.equal(nodes[0].locked, true);
  assert.deepEqual(nodes[0].children_json, []);
  for (const resource of ['nodes', 'annotations']) {
    await call('/boards/' + board.id + '/' + resource, 'GET', outsider, undefined, 403);
    await call('/boards/' + board.id + '/' + resource, 'GET', null, undefined, 401);
    await call('/boards/' + randomUUID() + '/' + resource, 'GET', owner, undefined, 404);
    await call('/boards/not-a-uuid/' + resource, 'GET', owner, undefined, 400);
  }
});

test('G6 comments validate input, require write permission and restrict resolution to author or editor', async () => {
  const input = { boardId: board.id, x: 10.5, y: -20, body: '  收紧开篇  ' };
  await call('/boards/annotations', 'POST', viewer, input, 403);
  await call('/boards/annotations', 'POST', outsider, input, 403);
  comment = (await call('/boards/annotations', 'POST', reviewer, input)).id;
  const comments = await call('/boards/' + board.id + '/annotations', 'GET', viewer);
  assert.equal(comments[0].body, '收紧开篇');
  assert.equal(comments[0].author_id, reviewer.user.id);
  const own = (await call('/boards/annotations', 'POST', owner, { ...input, body: '作者批注' })).id;
  await call('/boards/annotations/' + own + '/resolve', 'POST', reviewer, { resolved: true }, 403);
  await call('/boards/annotations/' + comment + '/resolve', 'POST', viewer, { resolved: true }, 403);
  await call('/boards/annotations/' + comment + '/resolve', 'POST', outsider, { resolved: true }, 403);
  assert.deepEqual(await call('/boards/annotations/' + comment + '/resolve', 'POST', reviewer, { resolved: true }), { id: comment, resolved: true });
  await call('/boards/annotations/' + comment + '/resolve', 'POST', owner, { resolved: false });
  for (const invalid of [{ ...input, x: null }, { ...input, y: 1e20 }, { ...input, body: ' ' },
    { ...input, body: 'x'.repeat(2001) }, { ...input, author_id: owner.user.id }]) {
    await call('/boards/annotations', 'POST', owner, invalid, 400);
  }
  for (const invalid of [{ resolved: 'false' }, {}, { resolved: true, boardId: board.id }]) {
    await call('/boards/annotations/' + comment + '/resolve', 'POST', owner, invalid, 400);
  }
  await call('/boards/annotations/' + randomUUID() + '/resolve', 'POST', owner, { resolved: true }, 404);
  const revision = (await call('/projects/' + project + '/snapshot')).revision;
  assert.equal(revision, 0, '草稿批注不能隐式修改正式作品');
});

test('G6 cookie mutations require CSRF; revoked grants stop existing board access immediately', async () => {
  const input = { boardId: board.id, x: 0, y: 0, body: 'cookie 评论' };
  await call('/boards/annotations', 'POST', owner, input, 403, { cookie: 'fr_session=' + owner.token });
  await call('/boards/annotations', 'POST', owner, input, 200,
    { cookie: 'fr_session=' + owner.token, origin: 'http://127.0.0.1:1424', 'x-focalroom-csrf': owner.csrf });
  await call('/projects/' + project + '/grants/' + reviewer.user.id, 'DELETE');
  await call('/boards?project=' + project, 'GET', reviewer, undefined, 403);
  await call('/boards/' + board.id + '/nodes', 'GET', reviewer, undefined, 403);
  await call('/boards/' + board.id + '/annotations', 'GET', reviewer, undefined, 403);
  await call('/boards/annotations', 'POST', reviewer, input, 403);
  await call('/boards/annotations/' + comment + '/resolve', 'POST', reviewer, { resolved: true }, 403);
  const denied = await pool.query('SELECT count(*)::int AS n FROM fr_acl_audit WHERE project_id=$1 AND user_id=$2 AND NOT allowed', [project, reviewer.user.id]);
  assert.ok(denied.rows[0].n >= 5);
});
