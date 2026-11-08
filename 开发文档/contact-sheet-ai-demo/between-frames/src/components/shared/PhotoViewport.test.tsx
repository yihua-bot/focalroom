import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { PhotoViewport, fitZoom } from './PhotoViewport';
import type { Photo } from '../../domain/models';

const photo: Photo = { id: 'one', index: 1, name: 'original.png', src: 'thumbnail', previewSrc: 'preview', status: 'undecided', rating: 0, note: '' };
const actual = { ...fitZoom, mode: 'actual' as const };
afterEach(() => vi.stubGlobal('devicePixelRatio', 1));

it('RAW 派生预览不显示原图或嵌入预览标识', async () => {
  render(<PhotoViewport photo={{ ...photo, loadOriginal: async () => ({ src: 'bounded-preview', kind: 'derived_preview' }) }} zoom={actual} onChange={vi.fn()} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'bounded-preview'));
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'derived-preview');
  expect(screen.getByRole('note')).toHaveTextContent('RAW 派生预览 · 非全分辨率');
  expect(screen.queryByText(/^原图 ·/)).not.toBeInTheDocument();
  expect(screen.queryByText(/嵌入预览/)).not.toBeInTheDocument();
});

it('只在请求实际像素时读取原图，Retina 100% 按物理像素显示', async () => {
  vi.stubGlobal('devicePixelRatio', 2);
  const loadOriginal = vi.fn().mockResolvedValue({ src: 'original', kind: 'original' });
  const view = render(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={fitZoom} onChange={vi.fn()} />);
  expect(loadOriginal).not.toHaveBeenCalled();
  view.rerender(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={actual} onChange={vi.fn()} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'original'));
  const image = screen.getByRole('img');
  Object.defineProperties(image, { naturalWidth: { value: 2400 }, naturalHeight: { value: 1600 } });
  fireEvent.load(image);
  expect(image).toHaveStyle({ width: '1200px', height: '800px' });
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'original');
  view.rerender(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={fitZoom} onChange={vi.fn()} />);
  expect(screen.getByRole('img')).toHaveAttribute('src', 'preview');
});

it('内嵌 JPEG 按自身像素检视，标明与 RAW 全分辨率的区别，解码失败后回退', async () => {
  vi.stubGlobal('devicePixelRatio', 2);
  render(<PhotoViewport photo={{ ...photo, loadOriginal: async () => ({ src: 'embedded-jpeg', kind: 'embedded_preview' }) }} zoom={actual} onChange={vi.fn()} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'embedded-jpeg'));
  const image = screen.getByRole('img');
  Object.defineProperties(image, { naturalWidth: { value: 1936 }, naturalHeight: { value: 1288 } });
  fireEvent.load(image);
  expect(image).toHaveStyle({ width: '968px', height: '644px' });
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'embedded-preview');
  expect(screen.getByRole('note')).toHaveTextContent('RAW 内嵌 JPEG 预览 · 非 RAW 全分辨率');
  expect(screen.queryByText(/^原图 ·/)).not.toBeInTheDocument();
  fireEvent.error(image);
  expect(screen.getByRole('img')).toHaveAttribute('src', 'preview');
  expect(screen.queryByRole('note')).not.toBeInTheDocument();
});

it('原图授权失败或解码失败时明确回退，不显示原图标记', async () => {
  const loadOriginal = vi.fn().mockRejectedValue(new Error('原图失联；当前显示缓存预览'));
  const view = render(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={actual} onChange={vi.fn()} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('原图失联');
  expect(screen.getByRole('img')).toHaveAttribute('src', 'preview');
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'preview');
  view.unmount();
  render(<PhotoViewport photo={{ ...photo, loadOriginal: async () => ({ src: 'broken', kind: 'original' }) }} zoom={actual} onChange={vi.fn()} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'broken'));
  fireEvent.error(screen.getByRole('img'));
  expect(screen.getByRole('alert')).toHaveTextContent('原图无法解码');
  expect(screen.getByRole('img')).toHaveAttribute('src', 'preview');
});

it('切回适合屏幕后忽略迟到的原图响应', async () => {
  let resolve!: (value: { src: string; kind: 'original' | 'derived_preview' | 'embedded_preview' }) => void;
  const loadOriginal = () => new Promise<{ src: string; kind: 'original' | 'derived_preview' | 'embedded_preview' }>(done => { resolve = done; });
  const view = render(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={actual} onChange={vi.fn()} />);
  view.rerender(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={fitZoom} onChange={vi.fn()} />);
  await act(async () => resolve({ src: 'late-original', kind: 'original' }));
  expect(screen.getByRole('img')).toHaveAttribute('src', 'preview');
  expect(screen.queryByText(/原图 ·/)).not.toBeInTheDocument();
});

const managed = { src: 'data:color-managed', status: 'transformed', source: 'Display P3', display: 'ColorSync LCD' };

it('色彩管理版本顶替缓存预览，并说明它是重渲染的代理而非原图', () => {
  render(<PhotoViewport photo={photo} zoom={fitZoom} onChange={vi.fn()} managed={managed} />);
  expect(screen.getByRole('img')).toHaveAttribute('src', 'data:color-managed');
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'color-managed');
  expect(screen.getByRole('note')).toHaveTextContent('色彩管理预览 · 由缓存预览重渲染，非原图 · 已按原文件配置渲染到显示器色彩');
});

it('未知状态码原样上屏，不猜一句好看的说法', () => {
  render(<PhotoViewport photo={photo} zoom={fitZoom} onChange={vi.fn()} managed={{ ...managed, status: 'display_muted' }} />);
  expect(screen.getByRole('note')).toHaveTextContent('色彩管理预览 · 由缓存预览重渲染，非原图 · display_muted');
});

it('100% 检视仍用原图直通，并说明色彩管理此刻没在这张图上生效', async () => {
  const loadOriginal = vi.fn().mockResolvedValue({ src: 'original', kind: 'original' });
  render(<PhotoViewport photo={{ ...photo, loadOriginal }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'original'));
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'original');
  expect(screen.getByText('100% 检视仍走原图直通，未做显示器色彩变换')).toBeInTheDocument();
  expect(screen.queryByText(/色彩管理预览/)).not.toBeInTheDocument();
});

it('开关开着时 100% 用全分辨率的显示器渲染，不再谎称直通', async () => {
  const loadOriginal = vi.fn().mockResolvedValue({ src: 'original', kind: 'original' });
  const loadManagedOriginal = vi.fn().mockResolvedValue({ src: 'full-res-managed', kind: 'display_managed' });
  render(<PhotoViewport photo={{ ...photo, loadOriginal, loadManagedOriginal }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'full-res-managed'));
  expect(loadOriginal).not.toHaveBeenCalled();
  expect(screen.getByTestId('photo-viewport')).toHaveAttribute('data-source', 'display-managed');
  expect(screen.getByRole('note')).toHaveTextContent('原图 · 显示器色彩 · 全分辨率 · 已按显示器 ICC 渲染');
  expect(screen.queryByText(/仍走原图直通/)).not.toBeInTheDocument();
});

it('全分辨率渲染还在进行时说清在等的是什么', async () => {
  let resolve!: (value: { src: string; kind: 'display_managed' }) => void;
  const loadManagedOriginal = () => new Promise<{ src: string; kind: 'display_managed' }>(done => { resolve = done; });
  render(<PhotoViewport photo={{ ...photo, loadManagedOriginal }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  expect(await screen.findByRole('status')).toHaveTextContent('正在按显示器色彩渲染全分辨率原图…');
  await act(async () => resolve({ src: 'full-res-managed', kind: 'display_managed' }));
  const image = screen.getByRole('img');
  expect(image).toHaveAttribute('src', 'full-res-managed');
  Object.defineProperties(image, { naturalWidth: { value: 2400 }, naturalHeight: { value: 1600 } });
  fireEvent.load(image);
  expect(screen.queryByText('正在按显示器色彩渲染全分辨率原图…')).not.toBeInTheDocument();
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});

it('全分辨率渲染被拒时把原因说清，并回退到色彩管理预览而不是直通原图', async () => {
  const loadOriginal = vi.fn().mockResolvedValue({ src: 'original', kind: 'original' });
  const loadManagedOriginal = vi.fn().mockRejectedValue(new Error('没有可用的全分辨率原图，未做显示器渲染：该照片的来源是派生预览'));
  render(<PhotoViewport photo={{ ...photo, loadOriginal, loadManagedOriginal }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('没有可用的全分辨率原图，未做显示器渲染');
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'data:color-managed'));
  expect(loadOriginal).not.toHaveBeenCalled();
  expect(screen.getByRole('note')).toHaveTextContent('色彩管理预览 · 由缓存预览重渲染，非原图');
  expect(screen.queryByText(/仍走原图直通/)).not.toBeInTheDocument();
});

it('关掉开关后 100% 回到原图直通，重新打开又走全分辨率渲染', async () => {
  const loadOriginal = vi.fn().mockResolvedValue({ src: 'original', kind: 'original' });
  const loadManagedOriginal = vi.fn().mockResolvedValue({ src: 'full-res-managed', kind: 'display_managed' });
  const view = render(<PhotoViewport photo={{ ...photo, loadOriginal, loadManagedOriginal }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'full-res-managed'));
  view.rerender(<PhotoViewport photo={{ ...photo, loadOriginal, loadManagedOriginal }} zoom={actual} onChange={vi.fn()} managed={null} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'original'));
  view.rerender(<PhotoViewport photo={{ ...photo, loadOriginal, loadManagedOriginal }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'full-res-managed'));
  expect(loadManagedOriginal).toHaveBeenCalledTimes(2);
});

it('切换色彩模式时立即隐藏上一模式的全分辨率像素，等待新结果',async()=>{
 let resolve!: (value:{src:string;kind:'display_managed'})=>void;
 const loadOriginal=vi.fn().mockResolvedValue({src:'original',kind:'original'});
 const loadManagedOriginal=()=>new Promise<{src:string;kind:'display_managed'}>(done=>{resolve=done;});
 const item={...photo,loadOriginal,loadManagedOriginal};
 const view=render(<PhotoViewport photo={item} zoom={actual} onChange={vi.fn()}/>);
 await waitFor(()=>expect(screen.getByRole('img')).toHaveAttribute('src','original'));
 view.rerender(<PhotoViewport photo={item} zoom={actual} onChange={vi.fn()} managed={{...managed,pixelSource:'original'}}/>);
 expect(screen.getByRole('img')).toHaveAttribute('src','data:color-managed');
 expect(screen.queryByText(/色彩由系统显示管理/)).not.toBeInTheDocument();
 expect(screen.getByRole('note')).toHaveTextContent('从原图生成，非全分辨率');
 await act(async()=>resolve({src:'new-managed',kind:'display_managed'}));
 expect(screen.getByRole('img')).toHaveAttribute('src','new-managed');
});

it('同 ID 更换预览版本后不展示旧的全分辨率画面',async()=>{
 let resolve!:(value:{src:string;kind:'original'})=>void;
 const loadOriginal=vi.fn().mockResolvedValueOnce({src:'old-original',kind:'original'}).mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));
 const view=render(<PhotoViewport photo={{...photo,loadOriginal}} zoom={actual} onChange={vi.fn()}/>);
 await waitFor(()=>expect(screen.getByRole('img')).toHaveAttribute('src','old-original'));
 view.rerender(<PhotoViewport photo={{...photo,previewSrc:'new-preview',loadOriginal}} zoom={actual} onChange={vi.fn()}/>);
 expect(screen.getByRole('img')).toHaveAttribute('src','new-preview');
 await act(async()=>resolve({src:'new-original',kind:'original'}));
 expect(screen.getByRole('img')).toHaveAttribute('src','new-original');
});

it('全分辨率渲染按 1:1 检视并说明色彩来路，坏掉时回指渲染版本身', async () => {
  render(<PhotoViewport photo={{ ...photo, loadManagedOriginal: async () => ({ src: 'full-res-managed', kind: 'display_managed' }) }} zoom={actual} onChange={vi.fn()} managed={managed} />);
  await waitFor(() => expect(screen.getByRole('img')).toHaveAttribute('src', 'full-res-managed'));
  const image = screen.getByRole('img');
  Object.defineProperties(image, { naturalWidth: { value: 2400 }, naturalHeight: { value: 1600 } });
  fireEvent.load(image);
  expect(image).toHaveStyle({ width: '2400px', height: '1600px' });
  expect(screen.getByText(/色彩已按本机显示器配置渲染并标注/)).toBeInTheDocument();
  expect(screen.queryByText(/色彩由系统显示管理/)).not.toBeInTheDocument();
  fireEvent.error(image);
  expect(await screen.findByRole('alert')).toHaveTextContent('原图 · 显示器色彩无法解码');
  expect(screen.getByRole('img')).toHaveAttribute('src', 'data:color-managed');
});
