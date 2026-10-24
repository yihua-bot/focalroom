import { test, expect, type Page } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { defaultAgentBudget } from '../src/domain/agent';
import type { CanvasComparison, CanvasPhoto } from '../src/domain/canvas';
import { installAccountFixture } from './account-fixture';
import { evidenceDirectory } from './evidence-directory';

const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  const result = await fetch('http://localhost:1447/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!result.ok) throw new Error(JSON.stringify(result.error)); return result.value;
};
type Snapshot = { board: { revision: number; owning_task_id: string | null }; nodes: CanvasPhoto[] };
async function prepare(page: Page, authenticated = false, goal = '保留原画布，尝试空间编排') {
  const errors: string[] = [], outside: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    outside.push(route.request().url()); return route.abort();
  });
  if (authenticated) await installAccountFixture(page);
  await page.addInitScript(() => {
    if (location.origin === 'http://localhost:1446') localStorage.setItem('focalroom-locale', 'zh');
    Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
      invoke: async (command: string, args: unknown) => {
        if (command.startsWith('plugin:event|') || command === 'set_native_menu_locale') return 0;
        const result = await fetch('http://localhost:1447/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
        if (!result.ok) throw result.error; return result.value;
      }, convertFileSrc: (path: string) => 'http://localhost:1447/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
    } });
  });
  const created = await rpc<{ id: string }>('create_project', { title: '编辑审稿 ' + randomUUID().slice(0, 6), brief: '保留原画布并比较空间编排' });
  const projectId = created.id;
  const selectionToken = await rpc<string>('choose_import', { projectId });
  const jobId = await rpc<string>('import_photos', { projectId, selectionToken });
  await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100_000 }).toBe('completed');
  const project = await rpc<{ photos: { id: string }[] }>('get_project', { projectId });
  const source = randomUUID();
  const execute = async (baseRevision: number, actions: unknown[]) => rpc<{ revision: number }>('canvas_execute', { request: { schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(), projectId, boardId: source, baseRevision, operations: actions.map(action => ({ opId: randomUUID(), action })) } });
  await execute(0, [{ type: 'CreateBoard', data: { title: `审稿原版 ${source.slice(0, 6)}` } }]);
  const { revision } = await execute(1, [
    { type: 'PlacePhoto', data: { boardId: source, assetId: project.photos[0].id, x: 20, y: 20, width: 220, height: 160 } },
    { type: 'AddText', data: { boardId: source, nodeType: 'sticky', x: 300, y: 40, width: 200, height: 100, content: '保留观看中的停顿' } },
  ]);
  const task = await rpc<{ ok: boolean; data: { taskId: string }; error: unknown }>('agent_request', { request: { requestId: randomUUID(), projectId, command: {
    type: authenticated ? 'StartConversation' : 'StartCanvasTask', ...(authenticated ? { expectedAccountId:null,focusPhotoIds:[],continuation:null } : {}), goal, selectedPhotoIds: [], targetCount: 1, budget: defaultAgentBudget, consentId: null, sourceBoard: { boardId: source, revision, selectedNodeIds: [] },
  } } });
  expect(task.ok, JSON.stringify(task.error)).toBe(true);
  await expect.poll(async () => (await rpc<{ tasks: Array<{ id: string; state: string }> }>('get_agent_projection', { projectId })).tasks.find(t => t.id === task.data.taskId)?.state, { timeout: 30_000 }).toBe('review_ready');
  const projection = await rpc<{ tasks: Array<{ id: string; state: string; boardDrafts: Array<{ id: string }> }> }>('get_agent_projection', { projectId });
  const complete = projection.tasks.find(t => t.id === task.data.taskId)!;
  expect(complete.state).toBe('review_ready'); const draft = complete.boardDrafts[0].id;
  const comparison = await rpc<CanvasComparison & { draft: { revision: number } }>('canvas_comparison', { projectId, boardId: draft });
  const original = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  await page.goto('/#/project/' + projectId);
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await page.getByRole('combobox', { name: '选择画布' }).selectOption(draft);
  await page.getByRole('button', { name: '审查改动', exact: true }).click();
  return { projectId, source, draft, comparison, original, errors, outside };
}

test('真实 Store 审稿：三模式、单项采纳、整组撤销重做和重启一致', async ({ page }) => {
  const { projectId, source, draft, comparison, original, errors, outside } = await prepare(page);
  const dialog = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
  await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
  const directory = evidenceDirectory('editorial-review'); await mkdir(directory, { recursive: true });
  const photoChange = comparison.changes.find(c => c.nodeType === 'photo')!;
  const index = comparison.changes.indexOf(photoChange);
  await dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  await dialog.getByRole('button', { name: '适合窗口', exact: true }).click();
  await expect.poll(() => dialog.locator('.canvas-photo-node img').evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.screenshot({ path: resolve(directory, 'selected-preview.png') });
  await dialog.getByRole('button', { name: '并排对比', exact: true }).click();
  await page.screenshot({ path: resolve(directory, 'side-by-side.png') });
  await dialog.getByRole('button', { name: '差异叠层', exact: true }).click();
  await page.screenshot({ path: resolve(directory, 'overlay.png') });
  await page.setViewportSize({ width: 900, height: 800 }); await page.screenshot({ path: resolve(directory, 'compact.png') });
  await page.setViewportSize({ width: 390, height: 844 }); await page.screenshot({ path: resolve(directory, 'mobile.png') });
  await page.setViewportSize({ width: 1440, height: 960 });
  expect(await rpc('canvas_snapshot', { projectId, boardId: source })).toEqual(original);
  await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  await expect(dialog.locator('.editorial-change').filter({ hasText: '已采纳' })).toBeVisible();
  await expect(dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true })).toBeDisabled();
  await dialog.getByRole('button', { name: '查看原画布', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('combobox', { name: '选择画布' })).toHaveValue(source);
  const applied = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  expect(applied.board.revision).toBe(original.board.revision + 1);
  expect(applied.nodes.find(n => n.id === photoChange.sourceNodeId)?.x).toBe(photoChange.after!.x);
  const untouched = original.nodes.find(n => n.nodeType === 'sticky')!;
  expect(applied.nodes.find(n => n.id === untouched.id)).toEqual(untouched);
  expect((await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: draft })).board.owning_task_id).not.toBeNull();
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect.poll(async () => (await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source })).nodes.find(n => n.id === photoChange.sourceNodeId)?.x).toBe(photoChange.before!.x);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect.poll(async () => (await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source })).nodes.find(n => n.id === photoChange.sourceNodeId)?.x).toBe(photoChange.after!.x);
  await page.goto('about:blank'); await rpc('test_process_restart');
  const restored = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source }); expect(restored.nodes.map(n => ({ id: n.id, x: n.x, y: n.y }))).toEqual(applied.nodes.map(n => ({ id: n.id, x: n.x, y: n.y })));
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});

test('真实采纳回执丢失：原命令重试不重复应用，选择在重新打开后保留', async ({ page }) => {
  const { projectId, source, original, comparison } = await prepare(page);
  const photo = comparison.changes.find(c => c.nodeType === 'photo')!; const index = comparison.changes.indexOf(photo);
  await page.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  let lost = false;
  await page.route('http://localhost:1447/rpc', async route => {
    const data = route.request().postDataJSON();
    if (!lost && data.command === 'canvas_execute' && data.args.request.operations[0].action.type === 'ApplyEditorialChanges') {
      lost = true; const result = await (await route.fetch()).json(); expect(result.ok).toBe(true);
      await route.fulfill({ json: { ok: false, error: { code: 'DATABASE', message: 'lost after commit', retriable: true } } });
    } else await route.fallback();
  });
  await page.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  await expect(page.getByText(/尚未确认画布操作结果/)).toBeVisible();
  await page.getByRole('button', { name: '关闭对话框', exact: true }).click();
  await page.getByRole('button', { name: /^确认上次操作/ }).click();
  await expect(page.getByRole('button', { name: /^确认上次操作/ })).toHaveCount(0);
  const result = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source }); expect(result.board.revision).toBe(original.board.revision + 1);
  expect(result.nodes.find(n => n.id === photo.sourceNodeId)?.x).toBe(photo.after!.x);
});

test('真实三方冲突、连续分批采纳与拒绝决定在进程重启后保留', async ({ page }) => {
  const { projectId, source, draft, comparison, original, errors, outside } = await prepare(page);
  await page.getByRole('button', { name: '关闭对话框', exact: true }).click();
  const photo = comparison.changes.find(c => c.nodeType === 'photo')!;
  await rpc('canvas_execute', { request: { schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(), projectId, boardId: source, baseRevision: original.board.revision,
    operations: [{ opId: randomUUID(), action: { type: 'MoveNodes', data: { boardId: source, nodeIds: [photo.sourceNodeId], dx: 15, dy: 0 } } }] } });
  await page.reload();
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await page.getByRole('combobox', { name: '选择画布' }).selectOption(draft);
  await page.getByRole('button', { name: '审查改动', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
  const index = comparison.changes.indexOf(photo);
  await dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  await expect(dialog.getByRole('region', { name: '选择冲突处理方式' })).toBeVisible();
  await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
  const directory = evidenceDirectory('editorial-review'); await mkdir(directory, { recursive: true });
  await page.screenshot({ path: resolve(directory, 'attribute-conflict.png') });
  await dialog.getByRole('button', { name: '使用建议', exact: true }).click();
  await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  await expect(dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true })).toBeDisabled();
  const secondIndex = comparison.changes.findIndex(c => c.kind === 'change' && c.nodeType === 'sticky');
  expect(secondIndex).toBeGreaterThanOrEqual(0);
  await dialog.getByRole('checkbox', { name: `选择改动 ${secondIndex + 1}`, exact: true }).check();
  await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  await expect(dialog.getByRole('checkbox', { name: `选择改动 ${secondIndex + 1}`, exact: true })).toBeDisabled();
  const lastIndex = comparison.changes.findIndex(c => c.kind === 'add' && c.nodeType === 'sticky');
  expect(lastIndex).toBeGreaterThanOrEqual(0);
  await dialog.getByRole('checkbox', { name: `选择改动 ${lastIndex + 1}`, exact: true }).check();
  await dialog.getByRole('button', { name: '不采纳', exact: true }).click();
  await expect(dialog.getByText(/未采纳/)).toBeVisible();
  await page.screenshot({ path: resolve(directory, 'successive-decisions.png') });
  const beforeRestart = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  expect(beforeRestart.board.revision).toBe(original.board.revision + 3);
  const decisions = await rpc<CanvasComparison>('canvas_comparison', { projectId, boardId: draft });
  expect(decisions.changes.filter(c => c.decision === 'accepted')).toHaveLength(2);
  expect(decisions.changes[lastIndex].decision).toBe('rejected');
  await page.goto('about:blank'); await rpc('test_process_restart');
  expect(await rpc('canvas_comparison', { projectId, boardId: draft })).toEqual(decisions);
  expect(await rpc('canvas_snapshot', { projectId, boardId: source })).toEqual(beforeRestart);
  await page.goto('/#/project/' + projectId);
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await page.getByRole('combobox', { name: '选择画布' }).selectOption(source);
  await page.getByRole('button', { name: '采纳历史', exact: true }).click();
  const history = page.getByRole('dialog', { name: '采纳历史', exact: true });
  await expect(history.getByRole('button', { name: '预览撤回', exact: true })).toHaveCount(2);
  await history.getByRole('button', { name: '预览撤回', exact: true }).last().click();
  await expect(history.getByRole('button', { name: '确认撤回', exact: true })).toBeEnabled();
  expect(await rpc('canvas_snapshot', { projectId, boardId: source })).toEqual(beforeRestart);
  await page.screenshot({ path: resolve(directory, 'historical-revert-preview.png') });
  await history.getByRole('button', { name: '确认撤回', exact: true }).click();
  await expect(history.getByText(/已撤回/)).toBeVisible();
  const reversed = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  expect(reversed.nodes.find(n => n.id === photo.sourceNodeId)!.x).toBe(photo.before!.x + 15);
  const second = comparison.changes[secondIndex];
  expect(reversed.nodes.find(n => n.id === second.sourceNodeId)!.x).toBe(beforeRestart.nodes.find(n => n.id === second.sourceNodeId)!.x);
  expect(reversed.board.revision).toBe(beforeRestart.board.revision + 1);
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});

test('真实关联删除：先确认分组与连接影响，再整组采纳、撤销、重做和重启',async({page})=>{
  const {projectId,source,draft,comparison,original,errors,outside}=await prepare(page,false,'画布结构移除验收');
  await page.getByRole('button',{name:'关闭对话框',exact:true}).click();
  const photo=comparison.changes.find(c=>c.kind==='remove'&&c.nodeType==='photo')!;expect(photo).toBeTruthy();
  const sourcePhoto=photo.sourceNodeId!;const other=original.nodes.find(n=>n.id!==sourcePhoto)!;
  const groupId=randomUUID(),arrowId=randomUUID();
  await rpc('canvas_execute',{request:{schemaVersion:3,commandId:randomUUID(),idempotencyKey:randomUUID(),projectId,boardId:source,baseRevision:original.board.revision,operations:[
    {opId:groupId,action:{type:'GroupNodes',data:{boardId:source,nodeIds:[sourcePhoto,other.id],label:'保留的组合'}}},
    {opId:arrowId,action:{type:'AddArrow',data:{boardId:source,fromNodeId:sourcePhoto,toNodeId:other.id}}},
    {opId:randomUUID(),action:{type:'ResizeNode',data:{boardId:source,nodeId:sourcePhoto,width:300,height:210}}},
  ]}});
  const before=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});const hashes=await rpc('test_fixture_checksums');
  await page.reload();await page.getByRole('button',{name:'叙事画布',exact:true}).click();await page.getByRole('combobox',{name:'选择画布'}).selectOption(draft);await page.getByRole('button',{name:'审查改动',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'智能编辑审稿',exact:true});
  await dialog.getByRole('checkbox',{name:`选择改动 ${comparison.changes.indexOf(photo)+1}`,exact:true}).check();
  const conflicts=dialog.getByRole('region',{name:'处理内容与关联变化'});await expect(conflicts).toBeVisible();
  await conflicts.getByText('一同调整的内容 · 2',{exact:true}).click();await expect(conflicts.getByText(/移除失去端点的连接线/)).toBeVisible();
  await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeDisabled();expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(before);
  const directory=evidenceDirectory('structural-merge');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'related-removal-confirmation.png')});
  await conflicts.getByRole('button',{name:'确认移除并整理关联',exact:true}).click();await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeEnabled();
  expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(before);
  await dialog.getByRole('button',{name:'采纳选中改动',exact:true}).click();await dialog.getByRole('button',{name:'查看原画布',exact:true}).click();
  const applied=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});expect(applied.nodes.some(n=>n.id===sourcePhoto||n.id===arrowId)).toBe(false);expect(JSON.parse(applied.nodes.find(n=>n.id===groupId)!.childrenJson!)).toEqual([other.id]);
  await page.getByRole('button',{name:'撤销',exact:true}).click();await expect.poll(async()=>(await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.length).toBe(before.nodes.length);
  expect((await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.find(n=>n.id===sourcePhoto)!.width).toBe(300);
  await page.getByRole('button',{name:'重做',exact:true}).click();await expect.poll(async()=>(await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.length).toBe(applied.nodes.length);
  const redone=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});
  expect(redone.nodes.map(node=>({...node,updatedAt:undefined}))).toEqual(applied.nodes.map(node=>({...node,updatedAt:undefined})));
  await page.goto('about:blank');await rpc('test_process_restart');expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(redone);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
});

test('真实原内容已移除：确认后恢复稳定身份，拒绝不会偷偷复原',async({page})=>{
  const {projectId,source,draft,comparison,original,errors,outside}=await prepare(page);
  await page.getByRole('button',{name:'关闭对话框',exact:true}).click();
  const photo=comparison.changes.find(c=>c.nodeType==='photo'&&c.kind==='change')!;
  await rpc('canvas_execute',{request:{schemaVersion:3,commandId:randomUUID(),idempotencyKey:randomUUID(),projectId,boardId:source,baseRevision:original.board.revision,operations:[{opId:randomUUID(),action:{type:'DeleteNode',data:{boardId:source,nodeId:photo.sourceNodeId}}}]}});
  const deleted=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});
  await page.reload();await page.getByRole('button',{name:'叙事画布',exact:true}).click();await page.getByRole('combobox',{name:'选择画布'}).selectOption(draft);await page.getByRole('button',{name:'审查改动',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'智能编辑审稿',exact:true});const checkbox=dialog.getByRole('checkbox',{name:`选择改动 ${comparison.changes.indexOf(photo)+1}`,exact:true});await checkbox.check();
  const conflicts=dialog.getByRole('region',{name:'处理内容与关联变化'});await expect(conflicts.getByText(/原内容已移除/)).toBeVisible();await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeDisabled();
  await conflicts.getByRole('button',{name:'保留当前，不采纳此项',exact:true}).click();await expect(dialog.locator('.editorial-change').filter({hasText:'未采纳'})).toBeVisible();expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(deleted);
  await checkbox.check();await conflicts.getByRole('button',{name:'按建议恢复',exact:true}).click();await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeEnabled();
  const directory=evidenceDirectory('structural-merge');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'restore-confirmation.png')});
  await dialog.getByRole('button',{name:'采纳选中改动',exact:true}).click();await dialog.getByRole('button',{name:'查看原画布',exact:true}).click();
  const restored=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});const node=restored.nodes.find(n=>n.id===photo.sourceNodeId)!;expect(node.assetId).toBe(photo.before!.assetId);expect(node.x).toBe(photo.after!.x);expect(restored.nodes.length).toBe(original.nodes.length);
  await page.getByRole('button',{name:'撤销',exact:true}).click();await expect.poll(async()=>(await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.some(n=>n.id===photo.sourceNodeId)).toBe(false);expect(errors).toEqual([]);expect(outside).toEqual([]);
});

test('真实讨论与修订：批注回复解决重开、重启保留、用户发送后创建独立新版本', async ({ page }) => {
  const { projectId,source,draft,errors,outside } = await prepare(page,true);
  const parent = await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:draft});
  const official = await rpc('get_project',{projectId});
  const review = page.getByRole('dialog',{name:'智能编辑审稿',exact:true});
  const discussion = review.getByRole('region',{name:'批注与讨论'});
  await discussion.getByRole('textbox',{name:'写下修改意见'}).fill('保留开篇，中段需要观看距离的变化');
  await discussion.getByRole('button',{name:'保存批注',exact:true}).click();
  await expect(discussion.getByText('保留开篇，中段需要观看距离的变化',{exact:true})).toBeVisible();
  await discussion.getByRole('button',{name:'回复',exact:true}).click();
  await discussion.getByRole('textbox',{name:'回复内容'}).fill('请用空镜作为中段过渡');
  await discussion.getByRole('textbox',{name:'回复内容'}).locator('..').getByRole('button',{name:'保存批注',exact:true}).click();
  await expect(discussion.getByText('请用空镜作为中段过渡',{exact:true})).toBeVisible();
  await discussion.getByRole('button',{name:'标记已解决'}).click();
  await expect(discussion.getByText('已解决',{exact:true})).toBeVisible();
  await discussion.getByRole('button',{name:'重新讨论'}).click();
  await expect(discussion.getByRole('button',{name:'回复',exact:true})).toBeEnabled();
  expect(await rpc('canvas_snapshot',{projectId,boardId:draft})).toEqual(parent);
  expect(await rpc('get_project',{projectId})).toEqual(official);
  const directory = evidenceDirectory('discussion'); await mkdir(directory,{recursive:true});
  await discussion.scrollIntoViewIfNeeded(); await page.screenshot({path:resolve(directory,'discussion-and-reply.png')});
  await page.goto('about:blank'); await rpc('test_process_restart');
  await page.goto('/#/project/'+projectId); await page.getByRole('button',{name:'叙事画布',exact:true}).click();
  await page.getByRole('combobox',{name:'选择画布'}).selectOption(draft); await page.getByRole('button',{name:'审查改动',exact:true}).click();
  const restored = page.getByRole('region',{name:'批注与讨论'});
  await expect(restored.getByText('请用空镜作为中段过渡',{exact:true})).toBeVisible();
  const prior = await rpc<{tasks:{id:string;state:string;boardDrafts:{id:string}[]}[]}>('get_agent_projection',{projectId});
  expect(prior.tasks).toHaveLength(1);
  await restored.getByRole('button',{name:'根据批注继续修改'}).click();
  const ai = page.getByRole('dialog',{name:'Focal AI',exact:true});
  await expect(ai.getByRole('button',{name:'取消方案引用'})).toBeVisible();
  expect((await rpc<typeof prior>('get_agent_projection',{projectId})).tasks).toHaveLength(1);
  await ai.getByRole('button',{name:'发送',exact:true}).click();
  await expect(ai.getByText('新的修订方案已准备好，原方案保留',{exact:true})).toBeVisible({timeout:30000});
  const result=await rpc<typeof prior>('get_agent_projection',{projectId});
  expect(result.tasks).toHaveLength(2); const child = result.tasks[0].boardDrafts[0].id; expect(child).not.toBe(draft);
  const modified=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:child});
  expect(modified.nodes.some(n => n.contentJson?.includes('请用空镜作为中段过渡'))).toBe(true);
  expect(await rpc('canvas_snapshot',{projectId,boardId:draft})).toEqual(parent);
  expect(await rpc('get_project',{projectId})).toEqual(official);
  const comments=await rpc<{messages:{resolved:boolean}[]}>('canvas_editorial_discussion',{projectId,draftId:draft,revision:parent.board.revision,offset:0,cursor:null});
  expect(comments.messages).toHaveLength(2); expect(comments.messages.every(m=>!m.resolved)).toBe(true);
  await page.screenshot({path:resolve(directory,'review-to-independent-revision.png')});
  // 修订依据取委托时的父版快照；晚到的意见仍留在父版，不重锚到新方案。
  await rpc('canvas_execute',{request:{schemaVersion:3,commandId:randomUUID(),idempotencyKey:randomUUID(),projectId,boardId:draft,baseRevision:parent.board.revision,operations:[{opId:randomUUID(),action:{type:'AddEditorialComment',data:{draftId:draft,draftRevision:parent.board.revision,changeId:null,threadId:null,body:'晚到意见，留给下一次修订'}}}]}});
  const context = await rpc<{discussion:{totalMessages:number;messages:{body:string}[]}}>('canvas_editorial_revision_context',{projectId,draftId:child,offset:0});
  expect(context.discussion.totalMessages).toBe(2);expect(context.discussion.messages.some(m=>m.body.includes('晚到'))).toBe(false);
  await page.goto('about:blank');await rpc('test_process_restart');
  await page.goto('/#/project/'+projectId);await page.getByRole('button',{name:'叙事画布',exact:true}).click();
  await page.getByRole('combobox',{name:'选择画布'}).selectOption(child);await page.getByRole('button',{name:'审查改动',exact:true}).click();
  const revisionContext=page.locator('.editorial-revision-context');await revisionContext.locator('summary').click();
  await expect(revisionContext.getByText('请用空镜作为中段过渡',{exact:true})).toBeVisible();
  await expect(revisionContext.getByText('晚到意见，留给下一次修订',{exact:true})).toHaveCount(0);
  await expect(revisionContext.getByRole('textbox')).toHaveCount(0);
  await revisionContext.scrollIntoViewIfNeeded();
  const contextDirectory=evidenceDirectory('cross-round-context');await mkdir(contextDirectory,{recursive:true});
  await page.screenshot({path:resolve(contextDirectory,'frozen-parent-context.png')});
  expect(await rpc('canvas_snapshot',{projectId,boardId:draft})).toEqual(parent);
  expect(await rpc('get_project',{projectId})).toEqual(official);
  expect((await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).board.revision).toBeGreaterThan(0);
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});
