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
