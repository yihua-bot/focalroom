import type { Page } from '@playwright/test';
import type { CloudStatus } from '../src/domain/cloud-auth';
import type { BillingOverview, SubscriptionCatalog } from '../src/domain/billing';

// 仅用于离线浏览器验收，不注册真实账户、不创建付款、不发送照片。
export async function installAccountFixture(page: Page, guest = false) {
  const user = { id: 'account-fixture', name: 'Yihua', email: 'author@fixture.test', avatar: null };
  const status: CloudStatus = { authenticated: !guest, waiting: false, origin: 'https://team.focroom.cc', user: guest ? null : user, error: null };
  const catalog: SubscriptionCatalog = { salesAvailable: true, plans: [
    { id: 'free', name: '基础版', description: '', features: ['local','selection','demo'], prices: [] },
    { id: 'pro', name: 'FocalRoom Pro', description: '', features: ['local','selection','ai','sharing','history'], prices: [
      { id: 'price_fixtureMonth', interval: 'month', amount: 1900, currency: 'USD', includedCredits: 50 },
      { id: 'price_fixtureYear', interval: 'year', amount: 19000, currency: 'USD', includedCredits: 600 },
    ] },
  ] };
  const overview: BillingOverview = { planId: 'free', planName: '基础版', status: 'free', currentPeriodEnd: null, cancelAtPeriodEnd: false,
    usage: { remainingPercent: 75, hasAllowance: true, availableCredits: 37.5, totalCredits: 50 }, invoices: [], manageAvailable: false, pendingCheckout: false, canCancelCheckout: true };
  const calls: string[] = [];
  await page.route('**/rpc', async route => {
    const { command } = route.request().postDataJSON();
    const values: Record<string, () => unknown> = {
      cloud_login_status: () => status,
      cloud_login_open: () => { status.waiting = true; },
      cloud_login_cancel: () => { status.waiting = false; },
      cloud_login_logout: () => { status.authenticated = false; status.user = null; return true; },
      billing_catalog: () => catalog, billing_overview: () => overview,
      billing_checkout: () => undefined, billing_manage: () => undefined, billing_cancel_checkout: () => undefined,
    };
    if (!values[command]) return route.fallback();
    calls.push(command);
    return route.fulfill({ json: { ok: true, value: values[command]() ?? null } });
  });
  return { status, catalog, overview, calls, async signIn() {
    status.authenticated = true; status.waiting = false; status.user = user;
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  } };
}

export async function installDesktopBridge(page: Page, rpcBase = 'http://localhost:1422') {
  await page.addInitScript(({ rpcBase }) => {
    if (!['http:', 'https:'].includes(window.location.protocol)) return;
    localStorage.setItem('focalroom-locale', 'zh');
    Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
      invoke: async (command: string, args: unknown) => {
        if (command.startsWith('plugin:event|') || command === 'set_native_menu_locale') return 0;
        const result = await fetch(rpcBase + '/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
        if (!result.ok) throw result.error;
        return result.value;
      },
      convertFileSrc: (path: string) => rpcBase + '/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
    } });
  }, { rpcBase });
}
