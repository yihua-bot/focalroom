// 真实 Rust AgentEngine → NestJS → 隔离 PostgreSQL；模型仅为进程内替身，照片为生成色块。
import {test,before,after,afterEach} from 'node:test';
import assert from 'node:assert/strict';
import {randomBytes,randomUUID,createHash} from 'node:crypto';
import {join,resolve} from 'node:path';
import {mkdtemp,readFile,access} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {once} from 'node:events';
import {DatabaseSync} from 'node:sqlite';
import {Pool} from 'pg';
import {createApi} from '../src/server.ts';
import {migrate,digest} from '../src/database.ts';
import {LocalPreviewStore} from '../src/storage.ts';
import {HostedAiService} from '../src/hosted-ai.ts';

const binary=resolve('src-tauri',process.env.CARGO_TARGET_DIR||'target','debug','m1-harness');
const origin='http://127.0.0.1:1424';
const config={key:'local-fixture-only',initialTokens:1000000,queue:{activeKeyId:'native',keys:{native:randomBytes(32).toString('base64')}}};
const budget={maxModelTurns:4,maxToolCalls:6,maxDurationSeconds:480,maxCloudImages:1,maxCostMinorUsd:100,maxContextTokens:48000,maxTotalTokens:100000,maxOutputTokens:1024};
let pool,api,base,photoId,projectId,blocked=false,release;
let textCalls=0,imageCalls=0;
const clients=new Set(),workers=new Set();
const reply=output=>Response.json({id:randomUUID(),model:'deepseek-flash',status:'completed',output,usage:{input_tokens:20,output_tokens:10,total_tokens:30}});
async function transport(body,key){
 assert.equal(key,config.key);assert.equal(body.store,false);assert.equal(body.model,'deepseek-flash');
 const content=body.input.flatMap(item=>item.content||[]);
 const images=content.filter(part=>part.type==='input_image');
 if(images.length){
  imageCalls++;assert.equal(images.length,1);
  assert.equal(content.find(part=>part.text?.startsWith('photo_id: ')).text,'photo_id: '+photoId);
  assert.ok(images[0].image_url.startsWith('data:image/jpeg;base64,'));
  if(blocked)await new Promise(resolve=>{release=resolve;});
  const observation={observations:[{photo_id:photoId,visible_facts:['generated color fixture'],interpretation:'local protocol fixture',uncertainty:'not a professional quality evaluation'}]};
  return reply([{type:'message',content:[{type:'output_text',text:JSON.stringify(observation)}]}]);
 }
 textCalls++;
 if(body.input.some(item=>item.type==='function_call_output'))return reply([{type:'message',content:[{type:'output_text',text:'本机协议检查完成。'}]}]);
 return reply([{type:'function_call',call_id:randomUUID(),name:'inspect_photos',arguments:JSON.stringify({project_id:projectId,photo_ids:[photoId]})}]);
}
async function until(read,predicate,label='local state',timeout=20000){
 const start=Date.now();let value;
 do{value=await read();if(predicate(value))return value;await new Promise(r=>setTimeout(r,25));}while(Date.now()-start<timeout);
 assert.fail(`${label} timed out: ${JSON.stringify(value)}`);
}
class NativeClient{
 constructor(root){
  this.root=root;this.pending=new Map();this.sequence=0;this.stderr='';
  this.child=spawn(binary,[root,'hosted-ai'],{stdio:['pipe','pipe','pipe'],env:{...process.env,BETWEEN_FRAMES_OPENAI_API_KEY:'',BETWEEN_FRAMES_ANTHROPIC_API_KEY:'',BETWEEN_FRAMES_GEMINI_API_KEY:'',FOCALROOM_AGENT_MODEL_KEY:'',FOCALROOM_DEEPSEEK_API_KEY:''}});
  this.exited=once(this.child,'exit');clients.add(this);
  createInterface({input:this.child.stdout}).on('line',line=>{
   const response=JSON.parse(line),pending=this.pending.get(response.id);this.pending.delete(response.id);
   if(response.ok)pending?.resolve(response.value);else pending?.reject(Object.assign(new Error(response.error.code),response.error));
  });
  this.child.stderr.on('data',chunk=>{this.stderr=(this.stderr+chunk).slice(-8000);});
  this.child.once('exit',()=>{for(const pending of this.pending.values())pending.reject(new Error('native process stopped'));this.pending.clear();});
 }
 async rpc(command,args={}){
  const id=++this.sequence;
  const result=new Promise((resolve,reject)=>this.pending.set(id,{resolve,reject}));
  this.child.stdin.write(JSON.stringify({id,command,args})+'\n');return result;
 }
 async stop(){if(this.child.exitCode===null&&this.child.signalCode===null){this.child.kill('SIGKILL');await this.exited;}clients.delete(this);}
 async login(user){return this.rpc('set_hosted_ai_test_session',{origin:base,token:user.token});}
 async task(id){return (await this.rpc('get_agent_projection',{projectId})).tasks.find(task=>task.id===id);}
 async command(command,requestId=randomUUID()){const result=await this.rpc('agent_request',{request:{requestId,projectId,command}});assert.equal(result.ok,true,JSON.stringify(result.error));return result.data.taskId;}
}
async function user(){
 const u=await api.service.auth.account({email:`${randomUUID()}@fixture.test`,password:'local-fixture-password',display_name:'Native Fixture'},true,randomUUID());
 await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1',[u.user.id]);return u;
}
async function setup(allow=true){
 const account=await user(),root=await mkdtemp(join(process.env.FOCALROOM_TEST_DIRECTORY,'native-'));
 const client=new NativeClient(root);await client.rpc('list_projects');await client.login(account);
 const project=await client.rpc('create_project',{title:'本机组合验证',brief:'仅协议测试，不作摄影质量结论'});projectId=project.id;
 const selectionToken=await client.rpc('choose_import',{projectId});const jobId=await client.rpc('import_photos',{projectId,selectionToken});
 await until(()=>client.rpc('get_import_job',{jobId}),job=>job.status==='completed','generated photo import');
 const original=await client.rpc('get_project',{projectId});assert.equal(original.photos.length,1);photoId=original.photos[0].id;
 const hash=createHash('sha256').update(await readFile(original.photos[0].originalPath||original.photos[0].original_path)).digest('hex');
 const ai=await client.rpc('get_ai_config',{projectId});
 const consent=await client.rpc('set_privacy_consent',{request:{requestId:randomUUID(),projectId,mode:'task',provider:ai.consentProvider,quota:1}});
 const id=await client.command({type:'StartConversation',expectedAccountId:account.user.id,goal:'观察当前色块，明确说明依据',selectedPhotoIds:[],focusPhotoIds:[],targetCount:1,budget,consentId:consent.id,sourceBoard:null,continuation:null,...(allow?{allowQueuedRecovery:true}:{})});
 return {account,root,client,id,original,hash};
}
const jobFor=async account=>(await pool.query('SELECT * FROM fr_ai_jobs WHERE user_id=$1',[account.user.id])).rows[0];
const quota=async account=>(await pool.query('SELECT spent,reserved FROM fr_ai_accounts WHERE user_id=$1',[account.user.id])).rows[0];
async function queued(account){return until(()=>jobFor(account),row=>row?.phase==='QUEUED','encrypted native queue');}
async function worker(){const service=new HostedAiService(pool,api.service.auth,config,transport);workers.add(service);await service.runQueueOnce();return service;}
async function restart(fixture){await fixture.client.stop();const next=new NativeClient(fixture.root);await next.rpc('list_projects');await next.login(fixture.account);return next;}
async function assertOriginal(client,fixture){
 const after=await client.rpc('get_project',{projectId});assert.equal(after.revision,fixture.original.revision);
 assert.deepEqual(after.photos,fixture.original.photos);
 assert.equal(createHash('sha256').update(await readFile(after.photos[0].originalPath||after.photos[0].original_path)).digest('hex'),fixture.hash);
}
function localCounts(root,id){
 const db=new DatabaseSync(join(root,'projects.sqlite'),{readOnly:true});
 try{return {observations:db.prepare('SELECT count(*) AS count FROM photo_observations').get().count,settlements:db.prepare("SELECT count(*) AS count FROM agent_steps WHERE task_id=? AND kind='vision_settled'").get(id).count};}finally{db.close();}
}
before(async()=>{
 assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);assert.ok(process.env.FOCALROOM_TEST_DIRECTORY);await access(binary);
 pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);
 api=await createApi({pool,origin,store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'native-previews')),hostedAi:config,hostedAiTransport:transport});
 // 真实 HTTP 提交，但先停消费者，以确定性验证进程中断和待发送状态。
 await api.service.ai.drain();await api.app.listen(0,'127.0.0.1');base=await api.app.getUrl();
});
afterEach(async()=>{release?.();release=undefined;blocked=false;for(const client of [...clients])await client.stop();for(const service of workers)await service.drain();workers.clear();});
after(async()=>{await api?.close();await pool?.end();});

test('native image job survives a killed client, completes through replacement worker and resumes without reupload or double charge',{timeout:60000},async()=>{
 const startImages=imageCalls,startText=textCalls,f=await setup();const stored=await queued(f.account);
 assert.equal(stored.session_digest,digest(f.account.token));assert.equal(stored.attempts,0);assert.ok(stored.payload.length>0);
 assert.equal(stored.payload.includes(Buffer.from('data:image/jpeg')),false);
 const client=await restart(f);assert.equal((await client.task(f.id)).state,'interrupted');
 assert.equal((await client.rpc('query_vision_result',{projectId,taskId:f.id})).state,'pending');assert.equal(imageCalls,startImages);
 await worker();await until(()=>jobFor(f.account),row=>row?.phase==='DONE','completed server job');
 assert.equal((await client.rpc('query_vision_result',{projectId,taskId:f.id})).state,'completed');
 const ready=await client.task(f.id);assert.equal(ready.state,'interrupted');assert.equal(ready.usage.cloudImages,1);
 // 找回仅结算并保存待回放结果；用户明确继续后才写入观察缓存。
 assert.deepEqual(localCounts(f.root,f.id),{observations:0,settlements:1});
 await client.command({type:'ResumeTask',taskId:f.id,budget});
 await until(()=>client.task(f.id),task=>['completed','review_ready'].includes(task.state),'resumed native conversation');
 assert.equal(imageCalls,startImages+1);assert.equal(textCalls,startText+2);assert.deepEqual(await quota(f.account),{spent:'90',reserved:'0'});
 assert.equal((await jobFor(f.account)).payload,null);assert.equal((await jobFor(f.account)).attempts,1);
 await assert.rejects(client.rpc('query_vision_result',{projectId,taskId:f.id}));
 assert.deepEqual(localCounts(f.root,f.id),{observations:1,settlements:1});await assertOriginal(client,f);
});

test('native cancellation after restart withdraws only the queued request and releases its server reservation',{timeout:60000},async()=>{
 const before=imageCalls,f=await setup();await queued(f.account);const client=await restart(f);
 const cancellationId=randomUUID();await client.command({type:'CancelTask',taskId:f.id},cancellationId);assert.equal((await client.task(f.id)).queueCancellation,'cancelled');
 await client.command({type:'CancelTask',taskId:f.id},cancellationId);await worker();
 assert.equal(imageCalls,before);assert.deepEqual(await quota(f.account),{spent:'30',reserved:'0'});
 const job=await jobFor(f.account);assert.equal(job.phase,'DONE');assert.equal(job.attempts,0);assert.equal(job.payload,null);
 assert.deepEqual(localCounts(f.root,f.id),{observations:0,settlements:0});await assertOriginal(client,f);
});

test('native cancellation honestly reports already dispatched and never refunds or applies the later result',{timeout:60000},async()=>{
 const before=imageCalls,f=await setup();await queued(f.account);blocked=true;await worker();
 await until(()=>imageCalls,n=>n===before+1,'blocked upstream request');
 assert.equal((await jobFor(f.account)).payload,null);const client=await restart(f);
 await client.command({type:'CancelTask',taskId:f.id});assert.equal((await client.task(f.id)).queueCancellation,'already_sent');
 assert.ok(Number((await quota(f.account)).reserved)>0);release();
 await until(()=>jobFor(f.account),row=>row?.phase==='DONE','late settled response');
 assert.deepEqual(await quota(f.account),{spent:'60',reserved:'0'});assert.equal((await client.task(f.id)).state,'cancelled');
 assert.equal(imageCalls,before+1);assert.deepEqual(localCounts(f.root,f.id),{observations:0,settlements:0});await assertOriginal(client,f);
});

test('switching native accounts prevents recovery, and revoking the original session rejects the unsent encrypted job',{timeout:60000},async()=>{
 const before=imageCalls,f=await setup();await queued(f.account);const client=await restart(f),other=await user();await client.login(other);
 await assert.rejects(client.rpc('query_vision_result',{projectId,taskId:f.id}),e=>e.code==='AUTH_CHANGED');
 await pool.query('DELETE FROM fr_sessions WHERE digest=$1',[digest(f.account.token)]);await worker();
 await until(()=>jobFor(f.account),row=>row?.phase==='DONE','revoked queued request');
 assert.equal(imageCalls,before);assert.deepEqual(await quota(f.account),{spent:'30',reserved:'0'});assert.equal((await jobFor(f.account)).payload,null);
 assert.deepEqual(localCounts(f.root,f.id),{observations:0,settlements:0});await assertOriginal(client,f);
});

test('native default does not opt into durable storage even when the service offers recovery',{timeout:60000},async()=>{
 const before=imageCalls,f=await setup(false);
 await until(()=>f.client.task(f.id),task=>['completed','review_ready'].includes(task.state),'default native conversation');
 assert.equal(await jobFor(f.account),undefined);assert.equal(imageCalls,before+1);
 assert.deepEqual(await quota(f.account),{spent:'90',reserved:'0'});assert.deepEqual(localCounts(f.root,f.id),{observations:1,settlements:1});await assertOriginal(f.client,f);
});
