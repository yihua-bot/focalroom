import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { HistoricalPhotoObservation } from '../../domain/revisionContext';
import { HistoricalObservations } from './HistoricalObservations';

it('原图相同也必须核对实际预览，旧版和不可读的观察不能配上当前图像', () => {
  const base: HistoricalPhotoObservation = {
    photoId:'photo',photoHash:'a'.repeat(64),previewSha256:'b'.repeat(64),photoName:'fixture.jpg',
    thumbnailPath:'/fixture.jpg',modelId:'fixture',promptVersion:'fixture',taskId:'parent',callId:'inspect',
    visibleFacts:['历史画面事实'],interpretation:'历史解释',uncertainty:'仍待作者判断',currentHashMatches:true,
  };
  render(<HistoricalObservations total={4} observations={(['verified','mismatch','unrecorded','unreadable'] as const)
    .map((status,index)=>({...base,photoId:'photo-'+index,currentPixelVersion:status}))} />);
  expect(screen.getAllByRole('img')).toHaveLength(1);
  expect(screen.getByText('看的是这张图（版本 bbbbbbbb）')).toBeVisible();
  expect(screen.getByText('当前预览已变化，这条历史观察不代表现在的图像。')).toBeVisible();
  expect(screen.getByText('来自更早的缓存，图像版本没记过')).toBeVisible();
  expect(screen.getByText('当前预览读不出来，这次没能核对图像')).toBeVisible();
  expect(screen.getAllByText('历史画面事实')).toHaveLength(4);
});

const row = (photoId: string, status: HistoricalPhotoObservation['currentPixelVersion']): HistoricalPhotoObservation => ({
  photoId, photoHash: 'a'.repeat(64), previewSha256: 'b'.repeat(64), photoName: `${photoId}.jpg`,
  thumbnailPath: '/fixture.jpg', modelId: 'fixture', promptVersion: 'fixture', taskId: 'parent', callId: 'inspect',
  visibleFacts: ['历史画面事实'], interpretation: '历史解释', uncertainty: '仍待作者判断', currentHashMatches: true,
  currentPixelVersion: status,
});

it('一次修完这一页里读不出来的预览，结局逐张说实话', async () => {
  const repair = vi.fn().mockResolvedValue([
    { photoId: 'bad-1', outcome: 'repaired', rebuilt: ['preview'], detail: '' },
    { photoId: 'bad-2', outcome: 'original_missing', rebuilt: [], detail: '' },
  ]);
  render(<HistoricalObservations total={4} onRepair={repair} observations={[
    row('bad-1', 'unreadable'), row('ok', 'verified'), row('bad-2', 'unreadable'), row('old', 'unrecorded'),
  ]} />);
  fireEvent.click(screen.getByRole('button', { name: '一次修复这 2 张本地预览' }));
  expect(await screen.findByText('已按原图重新算出本地预览，下次让 AI 看这张就能送上')).toBeVisible();
  expect(screen.getByText('原图不在这台机器上，要先重新定位这张照片')).toBeVisible();
  expect(repair).toHaveBeenCalledWith(['bad-1', 'bad-2']);
  expect(screen.queryByText('这一步没能完成，本地文件没有改动')).toBeNull();
  // 修好的那张不再挂着"读不出来"，没修成的那张必须还挂着：改口只改这一张。
  expect(screen.getByText('本地预览现在读得出来了；这条观察还是按修复前那份读的')).toBeVisible();
  expect(screen.getAllByText('当前预览读不出来，这次没能核对图像')).toHaveLength(1);
});

it('文件本来就能读也算现在读得出，但不改别的张的说法', async () => {
  const repair = vi.fn().mockResolvedValue([{ photoId: 'bad', outcome: 'healthy', rebuilt: [], detail: '' }]);
  render(<HistoricalObservations total={2} onRepair={repair} observations={[row('bad', 'unreadable'), row('other', 'mismatch')]} />);
  fireEvent.click(screen.getByRole('button', { name: '修复本地预览' }));
  expect(await screen.findByText('这两份本地文件本来就能读，没有改写')).toBeVisible();
  expect(screen.getByText('本地预览现在读得出来了；这条观察还是按修复前那份读的')).toBeVisible();
  expect(screen.getByText('当前预览已变化，这条历史观察不代表现在的图像。')).toBeVisible();
});

it('只有一张坏时也给出动作；这一步没做成不谎报修好', async () => {
  const failed = vi.fn().mockRejectedValue(new Error('disk'));
  render(<HistoricalObservations total={1} onRepair={failed} observations={[row('bad', 'unreadable')]} />);
  fireEvent.click(screen.getByRole('button', { name: '修复本地预览' }));
  expect(await screen.findByText('这一步没能完成，本地文件没有改动')).toBeVisible();
  expect(screen.queryByText('已按原图重新算出本地预览，下次让 AI 看这张就能送上')).toBeNull();
});

it('没有坏预览就不放修复按钮', () => {
  render(<HistoricalObservations total={2} observations={[row('a', 'verified'), row('b', 'mismatch')]} />);
  expect(screen.queryByRole('button', { name: /修复/ })).toBeNull();
});
