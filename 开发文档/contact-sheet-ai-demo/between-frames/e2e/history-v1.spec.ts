import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.tile')).toHaveCount(28);
});

test('多选一次撤销，重做与新编辑分支；备注输入不触发项目撤销', async ({ page }) => {
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.getByTestId('photo-3').getByRole('button', { name: /^查看第/ }).click({ modifiers: ['Shift'] });
  await page.keyboard.press('k');
  await expect(page.locator('.tile.keep')).toHaveCount(3);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.tile.keep')).toHaveCount(0);
  await page.keyboard.press('ControlOrMeta+Shift+z');
  await expect(page.locator('.tile.keep')).toHaveCount(3);
  await page.keyboard.press('ControlOrMeta+z');
  await expect(page.locator('.tile.keep')).toHaveCount(0);
  await page.keyboard.press('x');
  await expect(page.locator('.tile.reject')).toHaveCount(3);
  await expect(page.getByRole('button', { name: '重做', exact: true })).toBeDisabled();
  const note = page.getByRole('textbox', { name: '为什么保留 / 淘汰这张照片？' });
  await note.fill('文本自己的撤销');
  await page.keyboard.press('ControlOrMeta+z');
  await expect(page.locator('.tile.reject')).toHaveCount(3);
});

test('版本保存、范围预览、恢复与撤销恢复', async ({ page }) => {
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('k');
  await page.keyboard.press('4');
  await page.getByRole('button', { name: /^作品版本/ }).click();
  await page.getByRole('textbox', { name: '版本名称' }).fill('第一版');
  await page.getByRole('button', { name: '保存当前版本' }).click();
  await expect(page.getByRole('button', { name: /第一版/ })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.keyboard.press('x');
  await expect(page.locator('.tile.reject')).toHaveCount(1);
  await page.getByRole('button', { name: /^作品版本/ }).click();
  await page.getByRole('button', { name: /第一版/ }).click();
  await expect(page.getByRole('region', { name: '版本恢复预览' })).toContainText('加入 1 张');
  await page.getByRole('button', { name: '确认恢复此版本' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.tile.keep')).toHaveCount(1);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.tile.reject')).toHaveCount(1);
  await expect(page.getByRole('button', { name: /^作品版本/ })).toContainText('1');
});
