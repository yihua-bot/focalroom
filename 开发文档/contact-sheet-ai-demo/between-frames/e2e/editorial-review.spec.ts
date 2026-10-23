import { test, expect, type Page } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { defaultAgentBudget } from '../src/domain/agent';
import type { CanvasComparison, CanvasPhoto } from '../src/domain/canvas';

const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  const result = await fetch('http://localhost:1447/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!result.ok) throw new Error(JSON.stringify(result.error)); return result.value;
};
type Snapshot = { board: { revision: number; owning_task_id: string | null }; nodes: CanvasPhoto[] };
async function prepare(page: Page) {
  const errors: string[] = [], outside: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    outside.push(route.request().url()); return route.abort();
  });
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
    type: 'StartCanvasTask', goal: '保留原画布，尝试空间编排', selectedPhotoIds: [], targetCount: 1, budget: defaultAgentBudget, consentId: null, sourceBoard: { boardId: source, revision, selectedNodeIds: [] },
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
  const directory = resolve('../docs/v3/evidence/editorial-review-2026-10-23/browser'); await mkdir(directory, { recursive: true });
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
