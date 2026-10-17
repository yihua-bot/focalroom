import { expect, test } from '@playwright/test';

test('v2 Web 基座复用工作台，示例编辑可操作且刷新不伪装持久化', async ({ page }, testInfo) => {
  const errors: string[] = [];
  const remote: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    if (!new URL(request.url()).hostname.match(/^(127\.0\.0\.1|localhost)$/)) remote.push(request.url());
  });
  await page.goto('/');
  await expect(page).toHaveTitle('FocalRoom · Web 工作台预览');
  await expect.poll(() => page.getByRole('img', { name: 'FocalRoom 标志' }).evaluate(image => (image as HTMLImageElement).naturalWidth)).toBe(480);
  const favicon = await page.locator('link[rel="icon"]').getAttribute('href');
  expect((await page.request.get(favicon!)).headers()['content-type']).toContain('image/png');
  await expect(page.getByText('Web 工作台预览 · 示例数据，刷新后重置 · 进入共享评片')).toBeVisible();
  await expect(page.locator('.tile').first()).toBeVisible();
  await page.locator('.tile').first().click();
  await page.keyboard.press('k');
  await expect(page.getByRole('button', { name: /^已保留\s*1$/ })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('web-workspace-1440.png'), fullPage: true });
  await page.reload();
  await expect(page.getByRole('button', { name: /^已保留\s*0$/ })).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(page.locator('.tile').first()).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('web-workspace-1280.png'), fullPage: true });
  expect(errors).toEqual([]);
  expect(remote).toEqual([]);
});
