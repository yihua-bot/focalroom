import { expect, test } from '@playwright/test';

// v2 Web 入口现状：apps/web 是共享评片入口（浏览器登录/桌面回跳），不再内置示例工作台。
// 本旅程验证：入口可渲染、标志/图标正确、登录表单可用、无页面错误、不外发第三方请求。
test('v2 Web 共享入口渲染登录并保持零外发', async ({ page }, testInfo) => {
  const errors: string[] = [];
  const remote: string[] = [];
  // 受控 API mock：auth/me 未登录；magic 请求返回已发送（不真实发信、不依赖 PG）。
  await page.route('**/api/v2/**', async route => {
    const url = route.request().url();
    if (url.includes('/auth/me')) {
      return route.fulfill({ status: 401, contentType: 'application/json', body: JSON.stringify({ error: { code: 'UNAUTHORIZED', message: 'not signed in' } }) });
    }
    if (url.includes('/auth/magic/request')) {
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ sent: true }) });
    }
    return route.fulfill({ status: 404, contentType: 'application/json', body: JSON.stringify({ error: { code: 'NOT_FOUND', message: 'mock' } }) });
  });
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    if (!new URL(request.url()).hostname.match(/^(127\.0\.0\.1|localhost)$/)) remote.push(request.url());
  });
  await page.goto('/');
  await expect(page).toHaveTitle('FocalRoom — 团队协作审稿');
  await expect(page.getByRole('heading', { name: /共享评片|Shared review/i })).toBeVisible();
  await expect(page.getByRole('img', { name: 'FocalRoom 标志' })).toBeVisible();
  const favicon = await page.locator('link[rel="icon"]').getAttribute('href');
  expect((await page.request.get(favicon!)).headers()['content-type']).toContain('image/png');
  // 登录表单：邮箱输入与发送验证码动作存在（提交会命中本地 mock/失败路径，不真实发信）。
  const email = page.locator('input[type="email"]');
  await expect(email).toBeVisible();
  const send = page.getByRole('button', { name: /发送验证码|Send code/i });
  await expect(send).toBeDisabled();
  await email.fill('e2e-entry@fixture.test');
  await expect(send).toBeEnabled();
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.screenshot({ path: testInfo.outputPath('web-entry-1280.png'), fullPage: true });
  expect(errors).toEqual([]);
  expect(remote).toEqual([]);
});
