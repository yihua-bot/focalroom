import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { CanvasPhotoPicker } from './CanvasPhotoPicker';
import { setCopyLocale } from '../../app/copy';
import type { Photo } from '../../domain/models';
const photos: Photo[] = ['A', 'B', 'C'].map((name, index) => ({ id: name, name, index, src: '/photo.jpg', status: 'undecided', rating: 0, note: '' }));
afterEach(() => setCopyLocale('zh'));

it('标记并禁选已在画布的照片，全选只包含可添加照片', async () => {
  const user=userEvent.setup(),onAdd=vi.fn();
  render(<CanvasPhotoPicker photos={photos} sequencePhotoIds={['A','B','C']} selectedPhotoIds={[]} placedPhotoIds={['A']} busy={false} onClose={vi.fn()} onAdd={onAdd}/>);
  expect(screen.getByRole('checkbox',{name:'A已在画布'})).toBeDisabled();
  await user.click(screen.getByRole('button',{name:'全选可添加照片'}));
  await user.click(screen.getByRole('button',{name:'添加 2 张到画布'}));
  expect(onAdd).toHaveBeenCalledWith(['B','C']);
});

it('作品序列、全库和工作台选区彼此独立，按来源顺序添加而非点击顺序', async () => {
  const onAdd = vi.fn(); const user = userEvent.setup();
  render(<CanvasPhotoPicker photos={photos} sequencePhotoIds={['C', 'A']} selectedPhotoIds={['B']} busy={false} onClose={vi.fn()} onAdd={onAdd} />);
  expect(screen.queryByRole('checkbox', { name: 'B' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('checkbox', { name: 'A' }));
  await user.click(screen.getByRole('checkbox', { name: 'C' }));
  await user.click(screen.getByRole('button', { name: '添加 2 张到画布' }));
  expect(onAdd).toHaveBeenLastCalledWith(['C', 'A']);
  await user.click(screen.getByRole('button', { name: '工作台选区 · 1' }));
  expect(screen.getByRole('button', { name: '添加 0 张到画布' })).toBeDisabled();
  expect(screen.getAllByRole('checkbox')).toHaveLength(1);
  await user.click(screen.getByRole('checkbox', { name: 'B' }));
  await user.click(screen.getByRole('button', { name: '添加 1 张到画布' }));
  expect(onAdd).toHaveBeenLastCalledWith(['B']);
  expect(photos.map(p => p.status)).toEqual(['undecided', 'undecided', 'undecided']);
});

it('大量照片分页显示；搜索不丢失已选照片，限制最多300张', async () => {
  const user = userEvent.setup(); const many = Array.from({ length: 301 }, (_, index) => ({ ...photos[0], id: String(index), name: `photo-${index}` }));
  render(<CanvasPhotoPicker photos={many} sequencePhotoIds={[]} selectedPhotoIds={[]} busy={false} onClose={vi.fn()} onAdd={vi.fn()} />);
  expect(screen.getAllByRole('checkbox')).toHaveLength(60);
  expect(screen.getByRole('button', { name: '全选可添加照片' })).toBeDisabled();
  await user.click(screen.getByRole('checkbox', { name: 'photo-0' }));
  await user.type(screen.getByRole('searchbox'), 'photo-300');
  expect(screen.getAllByRole('checkbox')).toHaveLength(1);
  expect(screen.getByRole('button', { name: '添加 1 张到画布' })).toBeEnabled();
  await user.click(screen.getByRole('checkbox', { name: 'photo-300' }));
  expect(screen.getByRole('button', { name: '添加 2 张到画布' })).toBeEnabled();
});

it('英文界面仍清楚区分照片来源', () => {
  setCopyLocale('en');
  render(<CanvasPhotoPicker photos={photos} sequencePhotoIds={['A']} selectedPhotoIds={['B']} busy={false} onClose={vi.fn()} onAdd={vi.fn()} />);
  expect(screen.getByRole('button', { name: 'Work sequence · 1' })).toHaveAttribute('aria-pressed', 'true');
  expect(screen.getByRole('button', { name: 'Workbench selection · 1' })).toBeVisible();
});
