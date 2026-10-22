import { render, screen, waitFor, within } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DiagnosticsDialog } from './DiagnosticsDialog';
import { diagnosticsFixture } from './diagnostics.fixture';

function mount(query = vi.fn().mockResolvedValue(diagnosticsFixture())) {
  const onClose = vi.fn();
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <DiagnosticsDialog repository={{ query }} onClose={onClose} />
  </QueryClientProvider>);
  return { query, onClose };
}
describe('本地存储诊断', () => {
  it('存储界面不显示服务商配置、密钥和数据结构', async () => {
    const fixture=diagnosticsFixture();fixture.provider={providerId:'openai',ready:true,keySource:'focalroom'};
    mount(vi.fn().mockResolvedValue(fixture));
    expect(await screen.findByText(/FocalRoom /)).toBeVisible();
    expect(screen.queryByText(/提供方|WAL|已应用迁移|JSON/)).not.toBeInTheDocument();
    expect(screen.queryByText(/密钥已配置/)).not.toBeInTheDocument();
  });
  it('先查看统计，主动复制已显示的快照，刷新更新数据', async () => {
    const user = userEvent.setup();
    const { query } = mount();
    await screen.findByText('存储统计');
    expect(within(screen.getByRole('row', { name: /图片缓存/ })).getByText('2.0 KiB')).toBeVisible();
    const copy = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    await user.click(screen.getByRole('button', { name: '复制支持信息' }));
    expect(JSON.parse(copy.mock.calls[0][0])).toEqual(diagnosticsFixture());
    expect(await screen.findByText(/支持信息已复制/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: '刷新统计' }));
    await waitFor(() => expect(query).toHaveBeenCalledTimes(2));
  });
  it('不完整统计显示下界，不把未知库文件显示为 0B', async () => {
    const data = diagnosticsFixture();
    data.storage.complete = false;
    data.storage.database = { bytes: 0, files: 0, skippedEntries: 1, complete: false };
    data.storage.cache = { ...data.storage.cache, skippedEntries: 1, complete: false };
    data.storage.totalBytes -= 4096;
    mount(vi.fn().mockResolvedValue(data));
    await screen.findByText('暂不可用');
    expect(screen.getByText('至少 2.0 KiB')).toBeVisible();
    expect(screen.getByText(/部分内容暂时无法统计/)).toBeVisible();
  });
  it('读取失败可重试，复制失败保留重试入口，Escape 关闭', async () => {
    const user = userEvent.setup();
    const { onClose } = mount(vi.fn().mockRejectedValueOnce(new Error('unavailable')).mockResolvedValue(diagnosticsFixture()));
    expect(await screen.findByRole('alert')).toHaveTextContent('读取失败，请重试');
    expect(screen.getByRole('button', { name: '复制支持信息' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: '刷新统计' }));
    await screen.findByText('存储统计');
    vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(new Error('denied'));
    await user.click(screen.getByRole('button', { name: '复制支持信息' }));
    expect(await screen.findByText(/无法复制/)).toBeVisible();
    expect(screen.getByRole('button', { name: '复制支持信息' })).toBeEnabled();
    expect(screen.queryByLabelText('脱敏诊断 JSON')).not.toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });
});
