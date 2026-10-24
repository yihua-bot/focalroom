# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: editorial-review.spec.ts >> 真实关联删除：先确认分组与连接影响，再整组采纳、撤销、重做和重启
- Location: e2e/editorial-review.spec.ts:186:1

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: "review_ready"
Received: "failed"

Call Log:
- Timeout 30000ms exceeded while waiting on the predicate
```

# Test source

```ts
  1   | import { test, expect, type Page } from '@playwright/test';
  2   | import { randomUUID } from 'node:crypto';
  3   | import { mkdir } from 'node:fs/promises';
  4   | import { resolve } from 'node:path';
  5   | import { defaultAgentBudget } from '../src/domain/agent';
  6   | import type { CanvasComparison, CanvasPhoto } from '../src/domain/canvas';
  7   | import { installAccountFixture } from './account-fixture';
  8   | import { evidenceDirectory } from './evidence-directory';
  9   | 
  10  | const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  11  |   const result = await fetch('http://localhost:1447/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  12  |   if (!result.ok) throw new Error(JSON.stringify(result.error)); return result.value;
  13  | };
  14  | type Snapshot = { board: { revision: number; owning_task_id: string | null }; nodes: CanvasPhoto[] };
  15  | async function prepare(page: Page, authenticated = false, goal = '保留原画布，尝试空间编排') {
  16  |   const errors: string[] = [], outside: string[] = [];
  17  |   page.on('pageerror', error => errors.push(error.message));
  18  |   await page.route('**/*', route => {
  19  |     if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
  20  |     outside.push(route.request().url()); return route.abort();
  21  |   });
  22  |   if (authenticated) await installAccountFixture(page);
  23  |   await page.addInitScript(() => {
  24  |     if (location.origin === 'http://localhost:1446') localStorage.setItem('focalroom-locale', 'zh');
  25  |     Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
  26  |       invoke: async (command: string, args: unknown) => {
  27  |         if (command.startsWith('plugin:event|') || command === 'set_native_menu_locale') return 0;
  28  |         const result = await fetch('http://localhost:1447/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  29  |         if (!result.ok) throw result.error; return result.value;
  30  |       }, convertFileSrc: (path: string) => 'http://localhost:1447/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
  31  |     } });
  32  |   });
  33  |   const created = await rpc<{ id: string }>('create_project', { title: '编辑审稿 ' + randomUUID().slice(0, 6), brief: '保留原画布并比较空间编排' });
  34  |   const projectId = created.id;
  35  |   const selectionToken = await rpc<string>('choose_import', { projectId });
  36  |   const jobId = await rpc<string>('import_photos', { projectId, selectionToken });
  37  |   await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100_000 }).toBe('completed');
  38  |   const project = await rpc<{ photos: { id: string }[] }>('get_project', { projectId });
  39  |   const source = randomUUID();
  40  |   const execute = async (baseRevision: number, actions: unknown[]) => rpc<{ revision: number }>('canvas_execute', { request: { schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(), projectId, boardId: source, baseRevision, operations: actions.map(action => ({ opId: randomUUID(), action })) } });
  41  |   await execute(0, [{ type: 'CreateBoard', data: { title: `审稿原版 ${source.slice(0, 6)}` } }]);
  42  |   const { revision } = await execute(1, [
  43  |     { type: 'PlacePhoto', data: { boardId: source, assetId: project.photos[0].id, x: 20, y: 20, width: 220, height: 160 } },
  44  |     { type: 'AddText', data: { boardId: source, nodeType: 'sticky', x: 300, y: 40, width: 200, height: 100, content: '保留观看中的停顿' } },
  45  |   ]);
  46  |   const task = await rpc<{ ok: boolean; data: { taskId: string }; error: unknown }>('agent_request', { request: { requestId: randomUUID(), projectId, command: {
  47  |     type: authenticated ? 'StartConversation' : 'StartCanvasTask', ...(authenticated ? { expectedAccountId:null,focusPhotoIds:[],continuation:null } : {}), goal, selectedPhotoIds: [], targetCount: 1, budget: defaultAgentBudget, consentId: null, sourceBoard: { boardId: source, revision, selectedNodeIds: [] },
  48  |   } } });
  49  |   expect(task.ok, JSON.stringify(task.error)).toBe(true);
> 50  |   await expect.poll(async () => (await rpc<{ tasks: Array<{ id: string; state: string }> }>('get_agent_projection', { projectId })).tasks.find(t => t.id === task.data.taskId)?.state, { timeout: 30_000 }).toBe('review_ready');
      |                                                                                                                                                                                                             ^ Error: expect(received).toBe(expected) // Object.is equality
  51  |   const projection = await rpc<{ tasks: Array<{ id: string; state: string; boardDrafts: Array<{ id: string }> }> }>('get_agent_projection', { projectId });
  52  |   const complete = projection.tasks.find(t => t.id === task.data.taskId)!;
  53  |   expect(complete.state).toBe('review_ready'); const draft = complete.boardDrafts[0].id;
  54  |   const comparison = await rpc<CanvasComparison & { draft: { revision: number } }>('canvas_comparison', { projectId, boardId: draft });
  55  |   const original = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  56  |   await page.goto('/#/project/' + projectId);
  57  |   await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  58  |   await page.getByRole('combobox', { name: '选择画布' }).selectOption(draft);
  59  |   await page.getByRole('button', { name: '审查改动', exact: true }).click();
  60  |   return { projectId, source, draft, comparison, original, errors, outside };
  61  | }
  62  | 
  63  | test('真实 Store 审稿：三模式、单项采纳、整组撤销重做和重启一致', async ({ page }) => {
  64  |   const { projectId, source, draft, comparison, original, errors, outside } = await prepare(page);
  65  |   const dialog = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
  66  |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
  67  |   const directory = evidenceDirectory('editorial-review'); await mkdir(directory, { recursive: true });
  68  |   const photoChange = comparison.changes.find(c => c.nodeType === 'photo')!;
  69  |   const index = comparison.changes.indexOf(photoChange);
  70  |   await dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  71  |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  72  |   await dialog.getByRole('button', { name: '适合窗口', exact: true }).click();
  73  |   await expect.poll(() => dialog.locator('.canvas-photo-node img').evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  74  |   await page.screenshot({ path: resolve(directory, 'selected-preview.png') });
  75  |   await dialog.getByRole('button', { name: '并排对比', exact: true }).click();
  76  |   await page.screenshot({ path: resolve(directory, 'side-by-side.png') });
  77  |   await dialog.getByRole('button', { name: '差异叠层', exact: true }).click();
  78  |   await page.screenshot({ path: resolve(directory, 'overlay.png') });
  79  |   await page.setViewportSize({ width: 900, height: 800 }); await page.screenshot({ path: resolve(directory, 'compact.png') });
  80  |   await page.setViewportSize({ width: 390, height: 844 }); await page.screenshot({ path: resolve(directory, 'mobile.png') });
  81  |   await page.setViewportSize({ width: 1440, height: 960 });
  82  |   expect(await rpc('canvas_snapshot', { projectId, boardId: source })).toEqual(original);
  83  |   await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  84  |   await expect(dialog.locator('.editorial-change').filter({ hasText: '已采纳' })).toBeVisible();
  85  |   await expect(dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true })).toBeDisabled();
  86  |   await dialog.getByRole('button', { name: '查看原画布', exact: true }).click();
  87  |   await expect(dialog).toHaveCount(0);
  88  |   await expect(page.getByRole('combobox', { name: '选择画布' })).toHaveValue(source);
  89  |   const applied = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  90  |   expect(applied.board.revision).toBe(original.board.revision + 1);
  91  |   expect(applied.nodes.find(n => n.id === photoChange.sourceNodeId)?.x).toBe(photoChange.after!.x);
  92  |   const untouched = original.nodes.find(n => n.nodeType === 'sticky')!;
  93  |   expect(applied.nodes.find(n => n.id === untouched.id)).toEqual(untouched);
  94  |   expect((await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: draft })).board.owning_task_id).not.toBeNull();
  95  |   await page.getByRole('button', { name: '撤销', exact: true }).click();
  96  |   await expect.poll(async () => (await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source })).nodes.find(n => n.id === photoChange.sourceNodeId)?.x).toBe(photoChange.before!.x);
  97  |   await page.getByRole('button', { name: '重做', exact: true }).click();
  98  |   await expect.poll(async () => (await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source })).nodes.find(n => n.id === photoChange.sourceNodeId)?.x).toBe(photoChange.after!.x);
  99  |   await page.goto('about:blank'); await rpc('test_process_restart');
  100 |   const restored = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source }); expect(restored.nodes.map(n => ({ id: n.id, x: n.x, y: n.y }))).toEqual(applied.nodes.map(n => ({ id: n.id, x: n.x, y: n.y })));
  101 |   expect(errors).toEqual([]); expect(outside).toEqual([]);
  102 | });
  103 | 
  104 | test('真实采纳回执丢失：原命令重试不重复应用，选择在重新打开后保留', async ({ page }) => {
  105 |   const { projectId, source, original, comparison } = await prepare(page);
  106 |   const photo = comparison.changes.find(c => c.nodeType === 'photo')!; const index = comparison.changes.indexOf(photo);
  107 |   await page.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  108 |   let lost = false;
  109 |   await page.route('http://localhost:1447/rpc', async route => {
  110 |     const data = route.request().postDataJSON();
  111 |     if (!lost && data.command === 'canvas_execute' && data.args.request.operations[0].action.type === 'ApplyEditorialChanges') {
  112 |       lost = true; const result = await (await route.fetch()).json(); expect(result.ok).toBe(true);
  113 |       await route.fulfill({ json: { ok: false, error: { code: 'DATABASE', message: 'lost after commit', retriable: true } } });
  114 |     } else await route.fallback();
  115 |   });
  116 |   await page.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  117 |   await expect(page.getByText(/尚未确认画布操作结果/)).toBeVisible();
  118 |   await page.getByRole('button', { name: '关闭对话框', exact: true }).click();
  119 |   await page.getByRole('button', { name: /^确认上次操作/ }).click();
  120 |   await expect(page.getByRole('button', { name: /^确认上次操作/ })).toHaveCount(0);
  121 |   const result = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source }); expect(result.board.revision).toBe(original.board.revision + 1);
  122 |   expect(result.nodes.find(n => n.id === photo.sourceNodeId)?.x).toBe(photo.after!.x);
  123 | });
  124 | 
  125 | test('真实三方冲突、连续分批采纳与拒绝决定在进程重启后保留', async ({ page }) => {
  126 |   const { projectId, source, draft, comparison, original, errors, outside } = await prepare(page);
  127 |   await page.getByRole('button', { name: '关闭对话框', exact: true }).click();
  128 |   const photo = comparison.changes.find(c => c.nodeType === 'photo')!;
  129 |   await rpc('canvas_execute', { request: { schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(), projectId, boardId: source, baseRevision: original.board.revision,
  130 |     operations: [{ opId: randomUUID(), action: { type: 'MoveNodes', data: { boardId: source, nodeIds: [photo.sourceNodeId], dx: 15, dy: 0 } } }] } });
  131 |   await page.reload();
  132 |   await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  133 |   await page.getByRole('combobox', { name: '选择画布' }).selectOption(draft);
  134 |   await page.getByRole('button', { name: '审查改动', exact: true }).click();
  135 |   const dialog = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
  136 |   const index = comparison.changes.indexOf(photo);
  137 |   await dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  138 |   await expect(dialog.getByRole('region', { name: '选择冲突处理方式' })).toBeVisible();
  139 |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
  140 |   const directory = evidenceDirectory('editorial-review'); await mkdir(directory, { recursive: true });
  141 |   await page.screenshot({ path: resolve(directory, 'attribute-conflict.png') });
  142 |   await dialog.getByRole('button', { name: '使用建议', exact: true }).click();
  143 |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  144 |   await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  145 |   await expect(dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true })).toBeDisabled();
  146 |   const secondIndex = comparison.changes.findIndex(c => c.kind === 'change' && c.nodeType === 'sticky');
  147 |   expect(secondIndex).toBeGreaterThanOrEqual(0);
  148 |   await dialog.getByRole('checkbox', { name: `选择改动 ${secondIndex + 1}`, exact: true }).check();
  149 |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  150 |   await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
```