import { test, expect } from '@playwright/test';
import { mkdir, readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';

type StoredProject = { id: string; revision: number; photos: { id: string; status: string; note: string; originalPath: string }[]; sequence: string[]; events: { kind: string }[] };
const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  const result = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(response => response.json());
  if (!result.ok) throw new Error(result.error.message);
  return result.value as T;
};

test('v1.0 M1 真实 Store 离线旅程：200 张、批量选 25、Undo/Redo、版本恢复、私密导出与重启', async ({ page }) => {
  const screenshots = resolve('../docs/screenshots/v1-m1');
  await mkdir(screenshots, { recursive: true });
  const errors: string[] = [];
  const externalRequests: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    const url = new URL(route.request().url());
    if (['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) return route.continue();
    externalRequests.push(url.origin);
    return route.abort('internetdisconnected');
  });
  await page.addInitScript(() => {
    const root = globalThis as typeof globalThis & { isTauri: boolean; __TAURI_INTERNALS__: unknown };
    root.isTauri = true;
    Object.assign(root, { __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} } });
    root.__TAURI_INTERNALS__ = {
      invoke: async (command: string, args: unknown) => {
        if (command.startsWith('plugin:event|')) return 0;
        const result = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(response => response.json());
        if (!result.ok) throw result.error;
        return result.value;
      },
      convertFileSrc: (path: string) => `http://localhost:1422/asset?path=${encodeURIComponent(path)}`,
      transformCallback: () => 0,
    };
  });
  await page.goto('/');
  await page.getByRole('button', { name: '新建项目', exact: true }).first().click();
  await page.getByRole('textbox', { name: '项目名称' }).fill('光线的间隙 · M1 验收');
  await page.getByRole('textbox', { name: '创作意图' }).fill('在日常空间里，寻找画面之间的距离、光线与沉默。');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page.getByRole('heading', { name: '暂无照片' })).toBeVisible();
  await page.screenshot({ path: resolve(screenshots, 'empty.png') });
  const projectId = new URL(page.url()).hash.split('/').pop()!;
  const checksums = await rpc<string[][]>('test_fixture_checksums');
  expect(checksums).toHaveLength(200);
  await page.getByRole('button', { name: '导入照片', exact: true }).first().click();
  await page.getByRole('button', { name: '选择文件夹' }).click();
  await expect(page.getByText('导入完成 · 200 / 200', { exact: true })).toBeVisible({ timeout: 100_000 });
  await page.getByRole('button', { name: '完成', exact: true }).click();
  await expect(page.getByRole('button', { name: /^全部\s*200$/ })).toBeVisible();
  await expect.poll(() => page.locator('.photo-wrap img').first().evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  expect(await page.locator('.tile').count()).toBeLessThan(200);
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('k');
  await expect(page.getByRole('button', { name: /^已保留\s*1$/ })).toBeVisible();
  await page.getByRole('textbox', { name: '照片备注' }).fill('保留这张，让光线成为序列中的一次停顿。');
  await page.getByRole('button', { name: '保存备注' }).click();
  await expect(page.getByRole('button', { name: '保存备注' })).toBeDisabled();
  for (const index of [3, 5, 7, 10, 13]) {
    await page.getByTestId(`photo-${index}`).getByRole('button', { name: /^查看第/ }).click();
    await page.keyboard.press('k');
    await expect(page.getByTestId(`photo-${index}`)).toHaveClass(/keep/);
  }
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.locator('.workspace-scroll').evaluate(element => { element.scrollTop = 0; });
  await page.screenshot({ path: resolve(screenshots, 'workspace.png') });
  await page.getByRole('button', { name: '两图对比' }).click();
  await expect(page.getByRole('dialog').getByRole('img', { name: 'frame-000.jpg' })).toHaveJSProperty('naturalWidth', 480);
  await page.screenshot({ path: resolve(screenshots, 'compare.png') });
  await page.getByRole('button', { name: '偏好右张' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^已保留\s*6$/ })).toBeVisible();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  for (let index = 1; index < 25; index++) await page.keyboard.press('Shift+ArrowRight');
  await expect(page.getByRole('group', { name: '批量选片' })).toContainText('已选 25 张');
  await page.getByRole('button', { name: '保留选区' }).click();
  await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
  const kept = await rpc<StoredProject>('get_project', { projectId });
  await page.keyboard.press('k');
  expect(await rpc<StoredProject>('get_project', { projectId })).toEqual(kept);
  await page.getByRole('combobox', { name: '批量人工评分' }).selectOption('4');
  await expect(page.locator('.tile-rating').first()).toContainText('★★★★');
  await page.getByRole('button', { name: '作品序列', exact: true }).click();
  await expect(page.getByTestId('sequence-card')).toHaveCount(25);
  await page.getByRole('button', { name: '下移 frame-000.jpg' }).click();
  await expect(page.getByTestId('sequence-card').nth(1)).toContainText('frame-000.jpg');
  await page.mouse.move(1100, 30);
  await page.screenshot({ path: resolve(screenshots, 'sequence.png') });
  await page.getByRole('button', { name: /^作品版本/ }).click();
  await page.getByRole('textbox', { name: '版本名称' }).fill('25 张 · 初选');
  await page.getByRole('button', { name: '保存当前版本' }).click();
  await expect(page.getByRole('button', { name: /25 张 · 初选/ })).toBeVisible();
  await page.getByRole('button', { name: /25 张 · 初选/ }).click();
  await page.screenshot({ path: resolve(screenshots, 'versions.png') });
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '上移 frame-000.jpg' }).click();
  await expect(page.getByTestId('sequence-card').first()).toContainText('frame-000.jpg');
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.getByTestId('sequence-card').nth(1)).toContainText('frame-000.jpg');
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect(page.getByTestId('sequence-card').first()).toContainText('frame-000.jpg');
  await page.getByRole('button', { name: /^作品版本/ }).click();
  await page.getByRole('button', { name: /25 张 · 初选/ }).click();
  await expect(page.getByRole('region', { name: '版本恢复预览' })).toContainText('位置变化 2 张');
  await page.getByRole('button', { name: '确认恢复此版本' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByTestId('sequence-card').nth(1)).toContainText('frame-000.jpg');
  const before = await rpc<StoredProject>('get_project', { projectId });
  await rpc('test_restart');
  await page.reload();
  const after = await rpc<StoredProject>('get_project', { projectId });
  expect(after).toEqual(before);
  await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
  await expect(page.getByRole('textbox', { name: '照片备注' })).toHaveValue('保留这张，让光线成为序列中的一次停顿。');
  expect(new Set(after.events.map(event => event.kind))).toEqual(new Set(['project_create', 'import', 'decision', 'note', 'compare', 'reorder', 'rating', 'version_save', 'undo', 'redo', 'version_restore']));
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(checksums);
  for (const width of [1920, 1280]) {
    await page.setViewportSize({ width, height: width === 1920 ? 1080 : 800 });
    await expect.poll(async () => {
      const rows = page.locator('.virtual-row');
      const first = await rows.nth(0).boundingBox();
      const second = await rows.nth(1).boundingBox();
      return first && second ? second.y - first.y - first.height : 1000;
    }).toBeLessThan(40);
    await page.screenshot({ path: resolve(screenshots, `workspace-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  }
  await page.getByRole('button', { name: '导出 ↗' }).click();
  await page.getByRole('button', { name: '照片总览', exact: true }).click();
  await expect(page.locator('.export-receipt')).toContainText('已导出：');
  const htmlPath = (await page.locator('.export-receipt p').innerText()).replace('已导出：', '');
  const htmlBytes = await readFile(htmlPath);
  await expect(page.locator('.export-receipt')).toContainText(createHash('sha256').update(htmlBytes).digest('hex'));
  expect(htmlBytes.toString('utf8')).not.toContain(before.photos[0].originalPath);
  // JSON/CSV 保留后端兼容性校验，正式导出界面使用面向作品的选项。
  for (const format of ['json', 'csv']) {
    const receipt = await rpc<{ path: string; sha256: string }>('export_project', { projectId, format, includePaths: false });
    const bytes = await readFile(receipt.path);
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(receipt.sha256);
    if (format === 'json') {
      const exported = JSON.parse(bytes.toString('utf8'));
      expect(exported.includesLocalPaths).toBe(false);
      expect(exported.photos[0]).not.toHaveProperty('originalPath');
      expect(bytes.toString('utf8')).not.toContain(before.photos[0].originalPath);
    } else expect(bytes.toString('utf8').split('\r\n')[0]).not.toContain('original_path');
  }
  await page.keyboard.press('Escape');
  await rpc('test_move_source', { projectId });
  await page.reload();
  await expect(page.getByText('原图失联，缓存预览仍可查看。')).toBeVisible();
  await page.getByRole('button', { name: '重新定位原图' }).click();
  await expect(page.getByText('原图失联，缓存预览仍可查看。')).toHaveCount(0);
  expect((await rpc<StoredProject>('get_project', { projectId })).photos[0].id).toBe(before.photos[0].id);
  await page.getByRole('button', { name: '项目列表', exact: true }).click();
  const ownCard = page.locator('.project-card').filter({ hasText: '光线的间隙 · M1 验收' });
  await ownCard.getByRole('button', { name: '归档项目', exact: true }).click();
  await page.getByRole('button', { name: '确认归档', exact: true }).click();
  await expect(ownCard).toHaveCount(0);
  await page.locator('.project-list-tools').getByRole('button', { name: '归档项目', exact: true }).click();
  await ownCard.getByRole('button', { name: '恢复项目', exact: true }).click();
  await expect(ownCard.getByRole('button', { name: /200 张照片/ })).toBeVisible();
  await expect(ownCard.getByRole('button', { name: '归档项目', exact: true })).toBeVisible();
  expect(errors).toEqual([]);
  expect(externalRequests).toEqual([]);
});
