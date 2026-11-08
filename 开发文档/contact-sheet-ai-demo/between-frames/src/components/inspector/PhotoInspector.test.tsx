import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { Photo, PreviewRepairOutcome } from '../../domain/models';
import { PhotoInspector } from './PhotoInspector';

const photo = { id: 'a', name: 'a.jpg', src: '/a.jpg', index: 0, status: 'undecided', note: '', rating: 0, width: 100, height: 80 } as Photo;
const noop = () => undefined;

it('检查器里可以就地重算这张的本地预览，结局说实话', async () => {
  const onRepairPreview = vi.fn().mockResolvedValue([{ photoId: 'a', outcome: 'repaired', rebuilt: ['preview'], detail: '' }] as PreviewRepairOutcome[]);
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onRepairPreview={onRepairPreview} />);
  fireEvent.click(screen.getByRole('button', { name: '修复本地预览' }));
  expect(await screen.findByText('已按原图重新算出本地预览，下次让 AI 看这张就能送上')).toBeVisible();
  expect(onRepairPreview).toHaveBeenCalledOnce();
  expect(screen.getByText(/原图不动/)).toBeVisible();
});

it('这一步没做成不显示成已修好', async () => {
  const failed = vi.fn().mockRejectedValue(new Error('disk'));
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onRepairPreview={failed} />);
  fireEvent.click(screen.getByRole('button', { name: '修复本地预览' }));
  expect(await screen.findByText('这一步没能完成，本地文件没有改动')).toBeVisible();
  expect(screen.queryByText('已按原图重新算出本地预览，下次让 AI 看这张就能送上')).toBeNull();
});

it('没有这个能力就不放按钮', () => {
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} />);
  expect(screen.queryByRole('button', { name: '修复本地预览' })).toBeNull();
});

const inspectResult = (outcome: string): PreviewRepairOutcome => ({ photoId: 'a', outcome, rebuilt: [], detail: '' });
const withInspect = (outcome: string) => vi.fn().mockResolvedValue(inspectResult(outcome));

it('能只读检查这张，读得出来就不摆重算按钮', async () => {
  const onInspectPreview = withInspect('healthy');
  const onRepairPreview = vi.fn();
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onInspectPreview={onInspectPreview} onRepairPreview={onRepairPreview} />);
  expect(screen.queryByRole('button', { name: '修复本地预览' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: '检查这张的本地文件' }));
  expect(await screen.findByText('这张的本地预览现在读得出来，不用重算。')).toBeVisible();
  expect(onInspectPreview).toHaveBeenCalledOnce();
  expect(screen.queryByRole('button', { name: '修复本地预览' })).toBeNull();
  expect(onRepairPreview).not.toHaveBeenCalled();
});

it('检查出读不出来才摆重算按钮，重算完按现在的状态改口', async () => {
  const onInspectPreview = vi.fn().mockResolvedValueOnce(inspectResult('needs_repair')).mockResolvedValueOnce(inspectResult('healthy'));
  const onRepairPreview = vi.fn().mockResolvedValue([inspectResult('repaired')]);
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onInspectPreview={onInspectPreview} onRepairPreview={onRepairPreview} />);
  fireEvent.click(screen.getByRole('button', { name: '检查这张的本地文件' }));
  expect(await screen.findByText('这张的本地预览现在读不出来，可以只读原图重算。')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: '修复本地预览' }));
  expect(await screen.findByText('已按原图重新算出本地预览，下次让 AI 看这张就能送上')).toBeVisible();
  expect(await screen.findByText('这张的本地预览现在读得出来，不用重算。')).toBeVisible();
  expect(onInspectPreview).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole('button', { name: '修复本地预览' })).toBeNull();
});

it('原图不在这台机器上就说原图不在，不摆重算按钮', async () => {
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onInspectPreview={withInspect('original_missing')} onRepairPreview={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: '检查这张的本地文件' }));
  expect(await screen.findByText('原图不在这台机器上，要先重新定位这张照片')).toBeVisible();
  expect(screen.queryByRole('button', { name: '修复本地预览' })).toBeNull();
});

it('预览图加载不出来就自动只读检查一次', async () => {
  const onInspectPreview = withInspect('needs_repair');
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onInspectPreview={onInspectPreview} onRepairPreview={vi.fn()} />);
  fireEvent.error(screen.getByRole('img'));
  expect(await screen.findByText('这张的本地预览现在读不出来，可以只读原图重算。')).toBeVisible();
  expect(onInspectPreview).toHaveBeenCalledOnce();
});

it('检查这一步没做成不说成读得出来', async () => {
  const onInspectPreview = vi.fn().mockRejectedValue(new Error('disk'));
  render(<PhotoInspector photo={photo} busy={false} onDecide={noop} onRate={noop} onNote={noop} onEnlarge={noop} onInspectPreview={onInspectPreview} onRepairPreview={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: '检查这张的本地文件' }));
  expect(await screen.findByText('这一步没能完成，本地文件没有改动')).toBeVisible();
  expect(screen.queryByText('这张的本地预览现在读得出来，不用重算。')).toBeNull();
  expect(screen.queryByRole('button', { name: '修复本地预览' })).toBeNull();
});
