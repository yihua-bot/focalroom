import { test, expect } from '@playwright/test';
import { installAccountFixture, installDesktopBridge } from './account-fixture';
const evidence = '../docs/v3/evidence/account-subscription-2026-10-22';

test('SIMULATED 访客、登录恢复、账户菜单、订阅与账单完整旅程', async ({ page }) => {
  const errors: string[] = [], outbound: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (['localhost','127.0.0.1','[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    outbound.push(route.request().url()); return route.abort();
  });
  const account = await installAccountFixture(page, true);
  await installDesktopBridge(page);
  await page.goto('/');
  await expect(page.getByTestId('cloud-account')).toContainText('未登录');
  await page.getByRole('button', { name: '新建项目', exact: true }).first().click();
  await page.getByRole('textbox', { name: '项目名称' }).fill('光的日记');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page).toHaveURL(/#\/project\//);
  await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
  const login = page.getByRole('dialog', { name: '登录 FocalRoom' });
  await expect(login).toContainText('登录后即可使用 Focal AI，查看可用额度并管理订阅。');
  await expect(login).toContainText('登录不会自动上传照片。');
  expect(account.calls).not.toContain('cloud_login_open');
  await page.screenshot({ animations: 'disabled', path: `${evidence}/login-reason.png` });
  await login.getByRole('button', { name: '继续登录' }).click();
  await expect(login).toContainText('请在浏览器完成登录');
  await account.signIn();
  await expect(login).toHaveCount(0);
  await expect(page.getByRole('dialog', { name: 'Focal AI', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '关闭 Focal AI' }).click();
  const trigger = page.getByRole('button', { name: '账户菜单' });
  await trigger.click();
  const menu = page.getByRole('menu');
  await expect(menu).toContainText('Yihua');
  await expect(menu).toContainText('基础版');
  await expect(menu.getByRole('menuitem', { name: '剩余用量' })).toBeVisible();
  await page.screenshot({ animations: 'disabled', path: `${evidence}/account-menu.png` });
  await menu.getByRole('menuitem', { name: '订阅与账单' }).click();
  const subscription = page.getByRole('dialog', { name: '订阅与用量' });
  await expect(subscription.getByText('包含 50 点 AI 额度')).toBeVisible();
  await subscription.getByRole('button', { name: '年付', exact: true }).click();
  await expect(subscription.getByText('包含 600 点 AI 额度')).toBeVisible();
  await page.screenshot({ animations: 'disabled', path: `${evidence}/subscription-plans.png` });
  await subscription.getByRole('button', { name: '前往支付' }).click();
  await expect(subscription).toContainText('返回应用后会重新确认订阅状态。');
  await subscription.getByRole('button', { name: '剩余用量', exact: true }).click();
  await expect(subscription).toContainText('基础版');
  await expect(subscription).toContainText('75%');
  await page.screenshot({ animations: 'disabled', path: `${evidence}/subscription-usage.png` });
  // 模拟经过服务端核实后的结果；绝不由返回地址或点击付款自行开通。
  Object.assign(account.overview, { planId: 'pro', planName: 'FocalRoom Pro', status: 'active', currentPeriodEnd: '2026-11-22T00:00:00.000Z', cancelAtPeriodEnd: true, manageAvailable: true,
    invoices: [{ id: 'invoice-fixture', date: '2026-10-22T00:00:00.000Z', amount: 19000, currency: 'USD', status: 'paid', url: null }] });
  await subscription.getByRole('button', { name: '订阅与账单', exact: true }).click();
  await subscription.getByRole('button', { name: '刷新', exact: true }).click();
  await expect(subscription).toContainText('FocalRoom Pro');
  await expect(subscription).toContainText('可使用至');
  await expect(subscription).toContainText('已支付');
  await subscription.getByRole('button', { name: '管理订阅', exact: true }).click();
  expect(account.calls).toContain('billing_manage');
  await page.screenshot({ animations: 'disabled', path: `${evidence}/subscription-billing.png` });
  await page.keyboard.press('Escape');
  await trigger.click();
  await expect(menu).toContainText('FocalRoom Pro');
  await page.keyboard.press('ArrowDown'); await page.keyboard.press('End');
  await expect(menu.getByRole('menuitem', { name: '退出登录' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('cloud-account')).toContainText('未登录');
  expect(errors).toEqual([]); expect(outbound).toEqual([]);
});

test('SIMULATED Web 付款返回只查询账户，不信任成功标记，四语订阅可用', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => localStorage.setItem('focalroom-locale', 'zh'));
  const fixture = await installAccountFixture(page);
  await page.route('**/api/v2/**', route => {
    const path = new URL(route.request().url()).pathname;
    const value = path.endsWith('/auth/me') ? { ...fixture.status.user, csrf: 'fixture-csrf', emailVerified: true }
      : path.endsWith('/billing/catalog') ? fixture.catalog
      : path.endsWith('/billing/overview') ? fixture.overview : [];
    return route.fulfill({ json: value });
  });
  await page.goto('http://127.0.0.1:1424/shared#billing=return');
  const dialog = page.getByRole('dialog', { name: '订阅与用量' });
  await expect(dialog).toContainText('基础版');
  await expect(dialog).toContainText('暂无账单');
  await dialog.getByRole('button', { name: '选择订阅', exact: true }).click();
  await page.screenshot({ animations: 'disabled', path: `${evidence}/web-subscription.png` });
  await page.keyboard.press('Escape');
  await expect(page).not.toHaveURL(/billing=/);
  for (const [locale,title,menu] of [['en','Plans & usage','Plans'],['ja','プランと利用状況','プラン'],['ko','구독 및 이용량','구독 선택']] as const) {
    await page.getByRole('combobox', { name: 'Language' }).selectOption(locale);
    await page.getByRole('button', { name: '账户菜单' }).click();
    await page.getByRole('menuitem', { name: menu, exact: true }).click();
    await expect(page.getByRole('dialog', { name: title })).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.getByRole('dialog')).toBeInViewport();
    expect(await page.getByRole('dialog').evaluate(node => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    await page.keyboard.press('Escape');
  }
  expect(errors).toEqual([]);
});

test('SIMULATED 访客从用量页登录：弹窗位于订阅上层，取消后仍可浏览方案', async ({ page }) => {
  const fixture = await installAccountFixture(page, true);
  await installDesktopBridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: '账户菜单' }).click();
  await page.getByRole('menuitem', { name: '剩余用量' }).click();
  const subscription = page.getByRole('dialog', { name: '订阅与用量' });
  await subscription.getByRole('button', { name: '登录后继续' }).click();
  const login = page.getByRole('dialog', { name: '登录 FocalRoom' });
  await expect(login).toContainText('登录后可选择订阅、查看剩余用量和管理账单。');
  await login.getByRole('button', { name: '继续登录' }).click({ trial: true });
  await page.keyboard.press('Escape');
  await expect(login).toHaveCount(0);
  await expect(subscription).toBeVisible();
  await subscription.getByRole('button', { name: '选择订阅', exact: true }).click();
  await expect(subscription.getByText('FocalRoom Pro', { exact: true })).toBeVisible();
  expect(fixture.calls).not.toContain('billing_checkout');
  expect(fixture.calls).not.toContain('cloud_login_open');
});
