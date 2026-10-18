import { test,before,after } from "node:test";
import assert from "node:assert/strict";
import { randomBytes,createHash } from "node:crypto";
import { join } from "node:path";
import { Pool } from "pg";
import { createApi } from "../src/server.ts";
import { migrate,digest } from "../src/database.ts";
import { LocalPreviewStore } from "../src/storage.ts";
let pool,api,base;
const origin="http://127.0.0.1:1424";
async function post(path,body,account,extra={}) {
 const response=await fetch(base+"/api/v2/auth/desktop/"+path,{method:"POST",headers:{"content-type":"application/json",origin,...(account?{cookie:`fr_session=${account.token}`,"x-focalroom-csrf":account.csrf}:{}),...extra},body:JSON.stringify(body)});
 return {status:response.status,body:await response.json()};
}
async function browser() {return api.service.auth.account({email:randomBytes(8).toString("hex")+"@fixture.test",password:"local-fixture-password",display_name:"Fixture"},true,randomBytes(8).toString("hex"));}
async function flow(account) {
 const state=randomBytes(32).toString("hex"),verifier=randomBytes(32).toString("hex"),challenge=createHash("sha256").update(verifier).digest("base64url");
 const start=await post("start",{state,code_challenge:challenge});assert.equal(start.status,200);
 const authorize={request_id:start.body.request_id,state};
 const approved=account?await post("authorize",authorize,account):null;
 return {start,authorize,exchange:{...authorize,code:approved?.body.code,code_verifier:verifier},state,verifier};
}
before(async()=>{assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);api=await createApi({pool,origin,store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,"desktop-previews"))});await api.app.listen(0,"127.0.0.1");base=await api.app.getUrl();});
after(async()=>{await api?.close();await pool?.end();});
test("desktop grant stores hashes, fixed return URL and real PKCE session; browser never receives bearer",async()=>{
 const account=await browser(), f=await flow(account), url=new URL(f.start.body.login_url);
 assert.equal(url.origin,origin);assert.equal(url.pathname,"/shared");assert.equal(url.searchParams.get("desktop"),"1");assert.equal(url.searchParams.get("state"),f.state);
 const row=(await pool.query("SELECT * FROM fr_desktop_logins WHERE id=$1",[f.authorize.request_id])).rows[0];
 assert.equal(row.state_hash,digest(f.state));assert.equal(row.code_hash,digest(f.exchange.code));assert.ok(!JSON.stringify(row).includes(f.verifier));assert.ok(!JSON.stringify(row).includes(f.exchange.code));
 const result=await post("exchange",f.exchange);assert.equal(result.status,200);assert.match(result.body.token,/^[a-f0-9]{64}$/);assert.notEqual(result.body.token,account.token);assert.notEqual(result.body.token,result.body.csrf);
 const identity=await api.service.auth.authenticate({authorization:`Bearer ${result.body.token}`},false);assert.equal(identity.id,account.user.id);
 assert.equal((await post("exchange",f.exchange)).status,400);
});
test("authorize requires browser authentication, exact state and CSRF; malformed/external redirect rejected",async()=>{
 const account=await browser(), f=await flow();
 assert.equal((await post("authorize",f.authorize)).status,401);
 assert.equal((await post("authorize",f.authorize,account,{"x-focalroom-csrf":"wrong"})).status,403);
 assert.equal((await post("authorize",{...f.authorize,state:"b".repeat(64)},account)).status,400);
 assert.equal((await post("start",{state:f.state,code_challenge:"a".repeat(43),redirect_uri:"https://evil.test"})).status,400);
 assert.equal((await post("authorize",f.authorize,account,{origin:"https://evil.test"})).status,403);
});
test("wrong verifier/state/code cannot redeem or consume the legitimate grant",async()=>{
 const f=await flow(await browser());
 for(const bad of [{code_verifier:"z".repeat(64)},{state:"c".repeat(64)},{code:"0".repeat(64)}])assert.equal((await post("exchange",{...f.exchange,...bad})).status,400);
 assert.equal((await post("exchange",f.exchange)).status,200);
});
test("concurrent exchanges create only one desktop session",async()=>{
 const account=await browser(), f=await flow(account);
 const values=await Promise.all(Array.from({length:8},()=>post("exchange",f.exchange)));assert.equal(values.filter(r=>r.status===200).length,1);assert.ok(values.every(r=>[200,400].includes(r.status)));
 assert.equal((await pool.query("SELECT count(*)::int AS n FROM fr_sessions WHERE user_id=$1",[account.user.id])).rows[0].n,2);
});
test("grant and authorization code expiry both prevent redemption",async()=>{
 for(const column of ["expires_at","code_expires_at"]){const f=await flow(await browser());await pool.query(`UPDATE fr_desktop_logins SET ${column}=now()-interval '1 second' WHERE id=$1`,[f.authorize.request_id]);assert.equal((await post("exchange",f.exchange)).status,400);}
});
test("browser revocation/password reset cannot yield a new desktop credential",async()=>{
 const account=await browser(), f=await flow(account);await pool.query("DELETE FROM fr_sessions WHERE digest=$1",[digest(account.token)]);assert.equal((await post("exchange",f.exchange)).status,400);
 const second=await browser(), g=await flow(second);await pool.query("UPDATE fr_sessions SET expires_at=now()-interval '1 second' WHERE digest=$1",[digest(second.token)]);assert.equal((await post("exchange",g.exchange)).status,401);
});
test("existing browser session may approve, retry replaces code and another account cannot take over",async()=>{
 const a=await browser(),b=await browser(),f=await flow(a);
 assert.equal((await post("authorize",f.authorize,b)).status,409);
 const fresh=await post("authorize",f.authorize,a);assert.equal(fresh.status,200);
 assert.equal((await post("exchange",f.exchange)).status,400);
 assert.equal((await post("exchange",{...f.exchange,code:fresh.body.code})).status,200);
});
test("session creation failure rolls back code consumption, then legitimate exchange can recover",async()=>{
 const f=await flow(await browser());
 await pool.query("CREATE FUNCTION desktop_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture'; END $$; CREATE TRIGGER desktop_fail BEFORE INSERT ON fr_sessions FOR EACH ROW EXECUTE FUNCTION desktop_fail()");
 try{assert.equal((await post("exchange",f.exchange)).status,500);}finally{await pool.query("DROP TRIGGER desktop_fail ON fr_sessions; DROP FUNCTION desktop_fail()");}
 assert.equal((await post("exchange",f.exchange)).status,200);
});
test("revocation and redemption share session-first lock ordering without a deadlock",async()=>{
 const account=await browser(),f=await flow(account),tx=await pool.connect();
 let response;
 try {
  await tx.query("BEGIN");await tx.query("SET LOCAL lock_timeout=\u00272s\u0027");
  await tx.query("SELECT digest FROM fr_sessions WHERE digest=$1 FOR UPDATE",[digest(account.token)]);
  response=post("exchange",f.exchange);
  await pool.query("SELECT pg_sleep(0.05)");
  await tx.query("DELETE FROM fr_sessions WHERE digest=$1",[digest(account.token)]);
  await tx.query("COMMIT");
 }catch(error){await tx.query("ROLLBACK");throw error;}finally{tx.release();}
 assert.equal((await response).status,400);
});
