import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash, randomUUID } from "node:crypto";
import pg from "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/node_modules/pg/lib/index.js";
import { migrate } from "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/database.ts";
const { Pool } = pg;
assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, "isolated runner required");
const admin = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
const password = randomUUID();
let migrator, app;
try {
  for (const file of ["0001_shared_authority.sql", "0002_audit_envelopes.sql"]) {
    const sql = await readFile(new URL(file, "file:///Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/migrations/"), "utf8");
    await admin.query(sql);
    await admin.query("INSERT INTO fr_migrations(version,sha256) VALUES($1,$2)", [file.slice(0,4), createHash("sha256").update(sql).digest("hex")]);
  }
  await admin.query("INSERT INTO fr_users(id,email,password_hash,display_name,created_at) VALUES ('preserved-user','fixture@fr.test','fixture-only','fixture',now())");
  await admin.query(`CREATE ROLE focalroom_migrator LOGIN PASSWORD '${password}'`);
  await admin.query(`CREATE ROLE focalroom_app LOGIN PASSWORD '${password}'`);
  await admin.query("GRANT USAGE,CREATE ON SCHEMA public TO focalroom_migrator");
  await admin.query("GRANT USAGE ON SCHEMA public TO focalroom_app");
  await admin.query("GRANT ALL ON ALL TABLES IN SCHEMA public TO focalroom_migrator");
  await admin.query("GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO focalroom_migrator");
  function rolePool(user) {
    const url = new URL(process.env.FOCALROOM_TEST_DATABASE_URL); url.username=user; url.password=password;
    return new Pool({ connectionString:url.href });
  }
  migrator=rolePool("focalroom_migrator");
  await assert.rejects(()=>migrate(migrator), e=>e.code==="42501" && /owner/.test(e.message));
  assert.equal((await admin.query("SELECT count(*)::int AS n FROM fr_migrations")).rows[0].n,2);
  console.log("PASS: grants alone cannot upgrade administrator-owned tables; failed migration rolls back");
  // 与本次生产核对匹配：只转移 public 内 fr_ 表/序列/函数，不 REASSIGN OWNED。
  await admin.query(`BEGIN;
    DO $$ DECLARE item record; BEGIN
      FOR item IN SELECT relname FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='r' AND left(relname,3)='fr_' LOOP
        EXECUTE format('ALTER TABLE public.%I OWNER TO focalroom_migrator',item.relname);
      END LOOP;
      FOR item IN SELECT relname FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='S' AND left(relname,3)='fr_' LOOP
        EXECUTE format('ALTER SEQUENCE public.%I OWNER TO focalroom_migrator',item.relname);
      END LOOP;
      FOR item IN SELECT oid::regprocedure AS signature FROM pg_proc WHERE pronamespace='public'::regnamespace AND left(proname,3)='fr_' LOOP
        EXECUTE format('ALTER FUNCTION %s OWNER TO focalroom_migrator',item.signature);
      END LOOP;
    END $$;
    COMMIT;`);
  await migrate(migrator); await migrate(migrator);
  assert.deepEqual((await migrator.query("SELECT version FROM fr_migrations ORDER BY version")).rows.map(x=>x.version),["0001","0002","0006","0008","0009","0010","0011","0012"]);
  assert.equal((await migrator.query("SELECT display_name FROM fr_users WHERE id='preserved-user'")).rows[0].display_name,"fixture");
  assert.equal((await migrator.query("SELECT attempts FROM fr_magic_login_codes LIMIT 0")).rowCount,0);
  app=rolePool("focalroom_app");
  assert.equal((await app.query("SELECT count(*)::int AS n FROM fr_users")).rows[0].n,1);
  await app.query("UPDATE fr_users SET display_name='app-dml-pass' WHERE id='preserved-user'");
  for (const sql of ["ALTER TABLE fr_users ADD COLUMN forbidden text","CREATE TABLE forbidden(id text)"]) {
    await assert.rejects(()=>app.query(sql),e=>e.code==="42501");
  }
  console.log("PASS: scoped ownership transfer enables role migration from 0001/0002 to eight versions, idempotently");
  console.log("PASS: old row retained, OTP schema present; app DML allowed, DDL refused");
} finally {
  await app?.end(); await migrator?.end(); await admin.end();
}
