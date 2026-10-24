// 隔离 PostgreSQL + 假模型传输：不访问生产、不使用真实照片或密钥。
import {test,before,after} from 'node:test';
import assert from 'node:assert/strict';
import {randomBytes,randomUUID} from 'node:crypto';
import {join} from 'node:path';
import {Pool} from 'pg';
import sharp from 'sharp';
import {createApi} from '../src/server.ts';
import {migrate,digest} from '../src/database.ts';
import {LocalPreviewStore} from '../src/storage.ts';
import {HostedAiService,HOSTED_AI_POLICY,reconcileAiRequest} from '../src/hosted-ai.ts';
import {QUEUE_CONSENT,claimQueued,expireQueued,validateQueueConfig} from '../src/hosted-ai-queue.ts';

let pool,api,base,calls=0,seen,mode='ok',release;
const workers=[];
const origin='http://127.0.0.1:1424',endpoint='/api/v2/ai/deepseek-flash-v1/responses/async';
const key=randomBytes(32).toString('base64'),secret='fixture-provider-secret';
const queue={activeKeyId:'first',keys:{first:key}},config={key:secret,initialTokens:1000000,queue};
const body=()=>({model:'deepseek-flash',input:[{role:'user',content:[{type:'input_text',text:'private-fixture-text'}]}],max_output_tokens:128,store:false});
const auth=u=>({authorization:`Bearer ${u.token}`,origin});
const headers=(u,id,durable=true)=>({...auth(u),'content-type':'application/json','x-focalroom-ai-policy':HOSTED_AI_POLICY,'x-focalroom-request-id':id,...(durable?{'x-focalroom-recovery-consent':QUEUE_CONSENT}:{})});
async function transport(input,key){
 assert.equal(key,secret);calls++;seen=input;
 if(mode==='blocked')await new Promise(resolve=>{release=resolve;});
 if(mode==='network')throw new Error(secret);
 if(mode==='meter')return Response.json({status:'completed',output:[],usage:{total_tokens:30}});
 return Response.json({id:'fixture-response',status:'completed',output:[{type:'message',content:[{type:'output_text',text:'fixture-result'}]}],usage:{input_tokens:20,output_tokens:10,total_tokens:30}});
}
async function request(path,options){const r=await fetch(base+path,options);return {status:r.status,body:await r.json()};}
const submit=(u,id=randomUUID(),input=body(),extra={})=>request(endpoint,{method:'POST',headers:{...headers(u,id),...extra},body:JSON.stringify(input)});
const poll=(u,id)=>request(endpoint+'/'+id,{headers:auth(u)});
const cancel=(u,id,extra={})=>request(endpoint+'/'+id+'/cancel',{method:'POST',headers:{...auth(u),...extra}});
const quota=async u=> (await request('/api/v2/ai/account',{headers:auth(u)})).body;
async function user(){const u=await api.service.auth.account({email:`${randomUUID()}@fixture.test`,password:'fixture-password',display_name:'Fixture'},true,randomUUID());await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1',[u.user.id]);await quota(u);return u;}
const job=async (u,id)=>(await pool.query('SELECT * FROM fr_ai_jobs WHERE user_id=$1 AND request_id=$2',[u.user.id,id])).rows[0];
async function until(fn){for(let i=0;i<100;i++){if(await fn())return;await new Promise(r=>setTimeout(r,10));}assert.fail('fixture did not finish');}
async function worker(next=config){const service=new HostedAiService(pool,api.service.auth,next,transport);workers.push(service);return service;}
before(async()=>{
 assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);
 pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);
 api=await createApi({pool,origin,store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'queue-previews')),hostedAi:config,hostedAiTransport:transport});
 // 生产者模拟已停的工作进程；提交只入库，替代服务再领取。
 await api.service.ai.drain();await api.app.listen(0,'127.0.0.1');base=await api.app.getUrl();
});
after(async()=>{release?.();for(const w of workers)await w.drain();await api?.close();await pool?.end();});

// 每项结束显式停止替代 worker，避免后续队列由前项残留消费者领取。
async function stop(w){await w.drain();}

test('queue is opt-in, authenticated and unavailable when no validated encryption key is configured',async()=>{
 const u=await user(),start=calls;
 const disabled=await worker({key:secret,initialTokens:0});
 await assert.rejects(disabled.submit(headers(u,randomUUID()),body()),e=>e.code==='AI_QUEUE_NOT_AVAILABLE');
 await assert.rejects(disabled.submit({...headers(u,randomUUID()),'x-focalroom-recovery-consent':'other'},body()),e=>e.code==='AI_QUEUE_CONSENT_INVALID');
 assert.equal(calls,start);
 assert.equal((await cancel({token:'f'.repeat(64)},randomUUID())).status,401);
 assert.equal((await quota(u)).recovery.policy,QUEUE_CONSENT);
 assert.equal((await quota(u)).recovery.maxQueueSeconds,600);
 assert.throws(()=>validateQueueConfig({activeKeyId:'a',keys:{a:'not-a-key'}}));
 assert.throws(()=>validateQueueConfig({activeKeyId:'missing',keys:{first:key}}));
 assert.throws(()=>validateQueueConfig({activeKeyId:'toString',keys:{}}));
 await stop(disabled);
});

test('body and preview are encrypted at rest, without credentials, and resume on service replacement',async()=>{
 const u=await user(),id=randomUUID(),start=calls;
 const pixels=await sharp({create:{width:32,height:24,channels:3,background:'#369'}}).jpeg().withMetadata().toBuffer();
 const input={...body(),input:[{role:'user',content:[{type:'input_text',text:'private-fixture-text'},{type:'input_image',image_url:'data:image/jpeg;base64,'+pixels.toString('base64')}]}]};
 const r=await submit(u,id,input,{'x-focalroom-image-consent':'granted'});assert.equal(r.status,202);assert.equal(calls,start);
 const stored=await job(u,id);assert.equal(stored.phase,'QUEUED');assert.equal(stored.attempts,0);assert.equal(stored.session_digest,digest(u.token));
 assert.ok(Buffer.isBuffer(stored.payload));
 for(const text of ['private-fixture-text','data:image/jpeg',u.token,secret,key])assert.equal(stored.payload.includes(Buffer.from(text)),false);
 assert.ok((await quota(u)).reserved>0);
 const w=await worker();await w.runQueueOnce();await until(async()=> (await poll(u,id)).body.state==='COMPLETED');
 const done=await job(u,id);assert.equal(done.phase,'DONE');assert.equal(done.payload,null);assert.equal(done.attempts,1);assert.equal(calls,start+1);
 assert.equal(seen.input[0].content[0].text,'private-fixture-text');assert.deepEqual(seen.reasoning,{effort:'none'});
 assert.equal((await sharp(Buffer.from(seen.input[0].content[1].image_url.split(',')[1],'base64')).metadata()).exif,undefined);
 assert.equal((await quota(u)).spent,30);assert.equal((await quota(u)).reserved,0);await stop(w);
});

test('two consumers and duplicate submissions dispatch once and clear input before model waiting',async()=>{
 const u=await user(),id=randomUUID(),start=calls;mode='blocked';
 const a=await worker(),b=await worker();
 try{
  await submit(u,id);await Promise.all([a.runQueueOnce(),b.runQueueOnce()]);await until(()=>calls===start+1);
  const row=await job(u,id);assert.equal(row.phase,'DISPATCHING');assert.equal(row.payload,null);assert.equal(row.attempts,1);
  assert.equal((await submit(u,id)).status,202);assert.equal(calls,start+1);
  assert.equal((await submit(u,id,{...body(),instructions:'changed'})).status,409);
  await assert.rejects(reconcileAiRequest(pool,u.user.id,id,0,'premature'),e=>e.code==='AI_REQUEST_NOT_RECONCILABLE');
  const noCancel=await cancel(u,id);assert.equal(noCancel.status,200);assert.equal(noCancel.body.cancelled,false);
  release();await until(async()=> (await poll(u,id)).body.state==='COMPLETED');assert.equal(calls,start+1);
 }finally{release?.();mode='ok';await stop(a);await stop(b);}
});

test('queued cancellation is account-scoped, CSRF protected, idempotent and releases only unsent usage',async()=>{
 const u=await user(),other=await user(),id=randomUUID(),start=calls;await submit(u,id);
 assert.equal((await cancel(other,id)).status,404);
 assert.equal((await cancel(u,id,{authorization:'',cookie:`fr_session=${u.token}`})).status,403);
 assert.equal((await cancel(u,id)).body.cancelled,true);
 assert.equal((await cancel(u,id)).body.cancelled,true);
 assert.equal((await poll(u,id)).body.state,'REJECTED');assert.equal((await quota(u)).reserved,0);assert.equal((await job(u,id)).payload,null);
 const w=await worker();await w.runQueueOnce();assert.equal(calls,start);await stop(w);
});

test('logout and email revocation before dispatch reject the job without forwarding or retaining the body',async()=>{
 const start=calls;
 for(const revocation of ['session','email']){
  const u=await user(),id=randomUUID();await submit(u,id);
  if(revocation==='session')await pool.query('DELETE FROM fr_sessions WHERE digest=$1',[digest(u.token)]);
  else await pool.query('UPDATE fr_users SET email_verified_at=NULL WHERE id=$1',[u.user.id]);
  const w=await worker();await w.runQueueOnce();await stop(w);
  const r=(await pool.query('SELECT state,error_code FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2',[u.user.id,id])).rows[0];
  assert.equal(r.state,'REJECTED');assert.equal(r.error_code,'AI_QUEUE_PERMISSION_REVOKED');assert.equal((await job(u,id)).payload,null);
  assert.equal((await pool.query('SELECT reserved FROM fr_ai_accounts WHERE user_id=$1',[u.user.id])).rows[0].reserved,'0');
 }
 assert.equal(calls,start);
});

test('queue expiry purges encrypted input and returns reserved tokens without treating dispatched jobs as unsent',async()=>{
 const u=await user(),id=randomUUID(),start=calls;await submit(u,id);
 await pool.query("UPDATE fr_ai_jobs SET created_at=now()-interval '11 minutes',expires_at=now()-interval '1 minute' WHERE user_id=$1 AND request_id=$2",[u.user.id,id]);
 await expireQueued(pool);assert.equal((await poll(u,id)).body.error,'AI_QUEUE_EXPIRED');assert.equal((await quota(u)).reserved,0);assert.equal((await job(u,id)).payload,null);
 const another=randomUUID();await submit(u,another);
 const claimed=await claimQueued(pool,queue,HOSTED_AI_POLICY,v=>v);assert.equal(claimed.requestId,another);
 await pool.query("UPDATE fr_ai_jobs SET dispatched_at=now()-interval '7 minutes' WHERE user_id=$1 AND request_id=$2",[u.user.id,another]);
 await expireQueued(pool);assert.equal((await poll(u,another)).body.state,'UNKNOWN');assert.ok((await quota(u)).reserved>0);
 const w=await worker();await w.runQueueOnce();assert.equal(calls,start);await stop(w);
});

test('lost or corrupted encryption keys and swapped ciphertext never fall back to plaintext or call the model',async()=>{
 const start=calls;
 for(const failure of ['corrupt','wrong-key','swap']){
  const u=await user(),id=randomUUID();await submit(u,id);
  if(failure==='corrupt'){
   const blob=(await job(u,id)).payload;blob[blob.length-1]^=1;
   await pool.query('UPDATE fr_ai_jobs SET payload=$3 WHERE user_id=$1 AND request_id=$2',[u.user.id,id,blob]);
  }
  if(failure==='swap'){
   const b=await user(),second=randomUUID();await submit(b,second);
   const one=(await job(u,id)).payload,two=(await job(b,second)).payload;
   await pool.query('UPDATE fr_ai_jobs SET payload=$3 WHERE user_id=$1 AND request_id=$2',[u.user.id,id,two]);
   await pool.query('UPDATE fr_ai_jobs SET payload=$3 WHERE user_id=$1 AND request_id=$2',[b.user.id,second,one]);
  }
  const w=await worker(failure==='wrong-key'?{...config,queue:{activeKeyId:'other',keys:{other:randomBytes(32).toString('base64')}}}:config);
  await w.runQueueOnce();await until(async()=> (await job(u,id)).phase==='DONE');await w.runQueueOnce();await stop(w);
  assert.equal((await poll(u,id)).body.error,'AI_QUEUE_INPUT_UNAVAILABLE');assert.equal((await quota(u)).reserved,0);
 }
 assert.equal(calls,start);
});

test('key rotation retains old decryption key while using the new key for subsequent requests',async()=>{
 const u=await user(),id=randomUUID(),start=calls;await submit(u,id);
 const rotated={...config,queue:{activeKeyId:'second',keys:{first:key,second:randomBytes(32).toString('base64')}}};
 const w=await worker(rotated);await w.runQueueOnce();await until(async()=> (await poll(u,id)).body.state==='COMPLETED');
 assert.equal(calls,start+1);await stop(w);
 const next=randomUUID();await w.submit(headers(u,next),body());assert.equal((await job(u,next)).key_id,'second');
 await cancel(u,next);
});

test('enqueue and account reservation roll back together on database failure',async()=>{
 const u=await user(),id=randomUUID(),before=await quota(u),start=calls;
 await pool.query("CREATE FUNCTION fail_ai_queue_fixture() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture queue failure'; END $$; CREATE TRIGGER fail_ai_queue_fixture BEFORE INSERT ON fr_ai_jobs FOR EACH ROW EXECUTE FUNCTION fail_ai_queue_fixture()");
 try{assert.equal((await submit(u,id)).status,500);}finally{await pool.query('DROP TRIGGER fail_ai_queue_fixture ON fr_ai_jobs; DROP FUNCTION fail_ai_queue_fixture()');}
 assert.equal((await quota(u)).reserved,before.reserved);assert.equal((await job(u,id)),undefined);
 assert.equal((await pool.query('SELECT count(*)::int n FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2',[u.user.id,id])).rows[0].n,0);assert.equal(calls,start);
});

test('unknown transport or invalid usage never re-enters the durable queue or releases reserved usage',async()=>{
 for(const failure of ['network','meter']){
  const u=await user(),id=randomUUID(),start=calls;mode=failure;await submit(u,id);
  const w=await worker();await w.runQueueOnce();await until(async()=> (await poll(u,id)).body.state==='UNKNOWN');
  assert.equal((await job(u,id)).phase,'UNKNOWN');assert.equal((await job(u,id)).payload,null);assert.ok((await quota(u)).reserved>0);
  await w.runQueueOnce();await submit(u,id);assert.equal(calls,start+1);await stop(w);
 }
 mode='ok';
});

test('independent worker processes recover a queued job, but never replay after a killed dispatch',async()=>{
 const {fork}=await import('node:child_process');const {once}=await import('node:events');const {createServer}=await import('node:http');
 let count=0,hold=false;
 const sockets=new Set();
 const server=createServer((request,response)=>{
  count++;request.resume();
  if(!hold)response.writeHead(200,{'content-type':'application/json'}).end(JSON.stringify({status:'completed',output:[],usage:{input_tokens:20,output_tokens:10,total_tokens:30}}));
 });
 server.on('connection',socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket));});
 server.listen(0,'127.0.0.1');await once(server,'listening');
 const children=[];
 async function start(){
  const child=fork(new URL('./fixtures/ai-queue-process.mjs',import.meta.url),[],{stdio:['ignore','ignore','inherit','ipc'],env:{...process.env,FOCALROOM_TEST_QUEUE_KEY:key,FOCALROOM_TEST_QUEUE_MODEL:`http://127.0.0.1:${server.address().port}/responses`}});
  children.push(child);const exit=once(child,'exit');
  await Promise.race([once(child,'message').then(([message])=>assert.equal(message,'ready')),exit.then(()=>assert.fail('worker stopped before ready'))]);return child;
 }
 async function stopProcess(child,signal='SIGTERM'){if(child.exitCode!==null || child.signalCode)return;const done=once(child,'exit');child.kill(signal);await done;}
 try{
  const u=await user(),id=randomUUID();await submit(u,id);assert.equal(count,0);
  const first=await start();await until(async()=> (await poll(u,id)).body.state==='COMPLETED');
  assert.equal(count,1);await stopProcess(first);
  const secondId=randomUUID();hold=true;await submit(u,secondId);
  const doomed=await start();await until(()=>count===2);await stopProcess(doomed,'SIGKILL');
  const metadata=await job(u,secondId);assert.equal(metadata.phase,'DISPATCHING');assert.equal(metadata.payload,null);
  await pool.query("UPDATE fr_ai_jobs SET dispatched_at=now()-interval '7 minutes' WHERE user_id=$1 AND request_id=$2",[u.user.id,secondId]);
  const replacement=await start();await expireQueued(pool);
  assert.equal((await poll(u,secondId)).body.state,'UNKNOWN');
  await replacement.send('fixture-no-op');await new Promise(resolve=>setTimeout(resolve,350));
  assert.equal(count,2);assert.ok((await quota(u)).reserved>0);await stopProcess(replacement);
 }finally{
  for(const child of children)await stopProcess(child,'SIGKILL');
  for(const socket of sockets)socket.destroy();server.close();await once(server,'close');
 }
});

test('restored queued and dispatched snapshots are quarantined without replay or automatic refund',async()=>{
 const {readFile}=await import('node:fs/promises');
 const u=await user(),waiting=randomUUID(),sending=randomUUID(),start=calls;
 await submit(u,sending);const claimed=await claimQueued(pool,queue,HOSTED_AI_POLICY,v=>v);assert.equal(claimed.requestId,sending);
 await submit(u,waiting);const before=await quota(u);assert.ok(before.reserved>0);
 const sql=await readFile(new URL('../../../ops/shared/quarantine-restored-ai.sql',import.meta.url),'utf8');
 await pool.query(sql);await pool.query(sql); // 恢复保护可重复执行，不增加释放或扣费。
 for(const id of [sending,waiting]){
  assert.equal((await job(u,id)).phase,'UNKNOWN');assert.equal((await job(u,id)).payload,null);
  assert.equal((await poll(u,id)).body.state,'UNKNOWN');
 }
 const after=await quota(u);assert.equal(after.reserved,before.reserved);assert.equal(after.spent,before.spent);
 const w=await worker();await w.runQueueOnce();assert.equal(calls,start);await stop(w);
});

test('a late model response cannot overwrite an audited reconciliation',async()=>{
 const u=await user(),id=randomUUID(),start=calls;mode='blocked';await submit(u,id);
 const w=await worker();
 try{
  await w.runQueueOnce();await until(()=>calls===start+1);
  await pool.query("UPDATE fr_ai_requests SET created_at=now()-interval '11 minutes' WHERE user_id=$1 AND request_id=$2",[u.user.id,id]);
  await pool.query("UPDATE fr_ai_jobs SET dispatched_at=now()-interval '7 minutes' WHERE user_id=$1 AND request_id=$2",[u.user.id,id]);
  await expireQueued(pool);await poll(u,id);
  assert.equal((await reconcileAiRequest(pool,u.user.id,id,5,'fixture-audited-bill')).applied,true);
  release();await stop(w);
  assert.equal((await quota(u)).spent,5);assert.equal((await quota(u)).reserved,0);
  assert.equal((await job(u,id)).phase,'DONE');assert.equal((await job(u,id)).attempts,1);
  assert.equal((await poll(u,id)).body.result,null);assert.equal(calls,start+1);
 }finally{release?.();mode='ok';await stop(w);}
});

test('queue metrics expose aggregate phase, encrypted bytes and age without prompts, IDs or keys',async()=>{
 const u=await user(),id=randomUUID();await submit(u,id);
 const text=await (await fetch(base+'/api/v2/metrics')).text();
 assert.match(text,/focalroom_hosted_ai_jobs\{phase="QUEUED"\} 1/);
 assert.match(text,/focalroom_hosted_ai_queued_input_bytes [1-9][0-9]*/);
 assert.match(text,/focalroom_hosted_ai_oldest_queued_seconds [0-9]+/);
 for(const privateText of [u.user.id,u.token,id,key,secret,'private-fixture-text'])assert.ok(!text.includes(privateText));
 await cancel(u,id);
});

test('cross-instance queue capacity rejects overflow and rolls back the losing reservation',async()=>{
 const filler=await user(),a=await user(),b=await user(),start=calls;
 await pool.query("INSERT INTO fr_ai_requests(user_id,request_id,body_hash,policy,reserved,state) SELECT $1,'capacity-'||g::text,'fixture',$2,1,'PENDING' FROM generate_series(1,255) g",[filler.user.id,HOSTED_AI_POLICY]);
 await pool.query('UPDATE fr_ai_accounts SET reserved=255 WHERE user_id=$1',[filler.user.id]);
 await pool.query("INSERT INTO fr_ai_jobs(user_id,request_id,session_digest,key_id,input_hash,images,payload,expires_at) SELECT user_id,request_id,$2,'first',repeat('0',64),0,decode(repeat('00',29),'hex'),now()+interval '10 minutes' FROM fr_ai_requests WHERE user_id=$1 AND request_id LIKE 'capacity-%'",[filler.user.id,digest(filler.token)]);
 const results=await Promise.all([submit(a),submit(b)]);
 assert.deepEqual(results.map(r=>r.status).sort(),[202,429]);
 const losing=results[0].status===429?a:b;
 assert.equal((await quota(losing)).reserved,0);
 assert.equal((await pool.query("SELECT count(*)::int AS n FROM fr_ai_jobs WHERE phase='QUEUED'")).rows[0].n,256);
 assert.equal(calls,start);
 const {readFile}=await import('node:fs/promises');await pool.query(await readFile(new URL('../../../ops/shared/quarantine-restored-ai.sql',import.meta.url),'utf8'));
});
