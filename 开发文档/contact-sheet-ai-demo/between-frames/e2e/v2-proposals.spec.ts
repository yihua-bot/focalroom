import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import type { Project } from '../src/domain/models';
const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  const result = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!result.ok) throw new Error(result.error.message);
  return result.value;
};

test('v2 真实后端提案：GUI 提交、批量审批、单组撤销重做、重启与基线冲突', async ({ page }) => {
  const errors: string[] = [];
  const outside: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    outside.push(route.request().url()); return route.abort('internetdisconnected');
  });
  await page.addInitScript(() => {
    Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
      invoke: async (command: string, args: unknown) => {
        if (command.startsWith('plugin:event|')) return 0;
        const r = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
        if (!r.ok) throw r.error;
        return r.value;
      },
      convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
    } });
  });
  let project = await rpc<Project>('create_project', { title: 'v2 · 审批事务验收', brief: '按作者批准的位置编排' });
  const token = await rpc<string>('choose_import', { projectId: project.id });
  const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken: token });
  await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100000 }).toBe('completed');
  project = await rpc<Project>('get_project', { projectId: project.id });
  const [a, b] = project.photos;
  const create = async (title: string) => {
    const envelope = { version: '2.0', command_id: crypto.randomUUID(), workspace_id: 'local-workspace', project_id: project.id, expected_revision: project.revision, idempotency_key: crypto.randomUUID(), kind: 'proposal.create', payload: { base_revision: project.revision, title, operations: [
      { operation_id: 'add-a', photo_id: a.id, op: 'add', to_index: 0 },
      { operation_id: 'add-b', photo_id: b.id, op: 'add', to_index: 0 },
    ] } };
    const result = await rpc<{ ok: boolean }>('execute_v2_command', { envelope, principal: 'local-owner' });
    expect(result.ok).toBe(true);
  };
  await create('逆序开篇');
  await page.goto('/#/project/' + project.id);
  await page.getByRole('main').getByRole('button', { name: '编辑建议', exact: true }).click();
  const panel = page.locator('.proposals-panel');
  await panel.getByRole('button', { name: /逆序开篇/ }).click();
  await panel.getByRole('button', { name: '提交审批' }).click();
  await expect(panel.getByRole('button', { name: '执行所选操作' })).toBeVisible();
  await panel.getByRole('button', { name: '执行所选操作' }).click();
  await expect(panel.getByRole('status')).toHaveText('已执行 2 个操作');
  await expect.poll(async () => (await rpc<Project>('get_project', { projectId: project.id })).sequence).toEqual([b.id, a.id]);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect.poll(async () => (await rpc<Project>('get_project', { projectId: project.id })).sequence).toEqual([]);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect.poll(async () => (await rpc<Project>('get_project', { projectId: project.id })).sequence).toEqual([b.id, a.id]);
  await rpc('test_restart');
  await page.reload();
  await page.getByRole('main').getByRole('button', { name: '编辑建议', exact: true }).click();
  await panel.getByRole('button', { name: /逆序开篇/ }).click();
  await expect(panel.getByText('该提案已执行（可在工作台撤销）。')).toBeVisible();
  const screenshots = resolve('../docs/screenshots/v2-review-fixes');
  await mkdir(screenshots, { recursive: true });
  await panel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: resolve(screenshots, 'proposal-applied.png') });
  project = await rpc<Project>('get_project', { projectId: project.id });
  await create('冲突提案');
  await page.reload();
  await page.getByRole('main').getByRole('button', { name: '编辑建议', exact: true }).click();
  await panel.getByRole('button', { name: /冲突提案/ }).click();
  await panel.getByRole('button', { name: '提交审批' }).click();
  await expect(panel.getByRole('button', { name: '执行所选操作' })).toBeVisible();
  await rpc('execute_gui_command', { request: { commandId: crypto.randomUUID(), projectId: project.id, expectedProjectRevision: project.revision, causationId: null, idempotencyKey: crypto.randomUUID(), command: { type: 'SetPhotoNote', photoId: a.id, note: '审批前作者更新了判断' } } });
  await panel.getByRole('button', { name: '执行所选操作' }).click();
  await expect(panel.getByRole('status')).toContainText('重建提案');
  expect(await rpc<Project>('get_project', { projectId: project.id }).then(p => p.sequence)).toEqual([b.id, a.id]);
  await panel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: resolve(screenshots, 'proposal-conflict.png') });
  expect(errors).toEqual([]);
  expect(outside).toEqual([]);
});
