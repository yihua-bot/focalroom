import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SourceChangeList } from './SourceChangeList';
import type { DesktopProjectRepository, ImportChange } from '../../domain/models';

const change = (over: Partial<ImportChange> = {}): ImportChange => ({
  photoId: 'photo-1',
  path: '/Library/frame-007.jpg',
  filename: 'frame-007.jpg',
  previousSha256: 'a'.repeat(64),
  incomingSha256: 'b'.repeat(64),
  previousWidth: 3000,
  previousHeight: 2000,
  previousSize: 4_194_304,
  incomingWidth: 4000,
  incomingHeight: 3000,
  incomingSize: 8_388_608,
  ...over,
});
const repository = (over: Record<string, unknown> = {}) => ({
  applyPhotoChange: vi.fn().mockResolvedValue({ id: 'p1', revision: 4 }),
  ...over,
}) as unknown as DesktopProjectRepository;

describe('原图被外部就地改写', () => {
  it('把变化说清成两个尺寸，确认前不谎称已经落库', () => {
    render(<SourceChangeList repository={repository()} projectId="p1" changes={[change()]} />);
    expect(screen.getByText('检测到 1 张照片的原图被就地改写（在 Lightroom、Capture One 等软件里保存回原文件就是这个形状）。记录还没有变动，确认后才会换成文件现在的内容；选片、备注和序列位置都保留。')).toBeInTheDocument();
    expect(screen.getByText('库里记的 3000×2000 · 4.0 MB → 文件现在 4000×3000 · 8.0 MB')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '更新这条记录' })).toBeEnabled();
    expect(screen.queryByText('已更新这条记录')).not.toBeInTheDocument();
  });

  it('没有变化时整段不出现', () => {
    render(<SourceChangeList repository={repository()} projectId="p1" changes={[]} />);
    expect(screen.queryByRole('button', { name: '更新这条记录' })).not.toBeInTheDocument();
  });

  it('确认后只发一条命令，用这条记录在册的照片而不是界面给的路径', async () => {
    const applyPhotoChange = vi.fn().mockResolvedValue({ id: 'p1', revision: 4 });
    const onApplied = vi.fn();
    render(<SourceChangeList repository={repository({ applyPhotoChange })} projectId="p1" changes={[change()]} onApplied={onApplied} />);
    await userEvent.click(screen.getByRole('button', { name: '更新这条记录' }));
    expect(applyPhotoChange).toHaveBeenCalledWith('p1', 'photo-1');
    expect(applyPhotoChange.mock.calls[0][2]).toBeUndefined();
    expect(await screen.findByText('已更新这条记录')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '更新这条记录' })).not.toBeInTheDocument();
    expect(onApplied).toHaveBeenCalledTimes(1);
  });

  it('后端拒掉时说清原因，不冒充已更新', async () => {
    const applyPhotoChange = vi.fn().mockRejectedValue(new Error('这条记录已经是文件现在的内容，无需更新'));
    const onApplied = vi.fn();
    render(<SourceChangeList repository={repository({ applyPhotoChange })} projectId="p1" changes={[change()]} onApplied={onApplied} />);
    await userEvent.click(screen.getByRole('button', { name: '更新这条记录' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('这条记录已经是文件现在的内容，无需更新');
    expect(screen.queryByText('已更新这条记录')).not.toBeInTheDocument();
    expect(onApplied).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: '更新这条记录' })).toBeEnabled();
  });

  it('一张正在确认时另一条按住，避免同时改两条记录', async () => {
    let release: (() => void) | undefined;
    const applyPhotoChange = vi.fn(() => new Promise(resolve => { release = () => resolve({ id: 'p1', revision: 4 }); }));
    render(<SourceChangeList repository={repository({ applyPhotoChange })} projectId="p1" changes={[change(), change({ photoId: 'photo-2', filename: 'frame-008.jpg' })]} />);
    const buttons = screen.getAllByRole('button', { name: '更新这条记录' });
    await userEvent.click(buttons[0]);
    await waitFor(() => expect(applyPhotoChange).toHaveBeenCalledTimes(1));
    expect(buttons[1]).toBeDisabled();
    release?.();
    expect(await screen.findByText('已更新这条记录')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: '更新这条记录' })).toHaveLength(1);
  });

  it('没有这条通道的仓储只报事实，不给无法兑现的按钮', () => {
    render(<SourceChangeList repository={repository({ applyPhotoChange: undefined })} projectId="p1" changes={[change()]} />);
    expect(screen.getByText(/frame-007.jpg/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '更新这条记录' })).not.toBeInTheDocument();
  });
});
