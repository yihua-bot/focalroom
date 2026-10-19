import {test,before,after} from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {join} from 'node:path';
import {Pool} from 'pg';
import sharp from 'sharp';
import {createApi} from '../src/server.ts';
import {migrate,digest} from '../src/database.ts';
import {LocalPreviewStore} from '../src/storage.ts';
import {grantAiTokens,reconcileAiRequest,HOSTED_AI_POLICY} from '../src/hosted-ai.ts';

let pool,api,base,calls=0,mode='ok',seen;
const origin='http://127.0.0.1:1424',secret='fixture-server-secret-not-a-real-key';
const body=()=>({model:'deepseek-flash',input:[{role:'user',content:[{type:'input_text',text:'Build a fixture sequence'}]}],max_output_tokens:128,store:false});
function reply(){return Response.json({id:'fixture-response',status:'completed',output:[{type:'message',content:[{type:'output_text',text:'fixture'}]}],usage:{input_tokens:20,output_tokens:10,total_tokens:30}});}
async function transport(input,key){calls++;assert.equal(key,secret);seen=input;
 if(mode==='slow')await new Promise(resolve=>setTimeout(resolve,150));
 if(mode==='network')throw new Error(secret);
 if(mode==='reject')return new Response(secret,{status:401});
 if(mode==='server')return new Response(secret,{status:503});
 if(mode==='meter')return Response.json({status:'completed',output:[],usage:{total_tokens:30}});
 if(mode==='overflow')return Response.json({status:'completed',output:[],usage:{input_tokens:1,output_tokens:999999,total_tokens:1000000}});
 if(mode==='large')return new Response('x'.repeat(2*1024*1024+1));
 return reply();}
async function user(verified=true){const account=await api.service.auth.account({email:`${randomUUID()}@fixture.test`,password:'fixture-password',display_name:'Fixture'},true,randomUUID());
 if(verified)await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1',[account.user.id]);return account;}
const auth=u=>({authorization:`Bearer ${u.token}`,origin});
async function account(u){const r=await fetch(base+'/api/v2/ai/account',{headers:auth(u)});return {status:r.status,body:await r.json()};}
async function post(u,input=body(),id=randomUUID(),extra={}){const r=await fetch(base+'/api/v2/ai/deepseek-flash-v1/responses',{method:'POST',headers:{...auth(u),'content-type':'application/json','x-focalroom-ai-policy':HOSTED_AI_POLICY,'x-focalroom-request-id':id,...extra},body:JSON.stringify(input)});return {status:r.status,body:await r.json()};}
before(async()=>{assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);api=await createApi({pool,origin,store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'ai-previews')),hostedAi:{key:secret,initialTokens:25000},hostedAiTransport:transport});await api.app.listen(0,'127.0.0.1');base=await api.app.getUrl();});
after(async()=>{await api?.close();await pool?.end();});
test('hosted AI requires live account, verified email, correct policy and CSRF; key never returned',async()=>{
 mode='ok';const a=await user(false),start=calls;
 assert.equal((await account({token:'a'.repeat(64)})).status,401);
 assert.equal((await post(a)).status,403);assert.equal(calls,start);
 assert.equal((await account(a)).body.granted,0);
 await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1',[a.user.id]);
 const quota=(await account(a)).body;assert.equal(quota.granted,25000);assert.equal((await account(a)).body.granted,25000);
 assert.equal((await pool.query("SELECT count(*)::int n FROM fr_ai_grants WHERE user_id=$1 AND reason='initial'",[a.user.id])).rows[0].n,1);
 assert.equal((await post(a,body(),randomUUID(),{'x-focalroom-ai-policy':'another-service'})).status,409);
 assert.equal((await post(a,body(),randomUUID(),{authorization:'',cookie:`fr_session=${a.token}`})).status,403);
 const result=await post(a);assert.equal(result.status,200);assert.equal((await account(a)).body.spent,30);assert.equal((await account(a)).body.reserved,0);
 assert.ok(!JSON.stringify({quota,result}).includes(secret));
 await pool.query('DELETE FROM fr_sessions WHERE digest=$1',[digest(a.token)]);assert.equal((await post(a)).status,401);
});
test('quota and grants are account isolated and idempotent, cannot self-top-up over HTTP',async()=>{
 const a=await user(),b=await user();await account(a);await account(b);const id=randomUUID();
 assert.equal((await grantAiTokens(pool,a.user.id,id,1000,'fixture')).applied,true);
 assert.equal((await grantAiTokens(pool,a.user.id,id,1000,'fixture')).applied,false);
 await assert.rejects(grantAiTokens(pool,b.user.id,id,1000,'fixture'));
 assert.equal((await account(a)).body.granted,26000);assert.equal((await account(b)).body.granted,25000);
 const r=await fetch(base+'/api/v2/ai/grant',{method:'POST',headers:{...auth(a),'content-type':'application/json'},body:'{}'});assert.equal(r.status,404);
});
test('atomic reservation across two service instances rejects overspending and simultaneous duplicate IDs',async()=>{
 const a=await user();await account(a);await pool.query('UPDATE fr_ai_accounts SET granted=10000 WHERE user_id=$1',[a.user.id]);
 const {HostedAiService}=await import('../src/hosted-ai.ts');const peer=new HostedAiService(pool,api.service.auth,{key:secret,initialTokens:25000},transport);
 mode='slow';const start=calls,id=randomUUID();const headers={...auth(a),'x-focalroom-ai-policy':HOSTED_AI_POLICY,'x-focalroom-request-id':id};
 const results=await Promise.all([post(a,body(),id),peer.respond(headers,body()).then(body=>({status:200,body})).catch(e=>({status:e.status})),post(a)]);
 assert.equal(results.filter(r=>r.status===200).length,1);assert.equal(calls,start+1);assert.ok(results.some(r=>[402,409].includes(r.status)));
 const retry=await post(a,body(),id);assert.equal(retry.status,409);assert.equal(calls,start+1);
 mode='ok';
});
test('confirmed upstream rejection releases reservation; unknown network/meter/result holds it and never replays',async()=>{
 for(const failure of ['reject','network','server','meter','overflow','large']){
  const a=await user(),id=randomUUID();mode=failure;const start=calls,result=await post(a,body(),id);
  assert.equal(result.status,502);assert.ok(!JSON.stringify(result).includes(secret));
  const quota=(await account(a)).body;assert.equal(quota.spent,0);assert.equal(quota.reserved===0,failure==='reject');
  const row=(await pool.query('SELECT * FROM fr_ai_requests WHERE user_id=$1',[a.user.id])).rows[0];assert.equal(row.state,failure==='reject'?'REJECTED':'UNKNOWN');
  assert.equal((await post(a,body(),id)).status,409);assert.equal(calls,start+1);
 }
 mode='ok';
});
test('image forwarding requires consent and bounded local JPEG, strips metadata and blocks URLs/server tools',async()=>{
 const a=await user(),bytes=await sharp({create:{width:64,height:32,channels:3,background:'#369'}}).jpeg().withMetadata({orientation:1}).toBuffer();
 const image=url=>({...body(),input:[{role:'user',content:[{type:'input_image',image_url:url,detail:'high'}]}]});
 const data='data:image/jpeg;base64,'+bytes.toString('base64'),start=calls;
 assert.equal((await post(a,image(data))).status,403);
 for(const url of ['http://169.254.169.254/latest/meta-data','https://example.com/a.jpg','data:image/jpeg;base64,AAAA'])assert.equal((await post(a,image(url),randomUUID(),{'x-focalroom-image-consent':'granted'})).status,400);
 const big=await sharp({create:{width:1401,height:20,channels:3,background:'#fff'}}).jpeg().toBuffer();assert.equal((await post(a,image('data:image/jpeg;base64,'+big.toString('base64')),randomUUID(),{'x-focalroom-image-consent':'granted'})).status,400);
 assert.equal((await post(a,{...body(),tools:[{type:'web_search'}]})).status,400);assert.equal((await post(a,{...body(),previous_response_id:'other-user'})).status,400);
 assert.equal(calls,start);
 assert.equal((await post(a,image(data),randomUUID(),{'x-focalroom-image-consent':'granted'})).status,200);
 const part=seen.input[0].content[0];assert.equal(part.detail,'low');const meta=await sharp(Buffer.from(part.image_url.split(',')[1],'base64')).metadata();assert.equal(meta.exif,undefined);
});
test('disabled service is explicit and has no trial side effect; exhausted quota never calls provider',async()=>{
 const a=await user();await account(a);await pool.query('UPDATE fr_ai_accounts SET granted=0 WHERE user_id=$1',[a.user.id]);const start=calls;assert.equal((await post(a)).status,402);assert.equal(calls,start);
 const {HostedAiService}=await import('../src/hosted-ai.ts');const disabled=new HostedAiService(pool,api.service.auth);const b=await user();assert.equal((await disabled.account(auth(b))).enabled,false);
 await assert.rejects(disabled.respond(auth(b),body()),e=>e.status===503);assert.equal((await pool.query('SELECT count(*)::int n FROM fr_ai_accounts WHERE user_id=$1',[b.user.id])).rows[0].n,0);
});
test('operator reconciliation needs a confirmed reference, cannot release active calls, and is atomic/idempotent',async()=>{
 const a=await user(),id=randomUUID();mode='network';assert.equal((await post(a,body(),id)).status,502);mode='ok';
 const reserved=(await account(a)).body.reserved;assert.ok(reserved>0);
 await assert.rejects(reconcileAiRequest(pool,a.user.id,id,reserved+1,'fixture-bill'));
 assert.equal((await reconcileAiRequest(pool,a.user.id,id,12,'fixture-bill')).applied,true);
 assert.equal((await reconcileAiRequest(pool,a.user.id,id,12,'fixture-bill')).applied,false);
 await assert.rejects(reconcileAiRequest(pool,a.user.id,id,0,'other-bill'));
 const quota=(await account(a)).body;assert.equal(quota.reserved,0);assert.equal(quota.spent,12);
 const active=randomUUID();await pool.query("INSERT INTO fr_ai_requests(user_id,request_id,body_hash,policy,reserved,state) VALUES($1,$2,'fixture',$3,100,'PENDING')",[a.user.id,active,HOSTED_AI_POLICY]);await pool.query('UPDATE fr_ai_accounts SET reserved=reserved+100 WHERE user_id=$1',[a.user.id]);
 await assert.rejects(reconcileAiRequest(pool,a.user.id,active,0,'fixture-no-charge'));
 await pool.query("UPDATE fr_ai_requests SET created_at=now()-interval '11 minutes' WHERE user_id=$1 AND request_id=$2",[a.user.id,active]);
 await reconcileAiRequest(pool,a.user.id,active,0,'fixture-no-charge');assert.equal((await account(a)).body.reserved,0);
});
