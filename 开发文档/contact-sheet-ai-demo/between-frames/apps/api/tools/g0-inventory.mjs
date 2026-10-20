// G0 只盘点 runner 创建的空 PostgreSQL，禁止使用部署连接串。
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { Pool } from 'pg';
import { migrate } from '../src/database.ts';
assert.ok(process.env.FOCALROOM_TEST_DIRECTORY, '必须通过 apps/api/test/run.mjs 运行');
const url = new URL(process.env.FOCALROOM_TEST_DATABASE_URL);
assert.equal(url.hostname, '127.0.0.1');
const pool = new Pool({ connectionString: url.href });
try {
  await migrate(pool);
  const tables = (await pool.query("SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename")).rows.map(row => row.tablename);
  const migrations = (await pool.query('SELECT version,sha256 FROM fr_migrations ORDER BY version')).rows;
  const serverVersion = (await pool.query("SELECT current_setting('server_version') AS version")).rows[0].version;
  const result = { serverVersion, fixture: 'new isolated runner PostgreSQL; no production data', tables, migrations };
  const path = join(process.env.FOCALROOM_TEST_DIRECTORY, 'g0-schema.json');
  await writeFile(path, JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify({ inventory: path, tableCount: tables.length, migrationCount: migrations.length }));
} finally {
  await pool.end();
}
