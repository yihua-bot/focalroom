import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { Pool, type PoolClient } from 'pg';
export const digest = (value: string | Buffer) => createHash('sha256').update(value).digest('hex');
export async function transaction<T>(pool: Pool, action: (tx: PoolClient) => Promise<T>): Promise<T> {
  const tx = await pool.connect();
  try { await tx.query('BEGIN'); const result = await action(tx); await tx.query('COMMIT'); return result; }
  catch (error) { await tx.query('ROLLBACK'); throw error; }
  finally { tx.release(); }
}
// 显式运行；服务启动不对已有数据库自动执行 DDL。
export async function migrate(pool: Pool) {
  const sql = await readFile(new URL('../migrations/0001_shared_authority.sql', import.meta.url), 'utf8');
  const hash = digest(sql);
  await transaction(pool, async tx => {
    await tx.query('SELECT pg_advisory_xact_lock(759362109)');
    const exists = await tx.query("SELECT to_regclass('public.fr_migrations') AS name");
    if (exists.rows[0].name) {
      const applied = await tx.query('SELECT sha256 FROM fr_migrations WHERE version=$1', ['0001']);
      if (applied.rows[0]) { if (applied.rows[0].sha256 !== hash) throw new Error('迁移校验和不匹配'); return; }
    }
    await tx.query(sql);
    await tx.query('INSERT INTO fr_migrations(version,sha256) VALUES($1,$2)', ['0001', hash]);
  });
}
