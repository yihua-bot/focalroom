import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { expect, it, vi } from 'vitest';
import type { CloudAuthRepository, CloudStatus } from '../../domain/cloud-auth';
import { AccountProvider, useAccount } from './AccountProvider';

function fixture() {
  let value: CloudStatus = { authenticated: false, waiting: false, origin: 'https://team.focroom.cc', user: null, error: null };
  const listeners = new Set<() => void>();
  const repository: CloudAuthRepository = {
    status: vi.fn(async () => value), open: vi.fn(async () => { value = { ...value, waiting: true }; listeners.forEach(fn => fn()); }),
    cancel: vi.fn(async () => { value = { ...value, waiting: false }; listeners.forEach(fn => fn()); }), logout: vi.fn(), updateProfile: vi.fn(),
    subscribe: vi.fn(async callback => { listeners.add(callback); return () => listeners.delete(callback); }),
  };
  const signedIn = () => { value = { ...value, authenticated: true, waiting: false, user: { id: 'u', name: '摄影师', email: 'author@fixture.test' } }; listeners.forEach(fn => fn()); };
  return { repository, signedIn };
}
function mount(auth: CloudAuthRepository, action = vi.fn()) {
  function Entry() { const account = useAccount()!; return <button onClick={() => void account.requireSignIn('ai', action)}>AI 智能选片</button>; }
  render(<QueryClientProvider client={new QueryClient()}><AccountProvider auth={auth}><Entry /></AccountProvider></QueryClientProvider>);
  return action;
}

it('AI 入口说明登录原因，不自动打开浏览器；登录后仅恢复一次原操作', async () => {
  const f = fixture(), action = mount(f.repository), user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'AI 智能选片' }));
  expect(await screen.findByRole('dialog', { name: '登录 FocalRoom' })).toBeVisible();
  expect(screen.getByText(/登录后即可使用 AI 智能选片/)).toBeVisible();
  expect(screen.getByText('登录不会自动上传照片。')).toBeVisible();
  expect(f.repository.open).not.toHaveBeenCalled(); expect(action).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: '继续登录' }));
  expect(await screen.findByText('请在浏览器完成登录')).toBeVisible();
  await act(async () => f.signedIn());
  await waitFor(() => expect(action).toHaveBeenCalledTimes(1));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await act(async () => f.signedIn());
  expect(action).toHaveBeenCalledTimes(1);
});

it('取消登录丢弃待执行动作，之后登录也不会偷偷启动选片', async () => {
  const f = fixture(), action = mount(f.repository), user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'AI 智能选片' }));
  await user.click(await screen.findByRole('button', { name: '暂不登录' }));
  await act(async () => f.signedIn());
  expect(action).not.toHaveBeenCalled();
});

it('已登录直接执行，业务错误不会误弹登录框', async () => {
  const f = fixture(); f.signedIn();
  const action = vi.fn(async () => { throw new Error('模拟保存失败'); }); mount(f.repository, action);
  await userEvent.setup().click(screen.getByRole('button', { name: 'AI 智能选片' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('操作未完成，请重试。');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it('会话已过期时，旧的登录状态不能关闭弹窗或恢复受保护操作', async () => {
  const f = fixture(); f.signedIn();
  const action = mount(f.repository);
  await act(async () => {});
  vi.mocked(f.repository.status).mockResolvedValue({ authenticated: false, waiting: false, origin: 'https://team.focroom.cc', user: null, error: null });
  await userEvent.setup().click(screen.getByRole('button', { name: 'AI 智能选片' }));
  expect(await screen.findByRole('dialog', { name: '登录 FocalRoom' })).toBeVisible();
  expect(action).not.toHaveBeenCalled();
});
