import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { expect, it, vi } from 'vitest';
import { SubscriptionDialog } from './SubscriptionDialog';
import type { BillingOverview, BillingRepository, SubscriptionCatalog } from '../../domain/billing';
import type { CloudStatus } from '../../domain/cloud-auth';

const account: CloudStatus = { authenticated: true, waiting: false, origin: 'https://team.focroom.cc', user: { id: 'u', name: '摄影师', email: 'author@fixture.test' }, error: null };
const catalog: SubscriptionCatalog = { salesAvailable: true, plans: [
  { id: 'free', name: '基础版', description: '', features: ['local', 'selection', 'demo'], prices: [] },
  { id: 'pro', name: 'FocalRoom Pro', description: '', features: ['ai', 'sharing'], prices: [
    { id: 'price_monthly', interval: 'month', amount: 1900, currency: 'USD', includedCredits: 50 },
    { id: 'price_yearly', interval: 'year', amount: 19000, currency: 'USD', includedCredits: 600 },
  ] },
] };
function overview(): BillingOverview { return { planId: 'free', planName: '基础版', status: 'free', currentPeriodEnd: null, cancelAtPeriodEnd: false, usage: { remainingPercent: 75, hasAllowance: true, availableCredits: 37.5, totalCredits: 50 }, invoices: [], manageAvailable: false, pendingCheckout: false, canCancelCheckout: true }; }
function fixture(): BillingRepository { return { catalog: vi.fn(async () => catalog), overview: vi.fn(async () => overview()), checkout: vi.fn(async () => {}), manage: vi.fn(async () => {}), cancelCheckout: vi.fn(async () => {}) }; }
function mount(repository: BillingRepository, signedIn = true, page: 'plans'|'usage'|'billing' = 'plans') {
  const login = vi.fn();
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><SubscriptionDialog repository={repository} account={signedIn ? account : null} onClose={vi.fn()} initialPage={page} onLogin={login} /></QueryClientProvider>);
  return login;
}

it('套餐显示服务器价格和额度，按月/年选择对应价格，打开支付不冒充已订阅', async () => {
  const repository = fixture(), user = userEvent.setup(); mount(repository);
  expect(await screen.findByText('US$19.00')).toBeVisible();
  expect(screen.getByText('包含 50 点 AI 额度')).toBeVisible();
  await user.click(screen.getByRole('button', { name: '年付' }));
  expect(screen.getByText('US$190.00')).toBeVisible();
  expect(screen.getByText('包含 600 点 AI 额度')).toBeVisible();
  await user.click(screen.getByRole('button', { name: '前往支付' }));
  expect(repository.checkout).toHaveBeenCalledWith('price_yearly', expect.any(String));
  expect(await screen.findByText('返回应用后会重新确认订阅状态。')).toBeVisible();
  expect(screen.getByRole('button', { name: '前往支付' })).toBeEnabled();
  await user.click(screen.getByRole('button', { name: '取消本次付款' }));
  expect(repository.cancelCheckout).toHaveBeenCalledOnce();
  expect(await screen.findByText('本次付款已取消，可以重新选择订阅。')).toBeVisible();
});

it('访客先登录，不创建付款', async () => {
  const repository = fixture(), login = mount(repository, false);
  await userEvent.setup().click(await screen.findByRole('button', { name: '登录后继续' }));
  expect(login).toHaveBeenCalledOnce();expect(repository.checkout).not.toHaveBeenCalled();
});

it('销售未开放时展示方案，付款入口不可用', async () => {
  const repository = fixture();
  vi.mocked(repository.catalog).mockResolvedValue({ ...catalog, salesAvailable: false });
  mount(repository);
  expect(await screen.findByRole('button', { name: '订阅暂未开放' })).toBeDisabled();
  expect(repository.checkout).not.toHaveBeenCalled();
});

it('重新打开订阅仍可取消尚未完成的付款', async () => {
  const repository = fixture();
  vi.mocked(repository.overview).mockResolvedValue({ ...overview(), pendingCheckout: true });
  mount(repository, true, 'billing');
  await userEvent.setup().click(await screen.findByRole('button', { name: '取消本次付款' }));
  expect(repository.cancelCheckout).toHaveBeenCalledOnce();
});

it('付款渠道不支持主动取消时隐藏取消按钮并说明到期失效', async () => {
  const repository = fixture();
  vi.mocked(repository.overview).mockResolvedValue({ ...overview(), pendingCheckout: true, canCancelCheckout: false });
  mount(repository, true, 'billing');
  expect(await screen.findByText(/付款链接到期后会自动失效/)).toBeVisible();
  expect(screen.queryByRole('button', { name: '取消本次付款' })).toBeNull();
  expect(repository.cancelCheckout).not.toHaveBeenCalled();
});

it('网络异常重试复用付款编号，明确过期后才生成新编号', async () => {
  const repository = fixture();
  vi.mocked(repository.checkout).mockRejectedValueOnce(new Error('网络暂时不可用')).mockRejectedValueOnce(Object.assign(new Error('付款页面已过期'), { code: 'BILLING_SESSION_EXPIRED' }));
  mount(repository); const user = userEvent.setup();
  await user.click(await screen.findByRole('button', { name: '前往支付' }));
  await screen.findByRole('alert');
  await user.click(screen.getByRole('button', { name: '前往支付' }));
  await screen.findByText('付款页面已过期');
  await user.click(screen.getByRole('button', { name: '前往支付' }));
  const calls = vi.mocked(repository.checkout).mock.calls;
  expect(calls[0][1]).toBe(calls[1][1]); expect(calls[2][1]).not.toBe(calls[1][1]);
});

it('用量读取真实账户，返回应用后刷新；不把未知余额显示成零', async () => {
  const repository = fixture(); mount(repository, true, 'usage');
  expect(await screen.findByText('75%')).toBeVisible();
  expect(screen.getByText('剩余 37.5 点')).toBeVisible();
  vi.mocked(repository.overview).mockResolvedValue({ ...overview(), usage: { remainingPercent: null, hasAllowance: false, availableCredits: 0, totalCredits: 0 } });
  await act(async () => window.dispatchEvent(new Event('focus')));
  expect(await screen.findByText('暂无可用额度')).toBeVisible();
});

it('账单显示取消续费后的有效期，可进入账户专属管理页面', async () => {
  const repository = fixture();
  vi.mocked(repository.overview).mockResolvedValue({ ...overview(), planId: 'pro', planName: 'FocalRoom Pro', status: 'active', currentPeriodEnd: '2026-11-22T00:00:00.000Z', cancelAtPeriodEnd: true, manageAvailable: true, invoices: [{ id: 'invoice', date: '2026-10-22T00:00:00.000Z', amount: 1900, currency: 'USD', status: 'paid', url: null }] });
  mount(repository, true, 'billing');
  expect(await screen.findByText(/可使用至/)).toBeVisible();expect(screen.getByText('已支付')).toBeVisible();
  await userEvent.setup().click(screen.getByRole('button', { name: '管理订阅' }));
  await waitFor(() => expect(repository.manage).toHaveBeenCalledOnce());
});
