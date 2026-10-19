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
  it('账户服务显示 FocalRoom 和 DeepSeek，不声称用户配置了 OpenAI 密钥', async () => {
    const fixture=diagnosticsFixture();fixture.provider={providerId:'openai',ready:true,keySource:'focalroom'};
    mount(vi.fn().mockResolvedValue(fixture));
    expect(await screen.findByText(/提供方 FocalRoom AI · DeepSeek/)).toBeVisible();
    expect(screen.queryByText(/密钥已配置/)).not.toBeInTheDocument();
  });
  it('先查看统计，主动复制已显示的快照，刷新更新数据', async () => {
    const user = userEvent.setup();
    const { query } = mount();
    await screen.findByText('存储统计');
    expect(within(screen.getByRole('row', { name: /图片缓存/ })).getByText('2.0 KiB')).toBeVisible();
    const copy = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    await user.click(screen.getByRole('button', { name: '复制诊断信息' }));
    expect(JSON.parse(copy.mock.calls[0][0])).toEqual(diagnosticsFixture());
    expect(await screen.findByText(/诊断信息已复制/)).toBeVisible();
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
    expect(screen.getByText(/部分条目不可读取/)).toBeVisible();
  });
  it('读取失败可重试，复制失败保留手动复制入口，Escape 关闭', async () => {
    const user = userEvent.setup();
    const { onClose } = mount(vi.fn().mockRejectedValueOnce(new Error('unavailable')).mockResolvedValue(diagnosticsFixture()));
    expect(await screen.findByRole('alert')).toHaveTextContent('读取失败，请重试');
    expect(screen.getByRole('button', { name: '复制诊断信息' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: '刷新统计' }));
    await screen.findByText('存储统计');
    vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(new Error('denied'));
    await user.click(screen.getByRole('button', { name: '复制诊断信息' }));
    expect(await screen.findByText(/剪贴板写入失败/)).toBeVisible();
    await user.click(screen.getByText('检查诊断 JSON'));
    expect(screen.getByLabelText('脱敏诊断 JSON')).toBeVisible();
    await user.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });
});
