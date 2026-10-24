import { test, expect, type Page } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { defaultAgentBudget, type AgentTask } from '../src/domain/agent';
import { installAccountFixture } from './account-fixture';


async function installSurface(page:Page){
 const outside: string[] = [], errors: string[] = [];
 page.on('pageerror', error => errors.push(error.message));
 await page.route('**/*', route => {
  if (['localhost','127.0.0.1','[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
  outside.push(route.request().url()); return route.abort();
 });
 await installAccountFixture(page);
 await page.addInitScript(() => {
  localStorage.setItem('focalroom-locale','zh');
  Object.assign(globalThis,{isTauri:true,__TAURI_EVENT_PLUGIN_INTERNALS__:{unregisterListener:()=>{}},__TAURI_INTERNALS__:{
   invoke:async (command:string,args:unknown) => {
    if(command.startsWith('plugin:event|') || command==='set_native_menu_locale') return 0;
    const result=await fetch('http://localhost:1447/rpc',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({command,args})}).then(r=>r.json());
    if(!result.ok) throw result.error; return result.value;
   },convertFileSrc:(path:string)=>'http://localhost:1447/asset?path='+encodeURIComponent(path),transformCallback:()=>0,
  }});
 });
 return {outside,errors};
}

// 界面状态夹具；真实传输/持久回执/重启由 hosted_ai.rs 的本机 HTTP 测试验证。
test('未确认结果只查询，找回后必须明确继续，页面重开不会自动查询或发图', async ({ page }) => {
 const {outside,errors}=await installSurface(page);
 const commands: Record<string,unknown>[]=[];
 const task: AgentTask = { id:'recovery-ui-task',goal:'看看这组照片的叙事节奏',baseRevision:1,state:'paused',reason:'AI_RESULT_UNKNOWN',
  budget:{...defaultAgentBudget,maxCloudImages:2},usage:{modelTurns:1,toolCalls:0,inputEstimate:10,outputEstimate:5,cloudImages:2,costMinorUsd:1},elapsedMs:100,
  question:null,finalDraftIds:[],providerId:'openai',steps:[],createdAt:new Date().toISOString(),visionRecovery:{state:'unknown',images:2},conversation:true,messages:[{id:1,role:'user',content:'看看这组照片的叙事节奏',createdAt:new Date().toISOString()}] };
 let lookups=0;
 await page.route('**/rpc', async route => {
  const { command, args }=route.request().postDataJSON();
  let value: unknown;
  if (command==='get_agent_projection') value={tasks:[task],drafts:[]};
  else if (command==='get_ai_config') value={observation:null,consentProvider:'openai@'+'a'.repeat(64),providerId:'openai',modelId:'fixture',ready:true,apiKeyTail:null,keySource:'keychain',baseUrl:'http://localhost:1447/fixture',promptVersion:'fixture',capabilities:['vision','tool_calling','structured_output','instruction_following'],capabilitySource:'operator_declaration',configurationError:null,consent:null};
  else if (command==='query_vision_result') {
   lookups++; expect(args.taskId).toBe(task.id);
   if (lookups===1) {task.visionRecovery={state:'pending',images:2};value=task.visionRecovery;}
   else {task.visionRecovery=null;task.reason='照片分析结果已找回，可继续本次任务';task.steps=[{id:1,kind:'vision_result_recovered',summary:'结果已找回',createdAt:new Date().toISOString()}];value={state:'completed',images:2};}
  } else if (command==='agent_request') {
   commands.push(args.request.command);
   task.state='queued';value={ok:true,data:{taskId:task.id},error:null,revision:null,requestId:args.request.requestId};
  } else return route.fallback();
  return route.fulfill({json:{ok:true,value}});
 });
 const created=await fetch('http://localhost:1447/rpc',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({command:'create_project',args:{title:'连接恢复',brief:'保留当前工作，确认已有分析'}})}).then(r=>r.json());
 await page.goto('/#/project/'+created.value.id);
 await page.getByRole('main').getByRole('button',{name:'Focal AI',exact:true}).click();
 const drawer=page.getByRole('dialog',{name:'Focal AI'});
 await expect(drawer.getByRole('button',{name:'查询处理结果'})).toBeVisible();
 expect(lookups).toBe(0);expect(commands).toEqual([]);
 await expect(drawer.getByRole('button',{name:'调整照片范围'})).toHaveCount(0);
 await drawer.getByRole('button',{name:'查询处理结果'}).click();
 await expect(drawer.getByText('上次照片分析仍在处理中，请稍后查询。')).toBeVisible();
 await page.reload();
 await page.getByRole('main').getByRole('button',{name:'Focal AI',exact:true}).click();
 await expect(drawer.getByRole('button',{name:'查询处理结果'})).toBeVisible();
 expect(lookups).toBe(1);expect(commands).toEqual([]);
 const evidence=resolve('../docs/v3/evidence/vision-recovery-2026-10-24/browser');await mkdir(evidence,{recursive:true});
 await page.screenshot({path:resolve(evidence,'pending-result.png')});
 await drawer.getByRole('button',{name:'查询处理结果'}).click();
 await expect(drawer.getByRole('button',{name:'继续',exact:true})).toBeEnabled();
 expect(lookups).toBe(2);expect(commands).toEqual([]);
 await page.screenshot({path:resolve(evidence,'recovered-awaiting-continue.png')});
 await drawer.getByRole('button',{name:'继续',exact:true}).click();
 await expect.poll(()=>commands.length).toBe(1);
 expect(commands[0]).toMatchObject({type:'ResumeTask',taskId:task.id,budget:{maxCloudImages:2,maxCostMinorUsd:defaultAgentBudget.maxCostMinorUsd}});
 expect(outside).toEqual([]);expect(errors).toEqual([]);
});

test('本次恢复许可默认关闭、明确发送、取消回执与下一轮不继承',async({page})=>{
 const {outside,errors}=await installSurface(page);
 const rpc=async<T>(command:string,args:Record<string,unknown>={}):Promise<T>=>{
  const reply=await fetch('http://localhost:1447/rpc',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({command,args})}).then(r=>r.json());
  if(!reply.ok)throw new Error(JSON.stringify(reply.error));return reply.value;
 };
 const project=await rpc<{id:string}>('create_project',{title:'本次分析恢复',brief:'寻找照片之间的观看节奏'});
 const token=await rpc<string>('choose_import',{projectId:project.id});
 const importId=await rpc<string>('import_photos',{projectId:project.id,selectionToken:token});
 await expect.poll(async()=> (await rpc<{status:string}>('get_import_job',{jobId:importId})).status,{timeout:100000}).toBe('completed');
 const original=await rpc('get_project',{projectId:project.id});
 const commands:Record<string,unknown>[]=[];const tasks:AgentTask[]=[];
 const now=()=>new Date().toISOString();
 await page.route('**/rpc',async route=>{
  const {command,args}=route.request().postDataJSON();let value:unknown;
  if(command==='get_agent_projection')value={tasks,drafts:[]};
  else if(command==='get_ai_config')value={observation:null,consentProvider:'openai@'+'a'.repeat(64),providerId:'openai',modelId:'deepseek-flash',ready:true,apiKeyTail:null,keySource:'focalroom',baseUrl:'http://localhost:1447/fixture',promptVersion:'fixture',capabilities:['vision','tool_calling','structured_output','instruction_following'],capabilitySource:'operator_declaration',configurationError:null,consent:null,hostedAccount:{enabled:true,verified:true,policy:'deepseek-flash-v1',provider:'DeepSeek',model:'deepseek-flash',unit:'tokens',granted:10000,spent:0,reserved:0,available:10000,recovery:{enabled:true,policy:'encrypted-queue-v1',maxQueueSeconds:600}}};
  else if(command==='set_privacy_consent')value={id:9,...args.request,used:0,grantedAt:now(),revokedAt:null};
  else if(command==='agent_request'){
   const request=args.request.command;commands.push(request);
   if(request.type==='StartConversation'){
    tasks.unshift({id:'queue-ui-'+commands.length,goal:request.goal,baseRevision:1,state:'paused',reason:'AI_RESULT_UNKNOWN',budget:request.budget,usage:{modelTurns:1,toolCalls:0,inputEstimate:1,outputEstimate:1,cloudImages:1,costMinorUsd:1},elapsedMs:100,question:null,finalDraftIds:[],providerId:'openai',steps:[],createdAt:now(),visionRecovery:{state:'pending',images:1},conversation:true,messages:[{id:commands.length,role:'user',content:request.goal,createdAt:now()}]});
   }else if(request.type==='CancelTask'){
    const task=tasks.find(t=>t.id===request.taskId)!;task.state='cancelled';task.visionRecovery=null;task.queueCancellation='unconfirmed';
   }
   value={ok:true,data:{taskId:tasks[0].id},error:null,revision:null,requestId:args.request.requestId};
  }else return route.fallback();
  return route.fulfill({json:{ok:true,value}});
 });
 await page.goto('/#/project/'+project.id);await page.getByRole('main').getByRole('button',{name:'Focal AI',exact:true}).click();
 const drawer=page.getByRole('dialog',{name:'Focal AI'});await drawer.getByRole('button',{name:'设置',exact:true}).click();
 const settings=page.getByRole('dialog',{name:'设置',exact:true});
 const toggle=settings.getByRole('checkbox',{name:'本次分析可在服务恢复后继续'});
 await expect(toggle).not.toBeChecked();expect(commands).toEqual([]);
 await toggle.check();await expect(settings).toContainText('备份可能保留加密副本');
 const evidence=resolve('../docs/v3/evidence/queue-client-2026-10-24/browser');await mkdir(evidence,{recursive:true});
 await page.screenshot({path:resolve(evidence,'explicit-recovery-choice.png')});
 await settings.getByRole('button',{name:'关闭对话框',exact:true}).click();
 await drawer.getByRole('textbox',{name:'消息'}).fill('比较这组照片的叙事节奏');
 await drawer.getByRole('button',{name:'发送',exact:true}).click();
 await expect.poll(()=>commands.length).toBe(1);expect(commands[0]).toMatchObject({type:'StartConversation',allowQueuedRecovery:true});
 await drawer.getByRole('button',{name:'取消任务',exact:true}).click();
 await expect(drawer.getByText(/暂未确认排队撤回结果/)).toBeVisible();
 expect(commands[1]).toMatchObject({type:'CancelTask',taskId:tasks[0].id});
 await page.screenshot({path:resolve(evidence,'cancellation-unconfirmed.png')});
 await drawer.getByRole('button',{name:'设置',exact:true}).click();await expect(toggle).not.toBeChecked();
 await settings.getByRole('button',{name:'关闭对话框',exact:true}).click();
 await drawer.getByRole('textbox',{name:'消息'}).fill('继续讨论另一种方向');await drawer.getByRole('button',{name:'发送',exact:true}).click();
 await expect.poll(()=>commands.length).toBe(3);expect(commands[2]).not.toHaveProperty('allowQueuedRecovery');
 expect(await rpc('get_project',{projectId:project.id})).toEqual(original);
 expect(outside).toEqual([]);expect(errors).toEqual([]);
});
