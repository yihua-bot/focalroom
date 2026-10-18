import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import pg from "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/node_modules/pg/lib/index.js";
import { migrate } from "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/database.ts";
import { AuthService,createSession } from "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/auth.ts";
assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);
const pool=new pg.Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});
try {
 await migrate(pool);
 const user={id:randomUUID(),email:"desktop-proof@fixture.test",display_name:"isolated fixture"};
 await pool.query("INSERT INTO fr_users(id,email,display_name,password_hash) VALUES($1,$2,$3,$4)",[user.id,user.email,user.display_name,"not-a-real-password"]);
 const session=await createSession(pool,user);
 const auth=new AuthService(pool,"https://team.focroom.cc");
 await assert.rejects(()=>auth.authenticate({authorization:`Bearer ${session.csrf}`},false),e=>e.status===401&&e.code==="UNAUTHORIZED");
 assert.equal((await auth.authenticate({authorization:`Bearer ${session.token}`},false)).id,user.id);
 console.log(JSON.stringify({test:"csrf_is_not_a_desktop_auth_token",csrf_bearer_status:401,real_session_bearer:"accepted",csrf_length:session.csrf.length,token_length:session.token.length,real_mail_or_model_calls:0,production_changes:false}));
} finally {await pool.end();}
