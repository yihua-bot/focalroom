// M4 最小权限账户回归：角色分离创建、migrator 执行迁移、
// 应用角色仅 DML（DDL/建角色被拒）。由 apps/api/test/run.mjs 的隔离 PostgreSQL 启动。
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Pool } from 'pg';
import { migrate } from '../src/database.ts';

assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须由隔离 PostgreSQL runner 启动');
const admin = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
const APP_PASSWORD = 'app-role-pass-1';
const MIGRATOR_PASSWORD = 'migrator-role-pass-1';

test('M4 角色初始化（等价 postgres-init 脚本）与迁移角色执行迁移', async () => {
  // 与 ops/shared/postgres-init/01-create-roles.sh 相同语义：幂等创建 + 授权 + 默认权限。
  await admin.query(`DO $$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_migrator') THEN
      CREATE ROLE focalroom_migrator LOGIN PASSWORD '${MIGRATOR_PASSWORD}';
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
      CREATE ROLE focalroom_app LOGIN PASSWORD '${APP_PASSWORD}';
    END IF;
  END $$;`);
  await admin.query('GRANT CONNECT ON DATABASE postgres TO focalroom_migrator, focalroom_app');
  await admin.query('GRANT USAGE, CREATE ON SCHEMA public TO focalroom_migrator');
  await admin.query('GRANT USAGE ON SCHEMA public TO focalroom_app');
  await admin.query('ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO focalroom_migrator');
  await admin.query('ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO focalroom_migrator');
  await admin.query('ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO focalroom_app');
  await admin.query('ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO focalroom_app');

  const migrator = new Pool({
    connectionString: process.env.FOCALROOM_TEST_DATABASE_URL.replace(/\/\/[^@]*@/, `//focalroom_migrator:${MIGRATOR_PASSWORD}@`),
    max: 2,
  });
  await migrate(migrator);
  const versions = (await migrator.query('SELECT version FROM fr_migrations ORDER BY version')).rows.map((row) => row.version);
  assert.deepEqual(versions, ['0001', '0002', '0006', '0008', '0009', '0010', '0011', '0012', '0013', '0014', '0015', '0017', '0018', '0019', '0020', '0021', '0022', '0023']);
  // 幂等重放（0009 每次 migrate 都会重放授权）。
  await migrate(migrator);
  await migrator.end();
});

test('M4 应用角色只能 DML：读写通过，DDL 与建角色被拒', async () => {
  const app = new Pool({
    connectionString: process.env.FOCALROOM_TEST_DATABASE_URL.replace(/\/\/[^@]*@/, `//focalroom_app:${APP_PASSWORD}@`),
    max: 2,
  });
  // 读：迁移登记表。
  const read = await app.query('SELECT count(*)::int AS n FROM fr_migrations');
  assert.ok(read.rows[0].n >= 5);
  for (const privilege of ['SELECT','INSERT','UPDATE']) {
    assert.equal((await app.query('SELECT has_table_privilege(current_user,$1,$2) AS allowed',['fr_ai_jobs',privilege])).rows[0].allowed,true);
  }
  for (const privilege of ['DELETE','TRUNCATE']) {
    assert.equal((await app.query('SELECT has_table_privilege(current_user,$1,$2) AS allowed',['fr_ai_jobs',privilege])).rows[0].allowed,false);
  }
  for (const table of ['fr_source_assets', 'fr_asset_aliases', 'fr_edit_variants', 'fr_renditions', 'fr_boards', 'fr_canvas_nodes', 'fr_board_annotations', 'fr_author_hypotheses']) {
    for (const privilege of ['SELECT', 'INSERT', 'UPDATE', 'DELETE']) {
      assert.equal((await app.query('SELECT has_table_privilege(current_user,$1,$2) AS allowed', [table, privilege])).rows[0].allowed, true, `${table}: ${privilege}`);
    }
    assert.equal((await app.query("SELECT has_table_privilege(current_user,$1,'TRUNCATE') AS allowed", [table])).rows[0].allowed, false);
  }
  // 写：用户表 INSERT/UPDATE/DELETE 走全生命周期。
  const inserted = await app.query(
    "INSERT INTO fr_users(id,email,password_hash,display_name,created_at) VALUES('mp-1','mp@fr.test','x','最小权限回归',now()) RETURNING id",
  );
  assert.equal(inserted.rows[0].id, 'mp-1');
  await app.query("UPDATE fr_users SET display_name='已更新' WHERE id='mp-1'");
  await app.query("DELETE FROM fr_users WHERE id='mp-1'");
  // 拒绝：建表 / 删表 / 改表 / 建角色 / 授予超级权限。
  for (const [label, sql] of [
    ['CREATE TABLE', 'CREATE TABLE app_forbidden(id text)'],
    ['DROP TABLE', 'DROP TABLE fr_users'],
    ['ALTER TABLE', 'ALTER TABLE fr_users ADD COLUMN forbidden text'],
    ['CREATE ROLE', "CREATE ROLE escalation LOGIN"],
    ['ALTER ROLE SUPERUSER', 'ALTER ROLE focalroom_app SUPERUSER'],
  ]) {
    await assert.rejects(() => app.query(sql), (error) => {
      assert.match(error.code, /^42501$/, `${label} 应被权限拒绝，实际 ${error.code}: ${error.message}`);
      return true;
    }, `${label} 必须被拒绝`);
  }
  await app.end();
});
