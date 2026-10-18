import { test, expect, type Page } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';

// 界面文案跟随浏览器语言；本旅程断言中文，固定 zh-CN 保证与断言一致。
test.use({ locale: 'zh-CN' });

const screenshots = resolve('../docs/screenshots/m0');
async function ready(page: Page) {
  await page.goto('/');
  await expect(page.locator('.tile')).toHaveCount(28);
  await expect.poll(() => page.locator('.photo-wrap img').first().evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
}
async function keep(page: Page, index: number) {
  await page.getByTestId(`photo-${index}`).getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('k');
  await expect(page.getByTestId(`photo-${index}`)).toHaveClass(/keep/);
}

test('实际运行原始 standalone Demo 并核对主要交互', async ({ page }) => {
  await page.goto(`file://${resolve('../demo-standalone.html')}`);
  await expect(page.locator('.tile')).toHaveCount(28);
  await expect(page.locator('.tile img').first()).toHaveJSProperty('complete', true);
  await page.locator('[data-action="run-ai"]').click();
  await expect(page.locator('.tile')).toHaveCount(11);
  await expect(page.locator('.dock-images button')).toHaveCount(0);
  await page.locator('.tile').first().click();
  await page.keyboard.press('k');
  await expect(page.locator('.dock-images button')).toHaveCount(1);
  await page.locator('[data-action="view-seq"]').first().click();
  await expect(page.locator('.seq-card')).toHaveCount(1);
  await page.locator('[data-action="compare"]').click();
  await expect(page.locator('.compare-cols img')).toHaveCount(2);
});

test('工作台与检查器截图，完整构图、密度切换和面板收起', async ({ page }) => {
  await mkdir(screenshots, { recursive: true });
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await ready(page);
  await expect(page).toHaveTitle('FocalRoom');
  const logo = page.getByRole('img', { name: 'FocalRoom 标志' });
  await expect(logo).toBeVisible();
  await expect.poll(() => logo.evaluate(image => (image as HTMLImageElement).naturalWidth)).toBe(480);
  const favicon = await page.locator('link[rel="icon"]').getAttribute('href');
  expect((await page.request.get(favicon!)).headers()['content-type']).toContain('image/png');
  await page.getByRole('button', { name: '收起导航' }).click();
  await expect(logo).toBeVisible();
  await page.getByRole('button', { name: '展开导航' }).click();
  await keep(page, 1);
  await keep(page, 3);
  await keep(page, 5);
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await expect(page.locator('.photo-wrap img').first()).toHaveCSS('object-fit', 'contain');
  await page.screenshot({ path: resolve(screenshots, 'workspace.png') });
  await page.getByRole('button', { name: '切换网格密度' }).click();
  await expect(page.locator('.contact-grid')).toHaveClass(/dense/);
  await page.getByRole('button', { name: '收起检查器' }).click();
  await expect(page.locator('.inspector')).toHaveCount(0);
  await page.getByRole('button', { name: '展开检查器' }).click();
  await expect(page.locator('.inspector')).toBeVisible();
  expect(errors).toEqual([]);
});

test('比较截图、焦点陷阱、Esc 与偏好不会改写选片', async ({ page }) => {
  await ready(page);
  await page.getByRole('button', { name: '两图对比' }).click();
  const modal = page.getByRole('dialog');
  await expect(modal).toBeVisible();
  await page.screenshot({ path: resolve(screenshots, 'compare.png') });
  for (let i = 0; i < 8; i++) await page.keyboard.press('Tab');
  expect(await page.evaluate(() => Boolean(document.activeElement?.closest('[role="dialog"]')))).toBe(true);
  await page.keyboard.press('Escape');
  await expect(modal).toHaveCount(0);
  await page.keyboard.press('c');
  await page.getByRole('button', { name: '偏好右张' }).click();
  await expect(modal).toHaveCount(0);
  await expect(page.locator('.dock-images button')).toHaveCount(0);
  await expect(page.locator('.tile.keep, .tile.reject')).toHaveCount(0);
});

test('作品序列截图，鼠标拖拽与键盘上下移动一致', async ({ page }) => {
  await ready(page);
  for (const index of [1, 3, 5, 7, 10, 13]) await keep(page, index);
  await page.getByRole('button', { name: '作品序列', exact: true }).click();
  const cards = page.getByTestId('sequence-card');
  await expect(cards).toHaveCount(6);
  await page.getByRole('button', { name: '下移 某个上午' }).click();
  await expect(cards.nth(1)).toContainText('某个上午');
  await cards.nth(1).getByRole('button', { name: /^查看序列/ }).click();
  await page.keyboard.press('ArrowUp');
  await expect(cards.first()).toContainText('某个上午');
  const handle = await page.getByRole('button', { name: '拖动 某个上午' }).boundingBox();
  const target = await cards.nth(2).boundingBox();
  if (!handle || !target) throw new Error('拖拽目标不可见');
  await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await page.mouse.down();
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 20 });
  await page.mouse.up();
  await expect(cards.nth(2)).toContainText('某个上午');
  // 等待拖拽落位动画结束，截图不得捕获过渡中的偏移卡片。
  await expect.poll(async () => {
    const card = await cards.first().boundingBox();
    const grid = await page.locator('.sequence-grid').boundingBox();
    return Boolean(card && grid && Math.abs(card.x - grid.x) < 1);
  }).toBe(true);
  await page.mouse.move(1100, 30);
  await page.locator('.workspace-scroll').evaluate(element => { element.scrollTop = 0; });
  await page.screenshot({ path: resolve(screenshots, 'sequence.png') });
});

test('序列把手支持 Space 与方向键拖放，不与全局快捷键冲突', async ({ page }) => {
  await ready(page);
  for (const index of [1, 3, 5]) await keep(page, index);
  await page.getByRole('button', { name: '作品序列', exact: true }).click();
  await page.getByRole('button', { name: '拖动 某个上午' }).focus();
  await page.keyboard.press('Space');
  await expect(page.locator('.sequence-card.dragging')).toContainText('某个上午');
  // dnd-kit 激活后在下一帧测量可放置区域，等待真实渲染生命周期。
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  await page.keyboard.press('ArrowRight');
  await expect.poll(() => page.locator('.sequence-card.dragging').evaluate(element => getComputedStyle(element).transform)).not.toBe('matrix(1, 0, 0, 1, 0, 0)');
  await page.keyboard.press('Space');
  await expect(page.getByTestId('sequence-card').nth(1)).toContainText('某个上午');
  await expect(page.getByTestId('sequence-card')).toHaveCount(3);
});

test('序列把手处理单次合并移动事件，Escape 取消后顺序不变', async ({ page }) => {
  await ready(page);
  for (const index of [1, 3, 5]) await keep(page, index);
  await page.getByRole('button', { name: '作品序列', exact: true }).click();
  const cards = page.getByTestId('sequence-card');
  const begin = async () => {
    const handle = await page.getByRole('button', { name: '拖动 某个上午' }).boundingBox();
    if (!handle) throw new Error('拖动把手不可见');
    await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
    await page.mouse.down();
    await expect(page.locator('.sequence-card.dragging')).toContainText('某个上午');
    await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  };
  const target = await cards.nth(2).boundingBox();
  if (!target) throw new Error('落点不可见');
  await begin();
  // 原生窗口可能只交付一次移动，不能依赖第二个事件更新拖动位移。
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 1 });
  await page.mouse.up();
  await expect(cards.nth(2)).toContainText('某个上午');
  const before = await cards.allTextContents();
  await begin();
  const first = await cards.first().boundingBox();
  if (!first) throw new Error('取消测试落点不可见');
  await page.mouse.move(first.x + first.width / 2, first.y + first.height / 2, { steps: 1 });
  await page.keyboard.press('Escape');
  await page.mouse.up();
  await expect(page.locator('.sequence-card.dragging')).toHaveCount(0);
  expect(await cards.allTextContents()).toEqual(before);
});

test('新建项目的空状态截图与项目导航', async ({ page }) => {
  await ready(page);
  await page.getByRole('button', { name: '项目列表', exact: true }).click();
  await expect(page.getByRole('heading', { name: '项目' })).toBeVisible();
  await page.getByRole('button', { name: '新建项目', exact: true }).click();
  await page.locator('#project-title').fill('光线的间隙');
  await page.locator('#project-brief').fill('在日常空间里，寻找缓慢的光。');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page.getByRole('heading', { name: '暂无照片' })).toBeVisible();
  await page.screenshot({ path: resolve(screenshots, 'empty.png') });
  await page.getByRole('button', { name: '项目列表', exact: true }).click();
  await page.getByRole('button', { name: /示例项目|SAMPLE PROJECT/ }).click();
  await expect(page.locator('.tile')).toHaveCount(28);
});

test('备注编辑不会触发 K/X；刷新明确重置内存预览', async ({ page }) => {
  await ready(page);
  await keep(page, 1);
  await page.getByRole('textbox', { name: '照片备注' }).fill('kx 留白与安静');
  await page.getByRole('button', { name: '保存备注' }).click();
  await expect(page.locator('.dock-images button')).toHaveCount(1);
  await page.getByTestId('photo-2').getByRole('button', { name: /^查看第/ }).click();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await expect(page.getByRole('textbox', { name: '照片备注' })).toHaveValue('kx 留白与安静');
  await page.reload();
  await expect(page.locator('.dock-images button')).toHaveCount(0);
  await expect(page.getByRole('textbox', { name: '照片备注' })).toHaveValue('');
});

test('AI、导入和导出边界明示；没有外部请求', async ({ page }) => {
  const external: string[] = [];
  page.on('request', request => { if (!request.url().startsWith('http://localhost:1420')) external.push(request.url()); });
  await ready(page);
  await page.getByRole('main').getByRole('button', { name: '编辑建议' }).click();
  await expect(page.getByRole('button', { name: '运行示例任务' })).toBeDisabled();
  await page.getByRole('button', { name: '导出 ↗' }).click();
  await expect(page.getByRole('button', { name: '项目 JSON' })).toBeDisabled();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '导入照片', exact: true }).click();
  await expect(page.getByRole('dialog')).toContainText('请在桌面版导入照片。');
  expect(external).toEqual([]);
});

test('390px 窄屏能导航、选片和查看检查器，无横向溢出', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await ready(page);
  await expect(page.getByRole('button', { name: '作品序列', exact: true })).toBeVisible();
  await keep(page, 1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.screenshot({ path: resolve(screenshots, 'mobile.png'), fullPage: true });
});
