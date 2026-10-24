# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: selection-revision.spec.ts >> 真实选片修订：部分采纳后另存、查看冻结依据、重启保持且不修改父版或作品
- Location: e2e/selection-revision.spec.ts:14:1

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: getByRole('dialog', { name: 'Focal AI', exact: true }).getByText('新的选片修订已准备好，前版与作品序列保留', { exact: true })
Expected: visible
Timeout: 30000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" getByRole('dialog', { name: 'Focal AI', exact: true }).getByText('新的选片修订已准备好，前版与作品序列保留', { exact: true }) with timeout 30000ms
  - waiting for getByRole('dialog', { name: 'Focal AI', exact: true }).getByText('新的选片修订已准备好，前版与作品序列保留', { exact: true })

```

```yaml
- complementary:
  - img "FocalRoom 标志"
  - text: FocalRoom PHOTO STUDIO
  - button "收起导航":
    - img
  - navigation "项目列表":
    - button "项目列表":
      - img
      - text: 项目列表
  - text: 当前项目
  - strong: 选片修订依据
  - text: 200 张图像 · 1 张入选 本地项目
  - navigation "选片修订依据":
    - button "选片工作台":
      - img
      - text: 选片工作台
    - button "作品序列":
      - img
      - text: 作品序列
    - button "叙事画布":
      - img
      - text: 叙事画布
    - button "出版物":
      - img
      - text: 出版物
  - button "账户菜单":
    - strong: Yihua
    - text: 基础版
    - img
- main:
  - link "← 项目列表":
    - /url: "#/projects"
  - heading "选片修订依据" [level=1]
  - button "导入照片":
    - img
    - text: 导入照片
  - button "Focal AI":
    - img
    - text: Focal AI
  - text: “
  - paragraph: 保留每一轮选片和人工判断
  - button "编辑创作意图 ↗"
  - group "编辑历史":
    - button "撤销"
    - button "重做" [disabled]
    - button "作品版本"
    - button "调色版本"
    - button "操作历史 · 202"
    - text: 1 次可撤销编辑
  - button "全部 200" [pressed]
  - button "已保留 1"
  - button "待定 199"
  - button "已淘汰 0"
  - button "切换网格密度":
    - img
  - button "两图对比"
  - button "导出 ↗"
  - button "收起序列带" [pressed]:
    - img
  - button "收起侧栏":
    - img
  - img
  - searchbox "搜索照片文件名或备注"
  - text: 分组
  - combobox "网格分组":
    - option "不分组" [selected]
    - option "按选片状态"
    - option "按人工评分"
    - option "按文件名前缀"
  - text: ⌘ / Ctrl 点选 · Shift 连选 CONTACT SHEET 200 张
  - article:
    - button "查看第 1 张 frame-000.jpg":
      - img "frame-000.jpg"
      - text: "01"
    - text: FRAME 01
    - button "标记保留 frame-000.jpg":
      - img
    - button "标记淘汰 frame-000.jpg":
      - img
  - article:
    - button "查看第 2 张 frame-001.jpg":
      - img "frame-001.jpg"
      - text: 02 ◇ 已保留
    - text: FRAME 02
    - button "标记保留 frame-001.jpg" [pressed]:
      - img
    - button "标记淘汰 frame-001.jpg":
      - img
  - article:
    - button "查看第 3 张 frame-002.jpg":
      - img "frame-002.jpg"
      - text: "03"
    - text: FRAME 03
    - button "标记保留 frame-002.jpg":
      - img
    - button "标记淘汰 frame-002.jpg":
      - img
  - article:
    - button "查看第 4 张 frame-003.jpg":
      - img "frame-003.jpg"
      - text: "04"
    - text: FRAME 04
    - button "标记保留 frame-003.jpg":
      - img
    - button "标记淘汰 frame-003.jpg":
      - img
  - article:
    - button "查看第 5 张 frame-004.jpg":
      - img "frame-004.jpg"
      - text: "05"
    - text: FRAME 05
    - button "标记保留 frame-004.jpg":
      - img
    - button "标记淘汰 frame-004.jpg":
      - img
  - article:
    - button "查看第 6 张 frame-005.jpg":
      - img "frame-005.jpg"
      - text: "06"
    - text: FRAME 06
    - button "标记保留 frame-005.jpg":
      - img
    - button "标记淘汰 frame-005.jpg":
      - img
  - article:
    - button "查看第 7 张 frame-006.jpg":
      - img "frame-006.jpg"
      - text: "07"
    - text: FRAME 07
    - button "标记保留 frame-006.jpg":
      - img
    - button "标记淘汰 frame-006.jpg":
      - img
  - article:
    - button "查看第 8 张 frame-007.jpg":
      - img "frame-007.jpg"
      - text: "08"
    - text: FRAME 08
    - button "标记保留 frame-007.jpg":
      - img
    - button "标记淘汰 frame-007.jpg":
      - img
  - article:
    - button "查看第 9 张 frame-008.jpg":
      - img "frame-008.jpg"
      - text: "09"
    - text: FRAME 09
    - button "标记保留 frame-008.jpg":
      - img
    - button "标记淘汰 frame-008.jpg":
      - img
  - article:
    - button "查看第 10 张 frame-009.jpg":
      - img "frame-009.jpg"
      - text: "10"
    - text: FRAME 10
    - button "标记保留 frame-009.jpg":
      - img
    - button "标记淘汰 frame-009.jpg":
      - img
  - article:
    - button "查看第 11 张 frame-010.jpg":
      - img "frame-010.jpg"
      - text: "11"
    - text: FRAME 11
    - button "标记保留 frame-010.jpg":
      - img
    - button "标记淘汰 frame-010.jpg":
      - img
  - article:
    - button "查看第 12 张 frame-011.jpg":
      - img "frame-011.jpg"
      - text: "12"
    - text: FRAME 12
    - button "标记保留 frame-011.jpg":
      - img
    - button "标记淘汰 frame-011.jpg":
      - img
  - article:
    - button "查看第 13 张 frame-012.jpg":
      - img "frame-012.jpg"
      - text: "13"
    - text: FRAME 13
    - button "标记保留 frame-012.jpg":
      - img
    - button "标记淘汰 frame-012.jpg":
      - img
  - article:
    - button "查看第 14 张 frame-013.jpg":
      - img "frame-013.jpg"
      - text: "14"
    - text: FRAME 14
    - button "标记保留 frame-013.jpg":
      - img
    - button "标记淘汰 frame-013.jpg":
      - img
  - article:
    - button "查看第 15 张 frame-014.jpg":
      - img "frame-014.jpg"
      - text: "15"
    - text: FRAME 15
    - button "标记保留 frame-014.jpg":
      - img
    - button "标记淘汰 frame-014.jpg":
      - img
  - article:
    - button "查看第 16 张 frame-015.jpg":
      - img "frame-015.jpg"
      - text: "16"
    - text: FRAME 16
    - button "标记保留 frame-015.jpg":
      - img
    - button "标记淘汰 frame-015.jpg":
      - img
  - article:
    - button "查看第 17 张 frame-016.jpg":
      - img "frame-016.jpg"
      - text: "17"
    - text: FRAME 17
    - button "标记保留 frame-016.jpg":
      - img
    - button "标记淘汰 frame-016.jpg":
      - img
  - article:
    - button "查看第 18 张 frame-017.jpg":
      - img "frame-017.jpg"
      - text: "18"
    - text: FRAME 18
    - button "标记保留 frame-017.jpg":
      - img
    - button "标记淘汰 frame-017.jpg":
      - img
  - article:
    - button "查看第 19 张 frame-018.jpg":
      - img "frame-018.jpg"
      - text: "19"
    - text: FRAME 19
    - button "标记保留 frame-018.jpg":
      - img
    - button "标记淘汰 frame-018.jpg":
      - img
  - article:
    - button "查看第 20 张 frame-019.jpg":
      - img "frame-019.jpg"
      - text: "20"
    - text: FRAME 20
    - button "标记保留 frame-019.jpg":
      - img
    - button "标记淘汰 frame-019.jpg":
      - img
  - article:
    - button "查看第 21 张 frame-020.jpg":
      - img "frame-020.jpg"
      - text: "21"
    - text: FRAME 21
    - button "标记保留 frame-020.jpg":
      - img
    - button "标记淘汰 frame-020.jpg":
      - img
  - article:
    - button "查看第 22 张 frame-021.jpg":
      - img "frame-021.jpg"
      - text: "22"
    - text: FRAME 22
    - button "标记保留 frame-021.jpg":
      - img
    - button "标记淘汰 frame-021.jpg":
      - img
  - article:
    - button "查看第 23 张 frame-022.jpg":
      - img "frame-022.jpg"
      - text: "23"
    - text: FRAME 23
    - button "标记保留 frame-022.jpg":
      - img
    - button "标记淘汰 frame-022.jpg":
      - img
  - article:
    - button "查看第 24 张 frame-023.jpg":
      - img "frame-023.jpg"
      - text: "24"
    - text: FRAME 24
    - button "标记保留 frame-023.jpg":
      - img
    - button "标记淘汰 frame-023.jpg":
      - img
  - strong: Selected 01
  - text: 作品初选序列
  - button "查看入选 frame-001.jpg":
    - img "frame-001.jpg"
  - button "编辑序列 →"
- complementary "照片详情":
  - heading "照片详情" [level=2]
  - region "照片详情":
    - button "查看完整大图":
      - img "当前选中：frame-000.jpg"
      - img
    - text: FRAME 01
    - heading "frame-000.jpg" [level=3]
    - text: 待定
    - paragraph: 480 × 320
    - group "人工星级":
      - text: 你的评分
      - button "人工评分 1 星": ★
      - button "人工评分 2 星": ★
      - button "人工评分 3 星": ★
      - button "人工评分 4 星": ★
      - button "人工评分 5 星": ★
      - button "清除人工评分" [disabled]: 清除
    - group "册页标记":
      - text: 册页（印刷）
      - button "跨页大图"
      - button "整页图"
      - text: 默认每页两张
    - button "◇ 保留作品"
    - button "× 淘汰作品"
    - text: 照片备注
    - textbox "照片备注":
      - /placeholder: 构图、光线、拍摄背景…
    - button "保存备注" [disabled]
- dialog "Focal AI":
  - banner:
    - img
    - heading "Focal AI" [level=2]
    - paragraph: 选片修订依据
    - button "审美记忆":
      - img
    - button "设置":
      - img
    - button "关闭 Focal AI":
      - img
  - log "Focal AI 对话":
    - article "选片请求":
      - paragraph: 为开篇选择两张照片，先给方案
      - paragraph: 两份本地 Mock 草案已核验；仅供交互审核，不代表视觉或审美理解
      - img
      - strong: 项目已更新
      - paragraph: 正式项目已更新，草案需重新比较基线
      - img
      - strong: 方案 B · 反向节奏
      - paragraph: 本地 Mock 编辑练习；保留人工最终决定
      - button "查看方案"
      - button "继续修改"
      - img
      - strong: 方案 A · 人工线索
      - paragraph: 本地 Mock 编辑练习；保留人工最终决定
      - button "查看方案"
      - button "继续修改"
    - article "选片请求":
      - paragraph: 把顺序换一个方向，保留上一版和作品序列
      - img
      - strong: 失败
      - paragraph: MODEL_UNAVAILABLE：SIMULATED 修订剧本遇到拒绝
  - paragraph: 当前服务不支持看图，可参考备注和选片记录给出建议。
  - textbox "消息":
    - /placeholder: 问摄影问题，或告诉我你想一起完成什么…
  - group: 选项
  - button "发送" [disabled]:
    - img
  - img
  - combobox "照片范围":
    - option "项目照片 · 200" [selected]
    - option "所选照片 · 0"
```

# Test source

```ts
  1  | import { test,expect } from '@playwright/test';
  2  | import { randomUUID } from 'node:crypto';
  3  | import { mkdir } from 'node:fs/promises';
  4  | import { resolve } from 'node:path';
  5  | import { defaultAgentBudget,type AgentProjection } from '../src/domain/agent';
  6  | import { installAccountFixture,installDesktopBridge } from './account-fixture';
  7  | import { evidenceDirectory } from './evidence-directory';
  8  | 
  9  | const base='http://localhost:1447';
  10 | const rpc=async<T>(command:string,args:Record<string,unknown>={}):Promise<T>=>{
  11 |  const reply=await fetch(base+'/rpc',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({command,args})}).then(response=>response.json());if(!reply.ok)throw new Error(reply.error.message);return reply.value;
  12 | };
  13 | type LocalProject={id:string;revision:number;sequence:string[];photos:{id:string;name:string;status:string}[]};
  14 | test('真实选片修订：部分采纳后另存、查看冻结依据、重启保持且不修改父版或作品',async({page})=>{
  15 |  const outside:string[]=[],errors:string[]=[];page.on('pageerror',error=>errors.push(error.message));
  16 |  await page.route('**/*',route=>{if(['localhost','127.0.0.1','[::1]'].includes(new URL(route.request().url()).hostname))return route.continue();outside.push(route.request().url());return route.abort();});
  17 |  await installAccountFixture(page);await installDesktopBridge(page,base);
  18 |  const created=await rpc<LocalProject>('create_project',{title:'选片修订依据',brief:'保留每一轮选片和人工判断'});const projectId=created.id;
  19 |  const token=await rpc<string>('choose_import',{projectId});const jobId=await rpc<string>('import_photos',{projectId,selectionToken:token});
  20 |  await expect.poll(async()=>(await rpc<{status:string}>('get_import_job',{jobId})).status,{timeout:100000}).toBe('completed');
  21 |  const project=await rpc<LocalProject>('get_project',{projectId});const originalHashes=await rpc('test_fixture_checksums');
  22 |  const request=await rpc<{ok:boolean;data:{taskId:string};error:unknown}>('agent_request',{request:{requestId:randomUUID(),projectId,command:{type:'StartConversation',expectedAccountId:null,goal:'为开篇选择两张照片，先给方案',selectedPhotoIds:project.photos.slice(0,2).map(photo=>photo.id),focusPhotoIds:[],targetCount:2,budget:defaultAgentBudget,consentId:null,sourceBoard:null,continuation:null}}});expect(request.ok,JSON.stringify(request.error)).toBe(true);
  23 |  await expect.poll(async()=>(await rpc<AgentProjection>('get_agent_projection',{projectId})).tasks.find(task=>task.id===request.data.taskId)?.state,{timeout:30000}).toBe('review_ready');
  24 |  const initial=await rpc<AgentProjection>('get_agent_projection',{projectId});const parent=initial.drafts.find(draft=>draft.taskId===request.data.taskId)!;const approved=parent.items[0].photo_id;
  25 |  const id=randomUUID();const accepted=await rpc<{ok:boolean}>('execute_gui_command',{request:{commandId:id,idempotencyKey:id,projectId,expectedProjectRevision:project.revision,causationId:id,command:{type:'ApplyDraft',draftId:parent.id,expectedDraftRevision:parent.revision,approvedPhotoIds:[approved],mode:'partial'}}});expect(accepted.ok).toBe(true);
  26 |  const original=await rpc<LocalProject>('get_project',{projectId});expect(original.sequence).toEqual([approved]);const savedParent=(await rpc<AgentProjection>('get_agent_projection',{projectId})).drafts.find(draft=>draft.id===parent.id)!;
  27 |  await page.goto('/#/project/'+projectId);await page.getByRole('main').getByRole('button',{name:'Focal AI',exact:true}).click();const ai=page.getByRole('dialog',{name:'Focal AI',exact:true});
  28 |  await ai.locator('.chat-draft-card').filter({hasText:parent.title}).getByRole('button',{name:'继续修改',exact:true}).click();
  29 |  await ai.getByRole('textbox',{name:'消息',exact:true}).fill('把顺序换一个方向，保留上一版和作品序列');await ai.getByRole('button',{name:'发送',exact:true}).click();
> 30 |  await expect(ai.getByText('新的选片修订已准备好，前版与作品序列保留',{exact:true})).toBeVisible({timeout:30000});
     |                                                                  ^ Error: expect(locator).toBeVisible() failed
  31 |  const result=await rpc<AgentProjection>('get_agent_projection',{projectId});const child=result.drafts.find(draft=>draft.taskId===result.tasks[0].id)!;expect(child.id).not.toBe(parent.id);expect(child.items.map(item=>item.photo_id)).toEqual([...parent.items.map(item=>item.photo_id)].reverse());
  32 |  await ai.locator('.chat-draft-card').filter({hasText:'SIMULATED · 选片修订'}).getByRole('button',{name:'查看方案',exact:true}).click();
  33 |  const review=page.getByRole('dialog',{name:'SIMULATED · 选片修订',exact:true});const context=review.locator('.selection-revision-context');await context.locator('summary').first().click();
  34 |  await expect(context.getByText('采纳所选照片',{exact:true})).toBeVisible();const manual=context.locator('.revision-observation').filter({hasText:'采纳所选照片'});await manual.locator('summary').click();
  35 |  await expect(manual.getByRole('listitem')).toHaveCount(1);await expect(manual.getByRole('listitem')).toHaveText(project.photos.find(photo=>photo.id===approved)!.name);
  36 |  await expect(context.getByText('为开篇选择两张照片，先给方案',{exact:true})).toBeVisible();await expect(context.getByRole('checkbox')).toHaveCount(0);
  37 |  const directory=evidenceDirectory('selection-revision');await mkdir(directory,{recursive:true});await context.scrollIntoViewIfNeeded();await page.screenshot({path:resolve(directory,'selection-context.png')});
  38 |  expect(await rpc('get_project',{projectId})).toEqual(original);expect((await rpc<AgentProjection>('get_agent_projection',{projectId})).drafts.find(draft=>draft.id===parent.id)).toEqual(savedParent);
  39 |  await page.goto('about:blank');await rpc('test_process_restart');await page.goto('/#/project/'+projectId);await page.getByRole('main').getByRole('button',{name:'Focal AI',exact:true}).click();
  40 |  await page.getByRole('dialog',{name:'Focal AI',exact:true}).locator('.chat-draft-card').filter({hasText:'SIMULATED · 选片修订'}).getByRole('button',{name:'查看方案',exact:true}).click();
  41 |  await page.getByRole('dialog',{name:'SIMULATED · 选片修订',exact:true}).locator('.selection-revision-context summary').first().click();await expect(page.getByText('采纳所选照片',{exact:true})).toBeVisible();
  42 |  expect(await rpc('test_fixture_checksums')).toEqual(originalHashes);expect(await rpc('get_project',{projectId})).toEqual(original);expect(outside).toEqual([]);expect(errors).toEqual([]);
  43 | });
  44 | 
```