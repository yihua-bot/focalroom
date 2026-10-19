import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';

// 界面文案跟随浏览器语言；本旅程断言中文，固定 zh-CN 保证与断言一致。
test.use({ locale: 'zh-CN' });

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.tile')).toHaveCount(28);
});

test('多选、明确 K/X/U、批量人工评分和输入焦点隔离', async ({ page }) => {
  const pick = (index: number) => page.getByTestId(`photo-${index}`).getByRole('button', { name: /^查看第/ });
  await pick(1).click();
  await pick(3).click({ modifiers: ['ControlOrMeta'] });
  await expect(page.getByRole('group', { name: '批量选片' })).toContainText('已选 2 张');
  await pick(5).click({ modifiers: ['Shift'] });
  await expect(page.getByRole('group', { name: '批量选片' })).toContainText('已选 3 张');
  await page.keyboard.press('k');
  await expect(page.locator('.tile.keep')).toHaveCount(3);
  await page.keyboard.press('k');
  await expect(page.locator('.tile.keep')).toHaveCount(3);
  await page.keyboard.press('4');
  await expect(page.locator('.tile-rating')).toHaveCount(3);
  await page.keyboard.press('x');
  await expect(page.locator('.tile.reject')).toHaveCount(3);
  await page.keyboard.press('u');
  await expect(page.locator('.tile.reject, .tile.keep')).toHaveCount(0);
  await page.keyboard.press('0');
  await expect(page.locator('.tile-rating')).toHaveCount(0);
  const search = page.getByRole('searchbox', { name: '搜索照片文件名或备注' });
  await search.fill('kx12345');
  await expect(page.getByRole('heading', { name: '当前筛选没有照片' })).toBeVisible();
  await search.fill('城市边缘');
  await expect(page.locator('.tile')).toHaveCount(1);
  await pick(3).click();
  await page.keyboard.press('k');
  await expect(page.locator('.tile.keep')).toHaveCount(1);
  await search.fill('');
  await expect(page.locator('.tile.keep')).toHaveCount(1);
});

test('大图适合屏幕、100% 缓存像素、平移、翻片与关闭焦点', async ({ page }) => {
  const pick = page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ });
  await pick.click();
  await page.keyboard.press('Space');
  const modal = page.getByRole('dialog');
  const viewport = modal.getByTestId('photo-viewport');
  await expect(modal).toBeVisible();
  await expect.poll(() => viewport.locator('img').evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await modal.getByRole('button', { name: '大图100% 预览' }).click();
  const naturalWidth = await viewport.locator('img').evaluate(image => (image as HTMLImageElement).naturalWidth);
  await expect(viewport.locator('img')).toHaveCSS('width', `${naturalWidth}px`);
  await modal.getByRole('button', { name: '大图放大' }).click();
  await modal.getByRole('button', { name: '大图放大' }).click();
  const box = await viewport.boundingBox();
  if (!box) throw new Error('预览不可见');
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 60, box.y + box.height / 2 + 30, { steps: 5 });
  await page.mouse.up();
  await expect(viewport).not.toHaveAttribute('data-pan', '0,0');
  await modal.getByRole('button', { name: '下一张照片' }).click();
  await expect(modal.getByRole('heading')).toHaveText('交错的视线');
  await expect(viewport).toHaveAttribute('data-mode', 'fit');
  await page.keyboard.press('ArrowRight');
  await expect(modal.getByRole('heading')).toHaveText('城市边缘');
  await page.keyboard.press('Escape');
  await expect(modal).toHaveCount(0);
  await expect(pick).toBeFocused();
});

test('选中两张比较，同步与独立缩放平移不会改写状态', async ({ page }) => {
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.getByTestId('photo-3').getByRole('button', { name: /^查看第/ }).click({ modifiers: ['ControlOrMeta'] });
  await page.keyboard.press('c');
  const modal = page.getByRole('dialog');
  const viewports = modal.getByTestId('photo-viewport');
  await expect(modal.getByRole('img', { name: '某个上午' })).toBeVisible();
  await expect(modal.getByRole('img', { name: '城市边缘' })).toBeVisible();
  await modal.getByRole('button', { name: '左图放大' }).click();
  await modal.getByRole('button', { name: '左图放大' }).click();
  await expect(viewports.nth(0)).toHaveAttribute('data-scale', '2.25');
  await expect(viewports.nth(1)).toHaveAttribute('data-scale', '2.25');
  const box = await viewports.first().boundingBox();
  if (!box) throw new Error('比较预览不可见');
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 40, box.y + box.height / 2 + 25, { steps: 4 });
  await page.mouse.up();
  await expect(viewports.first()).not.toHaveAttribute('data-pan', '0,0');
  expect(await viewports.nth(1).getAttribute('data-pan')).toBe(await viewports.first().getAttribute('data-pan'));
  await modal.getByRole('checkbox', { name: '同步缩放与平移' }).uncheck();
  await modal.getByRole('button', { name: '右图适合屏幕' }).click();
  await expect(viewports.nth(1)).toHaveAttribute('data-mode', 'fit');
  await expect(viewports.nth(0)).toHaveAttribute('data-mode', 'actual');
  await modal.getByRole('button', { name: '不确定', exact: true }).click();
  await expect(modal).toHaveCount(0);
  await expect(page.locator('.tile.keep, .tile.reject')).toHaveCount(0);
});

test('搜索快捷键、当前视图全选、面板折叠和双尺寸可用性', async ({ page }) => {
  const screenshots = resolve('../docs/screenshots/v1-m0');
  await mkdir(screenshots, { recursive: true });
  await page.keyboard.press('Control+f');
  await expect(page.getByRole('searchbox')).toBeFocused();
  await page.getByRole('searchbox').fill('光');
  await page.getByTestId('photo-4').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('Control+a');
  const count = await page.locator('.tile').count();
  await page.keyboard.press('k');
  await expect(page.locator('.tile.keep')).toHaveCount(count);
  await page.getByRole('searchbox').fill('');
  await page.getByTestId('photo-4').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('s');
  await expect(page.getByTestId('sequence-card')).toHaveCount(count);
  await page.keyboard.press('g');
  await expect(page.locator('.tile')).toHaveCount(28);
  await page.getByRole('button', { name: '收起序列带' }).click();
  await expect(page.locator('.selection-dock')).toHaveCount(0);
  await page.getByRole('button', { name: '展开序列带' }).click();
  await page.getByRole('button', { name: '收起导航' }).click();
  await expect(page.locator('.app-shell')).toHaveClass(/sidebar-collapsed/);
  await page.getByRole('button', { name: '展开导航' }).click();
  await page.keyboard.press('Control+Shift+a');
  await expect(page.locator('.ai-drawer')).toBeVisible();
  await page.keyboard.press('Control+Shift+a');
  await expect(page.locator('.ai-drawer')).toHaveCount(0);
  await expect(page.locator('.inspector')).toBeVisible();
  for (const width of [1920, 1280]) {
    await page.setViewportSize({ width, height: width === 1920 ? 1080 : 800 });
    await page.screenshot({ path: resolve(screenshots, `preview-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await expect(page.getByRole('searchbox')).toBeVisible();
  }
});
