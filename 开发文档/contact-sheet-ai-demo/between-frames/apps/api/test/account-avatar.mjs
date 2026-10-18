import {test,before,after} from "node:test";
import assert from "node:assert/strict";
import {Pool} from "pg";
import {randomUUID} from "node:crypto";
import {join} from "node:path";
import {createApi} from "../src/server.ts";
import {migrate} from "../src/database.ts";
import {LocalPreviewStore} from "../src/storage.ts";
import {loginWithGoogle} from "../src/google-login.ts";
let pool,api,base,mailCode;
const originalFetch=globalThis.fetch,oldClient=process.env.FOCALROOM_GOOGLE_CLIENT_ID;
const client="isolated-client",email=`avatar-${randomUUID()}@gmail.com`,picture="https://lh3.googleusercontent.com/a/fixture";
let aud=client,verified="true",avatar=picture;
before(async()=>{
 assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);
 pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);
 api=await createApi({pool,origin:"http://127.0.0.1:1436",store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,"avatar-previews")),magicEmail:async(_email,code)=>{mailCode=code;return true;}});await api.app.listen(0,"127.0.0.1");base=await api.app.getUrl();
 process.env.FOCALROOM_GOOGLE_CLIENT_ID=client;
 globalThis.fetch=async(url,options)=>{
  if(String(url).startsWith("https://oauth2.googleapis.com/tokeninfo?"))return Response.json({aud,email,email_verified:verified});
  if(String(url)==="https://www.googleapis.com/oauth2/v3/userinfo")return Response.json({name:"Fixture",picture:avatar});
  return originalFetch(url,options);
 };
});
after(async()=>{globalThis.fetch=originalFetch;if(oldClient===undefined)delete process.env.FOCALROOM_GOOGLE_CLIENT_ID;else process.env.FOCALROOM_GOOGLE_CLIENT_ID=oldClient;await api?.close();await pool?.end();});
test("Google verified profile persists and is returned immediately, by /me and subsequent OTP session",async()=>{
 const account=await loginWithGoogle(pool,{access_token:"fixture-provider-access-token"},"avatar-fixture");assert.equal(account.user.avatar,picture);
 assert.equal((await pool.query("SELECT avatar_url FROM fr_users WHERE id=$1",[account.user.id])).rows[0].avatar_url,picture);
 const me=await originalFetch(base+"/api/v2/auth/me",{headers:{authorization:`Bearer ${account.token}`}});assert.equal(me.status,200);assert.equal((await me.json()).avatar,picture);
 const request=await originalFetch(base+"/api/v2/auth/magic/request",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({email})});assert.equal(request.status,200);
 const otp=await originalFetch(base+"/api/v2/auth/magic/verify",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({email,code:mailCode})});assert.equal(otp.status,200);assert.equal((await otp.json()).user.avatar,picture);
 const again=await loginWithGoogle(pool,{access_token:"fixture-provider-access-token"},"avatar-fixture");assert.equal(again.user.id,account.user.id);assert.equal(again.user.avatar,picture);
});
test("wrong audience, unverified email or self-asserted profile cannot create an identity",async()=>{
 const before=(await pool.query("SELECT count(*)::int AS n FROM fr_users WHERE email=ANY($1::text[])",[[email,"forged@fixture.test"]])).rows[0].n;
 aud="foreign-client";await assert.rejects(loginWithGoogle(pool,{access_token:"fixture-provider-access-token"},"avatar-fixture"),e=>e.code==="GOOGLE_AUD_MISMATCH");aud=client;
 verified="false";await assert.rejects(loginWithGoogle(pool,{access_token:"fixture-provider-access-token"},"avatar-fixture"),e=>e.code==="GOOGLE_EMAIL_UNVERIFIED");verified="true";
 await assert.rejects(loginWithGoogle(pool,{access_token:"fixture-provider-access-token",email:"forged@fixture.test",avatar:"https://evil.test/pixel"},"avatar-fixture"));
 assert.equal((await pool.query("SELECT count(*)::int AS n FROM fr_users WHERE email=ANY($1::text[])",[[email,"forged@fixture.test"]])).rows[0].n,before);
});
test("unsupported avatar does not replace the saved trusted image; OTP-only accounts remain null",async()=>{
 avatar="https://evil.test/tracking";const account=await loginWithGoogle(pool,{access_token:"fixture-provider-access-token"},"avatar-fixture");assert.equal(account.user.avatar,picture);
 const identity=await api.service.auth.authenticate({authorization:`Bearer ${account.token}`},false);assert.equal(identity.avatar_url,picture);
 const local=await api.service.auth.account({email:`empty-${randomUUID()}@fixture.test`,display_name:"Local",password:"isolated-avatar-password"},true,"avatar-password-fixture");assert.equal(local.user.avatar,null);
});
