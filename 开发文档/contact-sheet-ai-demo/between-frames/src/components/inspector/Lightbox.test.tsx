import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Lightbox } from './Lightbox';
import type { Photo } from '../../domain/models';

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
