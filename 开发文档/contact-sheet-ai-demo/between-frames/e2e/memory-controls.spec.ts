// M4 本地记忆：浏览器界面驱动真实 Rust/SQLite，禁止外发。
import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import type { MemoryProjection } from '../src/domain/memory';

async function rpc<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const response = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!response.ok) throw new Error(response.error.message);
  return response.value;
}
type Project = { id: string; revision: number; sequence: string[]; photos: { id: string; status: string; note: string }[] };

test('M4 记忆控制：确认、反例、排除/恢复、移除/恢复、搜索及重启', async ({ page }) => {
  const screenshots = resolve('../docs/screenshots/m4-memory-controls');
  await mkdir(screenshots, { recursive: true });
  const errors: string[] = []; const outside: string[] = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.route('**/*', route => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    outside.push(route.request().url()); return route.abort();
  });
  await page.addInitScript(() => Object.assign(globalThis, {
    isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
    __TAURI_INTERNALS__: {
      invoke: async (command: string, args: unknown) => {
        if (command.startsWith('plugin:event|')) return 0;
        const reply = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
        if (!reply.ok) throw reply.error; return reply.value;
      },
      convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
    },
  }));
  let project = await rpc<Project>('create_project', { title: 'M4 · 选择有证据', brief: '人工证据可排除、假设可纠正与恢复。' });
  const hashes = await rpc<string[][]>('test_fixture_checksums');
  const selectionToken = await rpc<string>('choose_import', { projectId: project.id });
  const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken });
  await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100000 }).toBe('completed');
  project = await rpc<Project>('get_project', { projectId: project.id });
  for (const index of [0, 1]) project = await rpc<Project>('update_photo_note', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[index].id, text: '留白' });
  project = await rpc<Project>('update_photo_decision', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[2].id, status: 'reject' });
  project = await rpc<Project>('update_photo_note', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[2].id, text: '留白太多' });
  const formal = await rpc<Project>('get_project', { projectId: project.id });
  const initial = await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id });
  const h = initial.hypotheses.find(h => h.statement.includes('留白'))!;
  const support = h.evidenceIds[0];
  await page.goto('/#/project/' + project.id);
  await page.getByRole('button', { name: '审美记忆', exact: true }).last().click();
  const card = page.locator('.memory-hypothesis').filter({ hasText: h.statement });
  await expect(card).toContainText('证据 2 条 · 反例 1 条');
  await card.getByText('查看反例', { exact: true }).click();
  await expect(card).toContainText('留白太多');
  await card.getByRole('button', { name: '确认此假设' }).click();
  await expect(card).toHaveAttribute('data-status', 'confirmed');
  await page.screenshot({ path: resolve(screenshots, 'confirmed-counterexample.png') });
  await page.getByRole('button', { name: '排除证据 #' + support, exact: true }).click();
  await expect(card).toContainText('证据 1 条');
  await page.getByRole('button', { name: '恢复证据 #' + support, exact: true }).click();
  await expect(card).toContainText('证据 2 条');
  await card.getByRole('button', { name: '移除此假设' }).click();
  await expect(card).toHaveCount(0);
  await page.getByRole('checkbox', { name: '显示已移除假设' }).check();
  await expect(card).toHaveAttribute('data-status', 'retired');
  await card.getByRole('button', { name: '恢复此假设' }).click();
  await expect(card).toHaveAttribute('data-status', 'unconfirmed');
  await page.getByRole('textbox', { name: '搜索偏好关键词' }).fill('逆光');
  await page.getByRole('button', { name: '搜索', exact: true }).click();
  await expect(page.getByText('没有匹配的假设')).toBeVisible();
  await page.getByRole('textbox', { name: '搜索偏好关键词' }).fill('留白');
  await page.getByRole('button', { name: '搜索', exact: true }).click();
  await expect(card).toBeVisible();
  const field = await page.getByRole('textbox', { name: '搜索偏好关键词' }).boundingBox();
  const searchButton = await page.getByRole('button', { name: '搜索', exact: true }).boundingBox();
  expect(field).not.toBeNull(); expect(searchButton).not.toBeNull();
  expect(Math.abs(field!.y + field!.height - searchButton!.y - searchButton!.height)).toBeLessThan(2);
  expect(field!.x + field!.width).toBeLessThanOrEqual(searchButton!.x);
  await mkdir(resolve('../docs/evidence/ux-chat-drawer-2026-10-20'), { recursive: true });
  await page.screenshot({ path: resolve('../docs/evidence/ux-chat-drawer-2026-10-20/memory-search.png') });
  await page.screenshot({ path: resolve(screenshots, 'restored-search.png') });
  const memory = await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id });
  expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(formal);
  await rpc('test_restart');
  expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(formal);
  expect(await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id })).toEqual(memory);
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(hashes);
  await page.reload();
  await page.getByRole('button', { name: '审美记忆', exact: true }).last().click();
  await expect(card).toHaveAttribute('data-status', 'unconfirmed');
  // 作者记忆授权通过真实 Rust 命令，来源卡保持只读并跨进程重启保留。
  let source = await rpc<Project>('create_project', { title: '作者记忆来源', brief: '仅临时测试素材' });
  const sourceToken = await rpc<string>('choose_import', { projectId: source.id });
  const sourceJob = await rpc<string>('import_photos', { projectId: source.id, selectionToken: sourceToken });
  await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId: sourceJob })).status, { timeout: 100000 }).toBe('completed');
  source = await rpc<Project>('get_project', { projectId: source.id });
  for (const index of [0, 1]) source = await rpc<Project>('update_photo_note', { projectId: source.id, expectedRevision: source.revision, photoId: source.photos[index].id, text: '留白' });
  source = await rpc<Project>('get_project', { projectId: source.id });
  const sourceHypothesis = (await rpc<MemoryProjection>('get_memory_projection', { projectId: source.id })).hypotheses.find(h => h.statement.includes('留白'))!;
  const confirmed = await rpc<{ ok: boolean }>('execute_memory_command', { request: { requestId: crypto.randomUUID(), projectId: source.id, command: { type: 'ConfirmHypothesis', hypothesisId: sourceHypothesis.id } } });
  expect(confirmed.ok).toBe(true);
  const toggle = page.getByRole('checkbox', { name: '参考其他本地项目的偏好' });
  await expect(toggle).not.toBeChecked();
  page.once('dialog', dialog => dialog.dismiss());
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  page.once('dialog', dialog => dialog.accept());
  await toggle.click();
  await expect(toggle).toBeChecked();
  const author = page.locator('[data-author-memory="' + sourceHypothesis.id + '"]');
  await expect(author).toContainText(source.id);
  await expect(author.getByRole('link')).toHaveAttribute('href', '#/project/' + source.id);
  await expect(author.getByRole('button')).toHaveCount(0);
  await mkdir(resolve('../docs/evidence/v2-m3-author-memory'), { recursive: true });
  await author.scrollIntoViewIfNeeded();
  await page.screenshot({ path: resolve('../docs/evidence/v2-m3-author-memory/author-memory-ui.png') });
  await rpc('test_restart');
  await page.reload();
  await page.getByRole('button', { name: '审美记忆', exact: true }).last().click();
  await expect(toggle).toBeChecked();
  await expect(author).toBeVisible();
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect(author).toHaveCount(0);
  expect((await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id })).crossProjectEnabled).toBe(false);
  expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(formal);
  expect(await rpc<Project>('get_project', { projectId: source.id })).toEqual(source);
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(hashes);
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});
