import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { Pool, type PoolClient } from "pg";
export const digest = (value: string | Buffer) =>
  createHash("sha256").update(value).digest("hex");
export async function transaction<T>(
  pool: Pool,
  action: (tx: PoolClient) => Promise<T>,
): Promise<T> {
  const tx = await pool.connect();
  try {
    await tx.query("BEGIN");
    const result = await action(tx);
    await tx.query("COMMIT");
    return result;
  } catch (error) {
    await tx.query("ROLLBACK");
    throw error;
  } finally {
    tx.release();
  }
}
async function migrations() {
  return Promise.all(
    ["0001_shared_authority.sql", "0002_audit_envelopes.sql"].map(
      async (name) => {
        const sql = await readFile(
          new URL("../migrations/" + name, import.meta.url),
          "utf8",
        );
        return { version: name.slice(0, 4), sql, sha256: digest(sql) };
      },
    ),
  );
}
// 显式运行；普通启动只核对版本，绝不自动 DDL。已登记迁移禁止修改字节。
export async function migrate(pool: Pool) {
  const versions = await migrations();
  await transaction(pool, async (tx) => {
    await tx.query("SELECT pg_advisory_xact_lock(759362109)");
    let exists = !!(
      await tx.query("SELECT to_regclass('public.fr_migrations') AS name")
    ).rows[0].name;
    if (exists) {
      const applied = (await tx.query("SELECT version FROM fr_migrations"))
        .rows;
      if (applied.some((v) => !versions.some((m) => m.version === v.version)))
        throw new Error("数据库包含未知迁移，拒绝以旧版本变更");
    }
    for (const migration of versions) {
      const old = exists
        ? (
            await tx.query(
              "SELECT sha256 FROM fr_migrations WHERE version=$1",
              [migration.version],
            )
          ).rows[0]
        : null;
      if (old) {
        if (old.sha256 !== migration.sha256)
          throw new Error("迁移校验和不匹配");
        continue;
      }
      await tx.query(migration.sql);
      exists = true;
      await tx.query(
        "INSERT INTO fr_migrations(version,sha256) VALUES($1,$2)",
        [migration.version, migration.sha256],
      );
    }
  });
}
export async function assertSchema(pool: Pool) {
  const expected = await migrations();
  try {
    const rows = (await pool.query("SELECT version,sha256 FROM fr_migrations"))
      .rows;
    if (
      rows.length !== expected.length ||
      rows.some(
        (row) =>
          !expected.some(
            (m) => m.version === row.version && m.sha256 === row.sha256,
          ),
      )
    )
      throw new Error("schema mismatch");
  } catch {
    throw new Error("共享数据库版本不匹配；请核对独立数据库并显式执行迁移");
  }
}
