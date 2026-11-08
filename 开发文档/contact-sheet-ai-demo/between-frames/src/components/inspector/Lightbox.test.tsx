import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Lightbox } from './Lightbox';
import type { Photo } from '../../domain/models';

const { invoke, isTauri } = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke, isTauri }));

const photos: Photo[] = [
  { id: 'a', name: '晨光.jpg', status: 'undecided', rating: 0, src: 'a.jpg' },
  { id: 'b', name: '巷口.jpg', status: 'keep', rating: 4, src: 'b.jpg' },
  { id: 'c', name: '窗边.jpg', status: 'reject', rating: 0, src: 'c.jpg' },
] as unknown as Photo[];

function open(overrides: Partial<Parameters<typeof Lightbox>[0]> = {}) {
  const props = {
    photos, activeId: 'a', onNavigate: vi.fn(), onClose: vi.fn(),
    onDecide: vi.fn(), onRate: vi.fn(), ...overrides,
  };
  const view = render(<Lightbox {...props} />);
  return { ...view, props };
}

describe('v1.1 沉浸审片', () => {
  it('灯箱内 K/X/U 与数字键直接决策评分', async () => {
    const { props } = open();
    const body = document.querySelector('.lightbox-body')!;
    fireEvent.keyDown(body, { key: 'k' });
    fireEvent.keyDown(body, { key: '5' });
    expect(props.onDecide).toHaveBeenCalledWith('a', 'keep');
    expect(props.onRate).toHaveBeenCalledWith('a', 5);
    fireEvent.keyDown(body, { key: 'x' });
    expect(props.onDecide).toHaveBeenCalledWith('a', 'reject');
  });
  it('状态区实时反映当前照片的状态与评分', () => {
    open({ activeId: 'b' });
    const status = document.querySelector('.review-status')!;
    expect(status.textContent).toContain('已保留');
    expect(status.textContent).toContain('★★★★');
  });
  it('胶片条展示全部照片、状态标识与当前高亮，点击跳转', async () => {
    const { props } = open();
    const strip = screen.getByLabelText('胶片条导航');
    expect(strip.querySelectorAll('button')).toHaveLength(3);
    expect(strip.querySelector('[aria-current="true"]')?.getAttribute('aria-label')).toContain('晨光');
    expect(strip.querySelector('[data-status="keep"]')).toBeTruthy();
    expect(strip.querySelector('[data-status="reject"]')).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: /巷口/ }));
    expect(props.onNavigate).toHaveBeenCalledWith(photos[1]);
  });
  it('按钮操作与键盘等价（无键盘焦点依赖）', async () => {
    const { props } = open();
    await userEvent.click(screen.getByRole('button', { name: '淘汰 X' }));
    expect(props.onDecide).toHaveBeenCalledWith('a', 'reject');
    await userEvent.click(screen.getByRole('button', { name: '评 3 星' }));
    expect(props.onRate).toHaveBeenCalledWith('a', 3);
  });
});

describe('灯箱里的色彩管理', () => {
  afterEach(() => { invoke.mockReset(); isTauri.mockReturnValue(true); });

  // 胶片条每张照片各有一张缩略图，检视画面必须从视口里取，否则断言命中缩略条。
  const stage = () => within(screen.getByTestId('photo-viewport'));

  it('开启后把按显示器渲染的图像真的显示出来，再点一次退回缓存预览', async () => {
    invoke.mockResolvedValue({ status: 'transformed', detail: null, sourceProfile: 'Display P3', displayProfile: 'ColorSync LCD', pixelSource: 'original', dataUrl: 'data:image/jpeg;base64,/9j/2Q==' });
    open({ projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: '色彩管理' }));
    await waitFor(() => expect(stage().getByRole('img')).toHaveAttribute('src', 'data:image/jpeg;base64,/9j/2Q=='));
    expect(invoke).toHaveBeenCalledWith('get_color_managed_preview', { projectId: 'p1', photoId: 'a' });
    expect(screen.getByText('已按原文件配置渲染到显示器色彩 · Display P3 → ColorSync LCD')).toBeInTheDocument();
    expect(stage().getByRole('note')).toHaveTextContent('色彩管理预览 · 从原图生成，非全分辨率');
    await userEvent.click(screen.getByRole('button', { name: '色彩管理已开启' }));
    expect(stage().getByRole('img')).toHaveAttribute('src', 'a.jpg');
    expect(stage().queryByRole('note')).not.toBeInTheDocument();
  });

  it('后端说没变换时画面不动，原因直接上屏', async () => {
    invoke.mockResolvedValue({ status: 'container_unsupported', detail: '该容器暂不支持读取嵌入色彩配置（HEIF、RAW、BigTIFF 等）', sourceProfile: '无法确定', displayProfile: 'ColorSync LCD', dataUrl: null });
    open({ projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: '色彩管理' }));
    expect(await screen.findByText(/该格式读不出色彩配置，未做色彩变换/)).toHaveTextContent('HEIF');
    expect(stage().getByRole('img')).toHaveAttribute('src', 'a.jpg');
    expect(screen.getByRole('button', { name: '色彩管理' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('只有桌面版能做本机显示器渲染，网页版不把话说满', async () => {
    isTauri.mockReturnValue(false);
    open({ projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: '色彩管理' }));
    expect(await screen.findByText('色彩管理预览仅桌面版本机可用')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
    expect(stage().getByRole('img')).toHaveAttribute('src', 'a.jpg');
  });

  it('换片后不沿用上一张的色彩管理版本', async () => {
    invoke.mockResolvedValue({ status: 'transformed', detail: null, sourceProfile: 'sRGB', displayProfile: 'ColorSync LCD', pixelSource: 'original', dataUrl: 'data:image/jpeg;base64,/9j/2Q==' });
    const view = open({ projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: '色彩管理' }));
    await waitFor(() => expect(stage().getByRole('img')).toHaveAttribute('src', 'data:image/jpeg;base64,/9j/2Q=='));
    view.rerender(<Lightbox photos={photos} activeId="b" onNavigate={vi.fn()} onClose={vi.fn()} onDecide={vi.fn()} onRate={vi.fn()} projectId="p1" />);
    expect(stage().getByRole('img')).toHaveAttribute('src', 'b.jpg');
    expect(screen.getByRole('button', { name: '色彩管理' })).toHaveAttribute('aria-pressed', 'false');
  });
});
