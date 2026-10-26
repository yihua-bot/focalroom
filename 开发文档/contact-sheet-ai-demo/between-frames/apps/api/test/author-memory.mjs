import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

let pool, api, base, owner, other, editor, workspace, otherWorkspace, projectA1, projectA2, projectB;

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

async function addProject(workspaceId, ownerId, title, promotionKey) {
  const id = randomUUID();
  await pool.query(
    "INSERT INTO fr_projects(id, workspace_id, owner_id, title, state, revision, snapshot, promotion_key, manifest_digest, initial_revision) VALUES($1,$2,$3,$4,'SHARED',0,'{}',$5,$6,0)",
    [id, workspaceId, ownerId, title, promotionKey, randomUUID()],
  );
  return id;
}

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须使用隔离 PostgreSQL runner');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  api = await createApi({ pool, origin: 'http://127.0.0.1:1424',
    store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'author-memory-previews')) });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
  const register = email => call('/auth/register', 'POST', null, { email, password: 'author-memory-password-123', display_name: email });
  owner = await register('memory-owner@fr.test');
  other = await register('memory-other@fr.test');
  editor = await register('memory-editor@fr.test');
  workspace = (await call('/workspaces'))[0].id;
  otherWorkspace = (await call('/workspaces', 'GET', other))[0].id;
  projectA1 = await addProject(workspace, owner.user.id, '作者项目一', 'mem-a1');
  projectA2 = await addProject(workspace, owner.user.id, '作者项目二', 'mem-a2');
  projectB = await addProject(otherWorkspace, other.user.id, '他人项目', 'mem-b1');
  await pool.query("INSERT INTO fr_grants(project_id, user_id, role) VALUES($1,$2,'EDITOR')", [projectA1, editor.user.id]);
});

after(async () => { await api?.close(); await pool?.end(); });

test('作者记忆读取要求 Creator：匿名 401，项目受邀编辑与无关用户 403', async () => {
  await call('/author-memory?project=' + projectA1, 'GET', null, undefined, 401);
  await call('/author-memory?project=' + projectA1, 'GET', editor, undefined, 403);
  await call('/author-memory?project=' + projectA1, 'GET', other, undefined, 403);
  await call('/author-memory?project=' + randomUUID(), 'GET', owner, undefined, 404);
  await call('/author-memory', 'GET', owner, undefined, 400);
});

test('Creator 跨项目只读：排除当前项目，过滤 retired 与停用条目，跨 Creator 不可见', async () => {
  const seed = async (projectId, statement, status = 'confirmed', disabled = false) => {
    await pool.query(
      "INSERT INTO fr_author_hypotheses(id, project_id, statement, status, confidence, disabled_for_retrieval) VALUES($1,$2,$3,$4,'moderate',$5)",
      [randomUUID(), projectId, statement, status, disabled],
    );
  };
  await seed(projectA2, '偏好低饱和冷色调');
  await seed(projectA2, '已退役假设', 'retired');
  await seed(projectA2, '检索停用假设', 'confirmed', true);
  await seed(projectA1, '当前项目假设');
  await seed(projectB, '他人假设');
  const rows = await call('/author-memory?project=' + projectA1, 'GET', owner);
  assert.deepEqual(rows.map(r => r.statement), ['偏好低饱和冷色调']);
  assert.equal(rows[0].project_id, projectA2);
  assert.equal(rows[0].project_title, '作者项目二');
  assert.deepEqual(await call('/author-memory?project=' + projectB, 'GET', other), []);
});

test('写入仅限 Creator，按 (project, statement) 幂等，非法输入与不存在项目被拒', async () => {
  await call('/author-memory', 'POST', editor, { projectId: projectA1, statement: '编辑代写', status: 'confirmed', confidence: 'moderate' }, 403);
  await call('/author-memory', 'POST', other, { projectId: projectA1, statement: '他人代写', status: 'confirmed', confidence: 'moderate' }, 403);
  const first = await call('/author-memory', 'POST', owner, { projectId: projectA2, statement: '新假设', status: 'unconfirmed', confidence: 'tentative' });
  const second = await call('/author-memory', 'POST', owner, { projectId: projectA2, statement: '新假设', status: 'confirmed', confidence: 'moderate' });
  assert.equal(first.id, second.id);
  const row = (await pool.query('SELECT status, confidence FROM fr_author_hypotheses WHERE id=$1', [first.id])).rows[0];
  assert.equal(row.status, 'confirmed');
  assert.equal(row.confidence, 'moderate');
  const visible = await call('/author-memory?project=' + projectA1, 'GET', owner);
  assert.deepEqual(visible.map(r => r.statement).sort(), ['偏好低饱和冷色调', '新假设']);
  for (const invalid of [
    { projectId: projectA2, statement: '', status: 'confirmed', confidence: 'moderate' },
    { projectId: projectA2, statement: 'x'.repeat(1001), status: 'confirmed', confidence: 'moderate' },
    { projectId: projectA2, statement: 'x', status: 'bogus', confidence: 'moderate' },
    { projectId: projectA2, statement: 'x', status: 'confirmed', confidence: 'bogus' },
    { projectId: projectA2, statement: 'x', status: 'confirmed' },
    { projectId: projectA2, statement: 'x', status: 'confirmed', confidence: 'moderate', extra: 1 },
  ]) {
    await call('/author-memory', 'POST', owner, invalid, 400);
  }
  await call('/author-memory', 'POST', owner, { projectId: randomUUID(), statement: 'x', status: 'confirmed', confidence: 'moderate' }, 404);
});
