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

test('H0-04 board with real asset alias returns photo_id for image ticket flow', async () => {
  // 创建带真实 alias 的画布节点——验证 photo_id 从 JOIN 返回
  const created = await call('/boards', 'POST', owner, { projectId: project, title: '真实照片画布' });
  // 复用 before 中真实上传建立的映射，不能以手工插入掩盖非 UUID 照片映射失败。
  const alias = (await pool.query(
    'SELECT a.asset_id FROM fr_asset_aliases a JOIN fr_source_assets s ON s.id=a.asset_id WHERE a.project_id=$1 AND a.photo_id=$2 AND s.project_id=$1',
    [project, 'p1'],
  )).rows[0];
  assert.ok(alias, '预览上传应同时建立资产与原照片编号的映射');
  const assetId = alias.asset_id;
  // 在画布上放一个引用该资产的照片节点
  await pool.query(
    'INSERT INTO fr_canvas_nodes(id, board_id, node_type, x, y, width, height, z_order, asset_id) VALUES($1,$2,\'photo\',50,60,220,160,1,$3)',
    [randomUUID(), created.id, assetId],
  );
  // 放一个无 alias 的节点（占位符场景）
  const orphanAsset = randomUUID();
  await pool.query(
    'INSERT INTO fr_canvas_nodes(id, board_id, node_type, x, y, width, height, z_order, asset_id) VALUES($1,$2,\'photo\',300,60,220,160,2,$3)',
    [randomUUID(), created.id, orphanAsset],
  );

  const nodes = await call('/boards/' + created.id + '/nodes', 'GET', owner);
  assert.equal(nodes.length, 2);
  const withAlias = nodes.find(n => n.asset_id === assetId);
  const withoutAlias = nodes.find(n => n.asset_id === orphanAsset);
  assert.ok(withAlias, '有 alias 的节点应存在');
  assert.equal(withAlias.photo_id, 'p1', 'JOIN 应返回 photo_id=p1');
  assert.ok(withoutAlias, '无 alias 的节点也应存在');
  assert.equal(withoutAlias.photo_id, null, '无 alias 返回 null photo_id');

  // photo_id 应能走现有 preview ticket 流程
  const ticket = await call('/projects/' + project + '/previews/p1/ticket', 'POST', owner);
  assert.ok(ticket.url, '应获得预览票据 URL');

  // outsider 不能读节点
  await call('/boards/' + created.id + '/nodes', 'GET', outsider, undefined, 403);
});

test('H0-04 uuid-text cast does not error on legacy text asset ids', async () => {
  // 旧版本可能存了非 UUID 格式的 asset_id——JOIN 不应崩
  const created = await call('/boards', 'POST', owner, { projectId: project, title: '旧格式画布' });
  await pool.query(
    "INSERT INTO fr_canvas_nodes(id, board_id, node_type, x, y, width, height, z_order, asset_id) VALUES($1,$2,'photo',10,10,100,100,1,'legacy-text-id')",
    [randomUUID(), created.id],
  );
  const nodes = await call('/boards/' + created.id + '/nodes', 'GET', owner);
  assert.equal(nodes.length, 1);
  assert.equal(nodes[0].photo_id, null, '旧 TEXT 引用正确无匹配，不报错');
});

test('G6 formal canvas commands: envelope, revision conflict, idempotency, locked-node and read-only policy', async () => {
  const created = await call('/boards', 'POST', owner, { projectId: project, title: '命令画布' });
  const envelope = (baseRevision, actions, override = {}) => ({
    schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(),
    projectId: project, baseRevision, ...override,
    operations: actions.map(action => ({ opId: randomUUID(), action })),
  });
  const post = (body, status = 200) => call('/boards/' + created.id + '/commands', 'POST', owner, body, status);

  // 快乐路径：建节点+批量移动，修订 0→1，节点真实生效。
  const nodeId = randomUUID();
  const firstEnvelope = envelope(0, [
    { type: 'createNode', nodeId, nodeType: 'photo', x: 10, y: 20, width: 300, height: 200 },
    { type: 'moveNodes', moves: [{ nodeId, x: 40, y: 50 }] },
  ]);
  const first = await post(firstEnvelope);
  assert.equal(first.ok, true);
  assert.equal(first.newRevision, 1);
  assert.deepEqual(first.affectedNodeIds, [nodeId, nodeId]);
  const applied = (await call('/boards/' + created.id + '/nodes', 'GET', owner)).find(n => n.id === nodeId);
  assert.ok(applied, '命令创建的节点应存在');
  assert.equal(applied.x, 40);
  assert.equal(applied.y, 50);

  // 幂等重放：同 commandId+同内容 → 同回执，不重复推进修订。
  const replay = await post(firstEnvelope);
  assert.deepEqual(replay, first);

  // 同 commandId 换内容 → 幂等冲突。
  await post({ ...firstEnvelope, baseRevision: 1, operations: [{ opId: randomUUID(), action: { type: 'updateNode', nodeId, x: 1 } }] }, 409);

  // 过期 baseRevision → 修订冲突。
  await post(envelope(0, [{ type: 'deleteNode', nodeId }]), 409);

  // 锁定：可锁定；锁定后几何修改被拒；"只解锁"放行。
  await post(envelope(1, [{ type: 'updateNode', nodeId, locked: true }]));
  await post(envelope(2, [{ type: 'updateNode', nodeId, x: 99 }]), 409);
  const unlock = await post(envelope(2, [{ type: 'updateNode', nodeId, locked: false }]));
  assert.equal(unlock.newRevision, 3);

  // 只读角色不能发正式命令。
  await call('/boards/' + created.id + '/commands', 'POST', viewer, envelope(3, [{ type: 'deleteNode', nodeId }]), 403);

  // official 状态只读；恢复草稿后命令恢复。
  await pool.query("UPDATE fr_boards SET state='official' WHERE id=$1", [created.id]);
  await post(envelope(3, [{ type: 'deleteNode', nodeId }]), 409);
  await pool.query("UPDATE fr_boards SET state='draft' WHERE id=$1", [created.id]);

  // 跨画布节点引用被拒（节点不属于命令目标画布）。
  const other = await call('/boards', 'POST', owner, { projectId: project, title: '另一命令画布' });
  await call('/boards/' + other.id + '/commands', 'POST', owner, envelope(0, [{ type: 'updateNode', nodeId, x: 5 }]), 409);

  // 重复 opId 与空更新直接拒绝。
  const dupOpId = randomUUID();
  const dup = envelope(3, [{ type: 'updateNode', nodeId, x: 7 }, { type: 'updateNode', nodeId, y: 8 }]);
  dup.operations = dup.operations.map(op => ({ ...op, opId: dupOpId }));
  await post(dup, 400);
  await post(envelope(3, [{ type: 'updateNode', nodeId }]), 400);

  // 收尾：解锁后的节点可删除，修订继续推进。
  const last = await post(envelope(3, [{ type: 'deleteNode', nodeId }]));
  assert.equal(last.newRevision, 4);
  assert.equal((await call('/boards/' + created.id + '/nodes', 'GET', owner)).length, 0);
});

test('G6 canvas event log: commands append ordered events; cursor replays incrementally', async () => {
  const created = await call('/boards', 'POST', owner, { projectId: project, title: '事件画布' });
  const envelope = (baseRevision, actions) => ({
    schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(),
    projectId: project, baseRevision,
    operations: actions.map(action => ({ opId: randomUUID(), action })),
  });
  const post = body => call('/boards/' + created.id + '/commands', 'POST', owner, body);

  // 三条命令 + 一次幂等重放 → 恰好三个事件（重放不追加）。
  const nodeIds = [randomUUID(), randomUUID()];
  const firstEnv = envelope(0, [{ type: 'createNode', nodeId: nodeIds[0], nodeType: 'photo', x: 0, y: 0, width: 100, height: 100 }]);
  await post(firstEnv);
  await post(firstEnv);
  await post(envelope(1, [{ type: 'createNode', nodeId: nodeIds[1], nodeType: 'photo', x: 50, y: 50, width: 100, height: 100 }]));
  await post(envelope(2, [{ type: 'moveNodes', moves: [{ nodeId: nodeIds[0], x: 10, y: 10 }] }]));

  const all = await call('/boards/' + created.id + '/events?after=0&limit=100');
  assert.equal(all.events.length, 3);
  assert.equal(all.nextCursor, null, '取尽后无游标');
  assert.deepEqual(all.events.map(e => e.revision), [1, 2, 3]);
  assert.ok(all.events.every(e => e.command_id && e.kind === 'canvas.command'));
  assert.ok(all.events.every(e => typeof e.id === 'number'), 'id 应为数字（BIGSERIAL 映射）');
  assert.ok(all.events.every((e, i) => i === 0 || all.events[i - 1].id < e.id), '事件 id 严格递增');

  // 游标分页：after=第一条 → 只剩两条。
  const tail = await call('/boards/' + created.id + '/events?after=' + all.events[0].id + '&limit=100');
  assert.equal(tail.events.length, 2);
  assert.deepEqual(tail.events.map(e => e.revision), [2, 3]);

  // limit 截断给出续读游标；续读到空后游标为 null。
  const page = await call('/boards/' + created.id + '/events?after=0&limit=2');
  assert.equal(page.events.length, 2);
  assert.equal(page.nextCursor, page.events[1].id);
  const rest = await call('/boards/' + created.id + '/events?after=' + page.nextCursor + '&limit=2');
  assert.equal(rest.events.length, 1);
  assert.equal(rest.events[0].revision, 3);
  assert.equal(rest.nextCursor, null);

  // 只读角色可读事件；无关用户拒绝。
  const viewerView = await call('/boards/' + created.id + '/events', 'GET', viewer);
  assert.equal(viewerView.events.length, 3);
  await call('/boards/' + created.id + '/events', 'GET', outsider, undefined, 403);
});

test('G6 board lifecycle: state transitions are editor-gated formal commands', async () => {
  const created = await call('/boards', 'POST', owner, { projectId: project, title: '生命周期画布' });
  const envelope = (baseRevision, actions) => ({
    schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(),
    projectId: project, baseRevision,
    operations: actions.map(action => ({ opId: randomUUID(), action })),
  });
  const post = (body, status = 200) => call('/boards/' + created.id + '/commands', 'POST', owner, body, status);
  const transition = (baseRevision, target, extra = []) =>
    envelope(baseRevision, [{ type: 'transitionState', target }, ...extra]);

  // 评审者（可提案、无编辑权）不能转换状态。
  await call('/boards/' + created.id + '/commands', 'POST', reviewer, transition(0, 'official'), 403);

  // 混排（内容+转换）直接拒绝。
  await post(transition(0, 'official', [{ type: 'createNode', nodeType: 'photo', x: 0, y: 0, width: 10, height: 10 }]), 400);

  // 拥有者发布草稿：修订推进、事件追加。
  const publish = await post(transition(0, 'official'));
  assert.equal(publish.ok, true);
  assert.equal(publish.newRevision, 1);
  const events = await call('/boards/' + created.id + '/events?after=0&limit=10');
  assert.equal(events.events.length, 1);
  assert.equal(events.events[0].revision, 1);

  // 正式画布拒绝内容命令；只能冻结。
  await post(envelope(1, [{ type: 'createNode', nodeType: 'photo', x: 0, y: 0, width: 10, height: 10 }]), 409);
  const freeze = await post(transition(1, 'frozen'));
  assert.equal(freeze.newRevision, 2);

  // 冻结为终态：一切命令拒绝。
  await post(transition(2, 'official'), 409);
  await post(envelope(2, [{ type: 'createNode', nodeType: 'photo', x: 0, y: 0, width: 10, height: 10 }]), 409);
});
