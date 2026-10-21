import { test, expect } from '@playwright/test';

type Project = { id: string; photos: Array<{ id: string }>; revision: number };
async function rpc<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const response = await fetch('http://localhost:1422/rpc', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }),
  }).then(value => value.json());
  if (!response.ok) throw new Error(response.error.message);
  return response.value as T;
}

test('G3/G7 真实 Store：画布照片、便签、连接、出版生成与进程重启', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    const url = new URL(route.request().url());
    return ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname) ? route.continue() : route.abort();
  });
  await page.addInitScript(() => {
    Object.assign(globalThis, {
      isTauri: true,
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      __TAURI_INTERNALS__: {
        invoke: async (command: string, args: unknown) => {
          if (command.startsWith('plugin:event|')) return 0;
          const result = await fetch('http://localhost:1422/rpc', {
            method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }),
          }).then(response => response.json());
          if (!result.ok) throw result.error;
          return result.value;
        },
        convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path),
        transformCallback: () => 0,
      },
    });
  });
  await page.goto('/');
  await page.getByRole('button', { name: '新建项目', exact: true }).first().click();
  await page.getByRole('textbox', { name: '项目名称' }).fill('叙事与出版');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page).toHaveURL(/#\/project\/[^/]+$/);
  const projectId = new URL(page.url()).hash.split('/').pop()!;
  // 前面的离线旅程会重定位一个源文件；本旅程按当前实际夹具验证完整导入。
  // 固定 200 张数量的断言继续由 local-core.spec.ts 覆盖。
  const fixtures = await rpc<string[][]>('test_fixture_checksums');
  const fixtureCount = fixtures.length;
  expect(fixtureCount).toBeGreaterThan(1);
  await page.getByRole('button', { name: '导入照片', exact: true }).first().click();
  await page.getByRole('button', { name: '选择文件夹' }).click();
  await expect(page.getByRole('status').filter({ hasText: new RegExp(`^导入完成 · ${fixtureCount} / ${fixtureCount}$`) })).toBeVisible({ timeout: 100_000 });
  await page.getByRole('button', { name: '完成', exact: true }).click();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('k');
  await expect(page.getByTestId('photo-1')).toHaveClass(/keep/);
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await expect(page.getByText('画布功能需要桌面版。')).toHaveCount(0);
  await expect(page.locator('.sequence-board')).toHaveCount(0);
  await page.getByRole('button', { name: '新建画布', exact: true }).click();
  await expect(page.getByRole('combobox', { name: '选择画布' })).toHaveValue(/.+/);
  await page.getByRole('button', { name: /放入选中照片/ }).click();
  await expect(page.locator('.canvas-photo-node img')).toHaveCount(1);
  await expect.poll(() => page.locator('.canvas-photo-node img').evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.getByRole('button', { name: '便签', exact: true }).click();
  await page.getByRole('textbox', { name: '便签内容' }).fill('用这张照片作为开篇');
  await page.getByRole('button', { name: '保存便签', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  await expect(page.locator('.canvas-sticky-node')).toHaveText('用这张照片作为开篇');
  await expect(page.locator('.canvas-photo-node')).toHaveCount(1);
  await page.getByRole('button', { name: '连接', exact: true }).click();
  await page.locator('.canvas-photo-node').click();
  await page.locator('.canvas-sticky-node').click();
  await expect(page.locator('.canvas-board line')).toHaveCount(1);
  await page.screenshot({ path: '../docs/v3/evidence/h0-2026-10-21/canvas.png' });
  await page.getByRole('button', { name: '出版物', exact: true }).click();
  await expect(page.getByText('出版功能需要桌面版。')).toHaveCount(0);
  await expect(page.locator('.canvas-wrapper')).toHaveCount(0);
  await page.getByRole('textbox', { name: '出版物标题' }).fill('光与停顿');
  await page.getByRole('button', { name: '创建', exact: true }).click();
  await expect(page.locator('.publication-card')).toHaveCount(1);
  await page.getByRole('button', { name: /从序列生成/ }).click();
  await expect(page.getByRole('status').filter({ hasText: '已生成 1 页初稿' })).toBeVisible();
  await page.getByRole('button', { name: '查看页面', exact: true }).click();
  await expect(page.locator('.publication-page-preview')).toHaveCount(2);
  await expect.poll(() => page.locator('.publication-frame img').first().evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.screenshot({ path: '../docs/v3/evidence/h0-2026-10-21/publication.png' });
  await page.getByRole('button', { name: /从序列生成/ }).click();
  await expect(page.getByRole('status').filter({ hasText: /已有页面/ })).toBeVisible();
  await page.goto('about:blank');
  await rpc('test_process_restart');
  await page.goto('/');
  await page.getByText('叙事与出版', { exact: true }).first().click();
  await page.getByRole('button', { name: '出版物', exact: true }).click();
  await expect(page.getByText('光与停顿', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await expect(page.locator('.canvas-photo-node img')).toHaveCount(1);
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  const reopened = await rpc<Project>('get_project', { projectId });
  expect(reopened.photos.length).toBeGreaterThan(0);
  expect(errors).toEqual([]);
});
