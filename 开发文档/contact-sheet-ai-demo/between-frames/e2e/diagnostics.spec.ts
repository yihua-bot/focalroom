// M5 诊断使用桌面同一 Rust 投影，不依赖真实模型或钥匙串写删。
import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import type { Diagnostics } from '../src/domain/diagnostics';

async function rpc<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const reply = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!reply.ok) throw new Error(reply.error.message);
  return reply.value;
}
test('M5 本地诊断：真实统计、查看后复制、刷新及正式状态只读', async ({ page, context }) => {
  const screenshots = resolve('../docs/screenshots/m5-storage-diagnostics');
  await mkdir(screenshots, { recursive: true });
  const errors: string[] = []; const outside: string[] = [];
  page.on('pageerror', e => errors.push(e.message));
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
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
  const project = await rpc<{ id: string }>('create_project', { title: 'M5 · 私密项目名称不进入诊断', brief: '私密意图不进入诊断' });
  const before = await rpc('get_project', { projectId: project.id });
  const hashes = await rpc('test_fixture_checksums');
  await page.goto('/#/project/' + project.id);
  await page.getByRole('button', { name: 'Agent · 本地 Mock', exact: true }).click();
  await page.getByText('模型与密钥 · 诊断', { exact: true }).click();
  await page.getByRole('button', { name: '查看本地存储与诊断' }).click();
  const dialog = page.getByRole('dialog', { name: '本地存储与诊断' });
  await expect(dialog.getByRole('heading', { name: '存储统计' })).toBeVisible();
  await expect(dialog.getByRole('row', { name: /图片缓存/ })).toBeVisible();
  await dialog.getByText('检查诊断 JSON', { exact: true }).click();
  const shown = JSON.parse(await dialog.getByLabel('脱敏诊断 JSON').textContent() ?? '{}') as Diagnostics;
  expect(shown.schemaVersion).toBe(1);
  expect(shown.storage.database.bytes).toBeGreaterThan(0);
  expect(shown.storage.complete).toBe(true);
  expect(shown.provider.providerId).toBe('mock');
  expect(shown.migrationsApplied).toBe(16);
  expect(shown.database.projects).toBeGreaterThanOrEqual(1);
  expect(JSON.stringify(shown)).not.toMatch(/私密|originalPath|apiKeyTail|baseUrl|\/Users\//);
  await dialog.getByRole('button', { name: '复制诊断信息' }).click();
  await expect(dialog.getByText('诊断信息已复制，可自行检查后用于反馈。')).toBeVisible();
  expect(JSON.parse(await page.evaluate(() => navigator.clipboard.readText()))).toEqual(shown);
  await page.screenshot({ path: resolve(screenshots, 'view-and-copy.png') });
  await dialog.getByRole('button', { name: '刷新统计' }).click();
  await expect(dialog.getByText('正在读取本地统计…')).toHaveCount(0);
  const refreshed = JSON.parse(await dialog.getByLabel('脱敏诊断 JSON').textContent() ?? '{}') as Diagnostics;
  expect(refreshed.database).toEqual(shown.database);
  expect(await rpc('get_project', { projectId: project.id })).toEqual(before);
  expect(await rpc('test_fixture_checksums')).toEqual(hashes);
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('button', { name: '查看本地存储与诊断' })).toBeFocused();
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});
