import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { VersionsDialog } from './VersionsDialog';
import type { Project } from '../../domain/models';

const photos = [
  { id: 'a', name: '晨光.jpg', status: 'keep' as const, rating: 4, note: '' },
  { id: 'b', name: '巷口.jpg', status: 'keep' as const, rating: 5, note: '' },
  { id: 'c', name: '窗边.jpg', status: 'undecided' as const, rating: 0, note: '' },
];
const project = {
  id: 'p1', title: 't', brief: '', revision: 9,
  photos, sequence: ['b', 'c'],
  versions: [
    { id: 'v2', label: '第二轮', basedOnRevision: 8, createdAt: '2026-10-14T02:00:00Z', photoIds: ['c', 'a'], photos: [
      { photoId: 'a', status: 'keep' as const, rating: 3, note: '' },
      { photoId: 'c', status: 'keep' as const, rating: 0, note: '' },
    ] },
    { id: 'v1', label: '初选', basedOnRevision: 4, createdAt: '2026-10-14T01:00:00Z', photoIds: ['a', 'b'], photos: [
      { photoId: 'a', status: 'keep' as const, rating: 4, note: '' },
      { photoId: 'b', status: 'keep' as const, rating: 4, note: '原备注' },
    ] },
  ],
} as unknown as Project;

describe('v1.1 版本对比', () => {
  it('对比两个版本：新增/移除/移位/人工判断变化', async () => {
    render(<VersionsDialog project={project} busy={false} onSave={vi.fn()} onRestore={vi.fn()} onClose={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('tab', { name: '版本对比' }));
    await user.selectOptions(screen.getByRole('combobox', { name: '基准版本' }), 'v1');
    await user.selectOptions(screen.getByRole('combobox', { name: '对比版本' }), 'v2');
    expect(screen.getByText('初选 → 第二轮')).toBeInTheDocument();
    // v2 相对 v1：新增 c、移除 b、a 从第 1 位移到第 2 位、a 评分 4→3、（b 被移除不计状态变化）。
    const summary = screen.getByText(/新增 1 张/, { exact: false }).closest('p')!;
    expect(summary.textContent).toContain('移除 1 张');
    expect(summary.textContent).toContain('移位 1 张');
    expect(summary.textContent).toContain('状态/评分/备注变化 1 张');
    expect(screen.getByText('窗边.jpg')).toBeInTheDocument();
    expect(screen.getByText('巷口.jpg')).toBeInTheDocument();
    expect(screen.getByText('晨光.jpg：第 1 位 → 第 2 位')).toBeInTheDocument();
    expect(screen.getByText('晨光.jpg：评分 4 → 3')).toBeInTheDocument();
  });
  it('版本与当前序列对比', async () => {
    render(<VersionsDialog project={project} busy={false} onSave={vi.fn()} onRestore={vi.fn()} onClose={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('tab', { name: '版本对比' }));
    await user.selectOptions(screen.getByRole('combobox', { name: '基准版本' }), 'v2');
    await user.selectOptions(screen.getByRole('combobox', { name: '对比版本' }), 'current');
    // 当前 [b,c] vs v2 [c,a]：新增 b、移除 a、c 从第 1 位到第 2 位。
    expect(screen.getByText('第二轮 → 当前序列')).toBeInTheDocument();
    const summary = screen.getByText(/新增 1 张/, { exact: false }).closest('p')!;
    expect(summary.textContent).toContain('移除 1 张');
    expect(summary.textContent).toContain('移位 1 张');
  });
  it('未选齐两个快照时给出引导且不报错', async () => {
    render(<VersionsDialog project={project} busy={false} onSave={vi.fn()} onRestore={vi.fn()} onClose={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('tab', { name: '版本对比' }));
    await user.selectOptions(screen.getByRole('combobox', { name: '基准版本' }), 'v1');
    expect(screen.getByText('选择要比较的版本。')).toBeInTheDocument();
  });
  it('恢复预览模式保留原语义', async () => {
    render(<VersionsDialog project={project} busy={false} onSave={vi.fn()} onRestore={vi.fn()} onClose={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /第二轮/ }));
    expect(screen.getByText(/恢复后 2 张：加入 1 张，移出 1 张，位置变化 1 张/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '确认恢复此版本' })).toBeEnabled();
  });
});
