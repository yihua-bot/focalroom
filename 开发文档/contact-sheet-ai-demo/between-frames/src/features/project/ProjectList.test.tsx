import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ProjectList } from './ProjectList';
import type { Project } from '../../domain/models';

const projects = [
  { id: 'a', title: '家的日常', brief: '', tags: ['家庭', '2026'], photos: [], sequence: [], updatedAt: '2026-10-14T01:00:00Z' },
  { id: 'b', title: '街道切片', brief: '', tags: ['街头'], photos: [], sequence: [], updatedAt: '2026-10-14T02:00:00Z' },
  { id: 'c', title: '无标签项目', brief: '', photos: [], sequence: [], updatedAt: '2026-10-14T03:00:00Z' },
] as unknown as Project[];

describe('v1.1 项目标签筛选', () => {
  it('显示标签筛选条与卡片标签', () => {
    render(<ProjectList projects={projects} onOpen={vi.fn()} onCreate={vi.fn()} />);
    expect(screen.getByRole('button', { name: '家庭' })).toBeInTheDocument();
    expect(screen.getByText('街头', { selector: '.tag-filter button' })).toBeInTheDocument();
    expect(screen.getByText('家庭', { selector: '.tag-chips span' })).toBeInTheDocument();
    expect(screen.getByText('街头', { selector: '.tag-chips span' })).toBeInTheDocument();
  });
  it('按标签筛选并支持清除', async () => {
    render(<ProjectList projects={projects} onOpen={vi.fn()} onCreate={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '家庭' }));
    expect(screen.getByText('家的日常')).toBeInTheDocument();
    expect(screen.queryByText('街道切片')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '全部' }));
    expect(screen.getByText('街道切片')).toBeInTheDocument();
  });
});

describe('快速开始入口', () => {
  it('本地项目列表显示快速开始按钮并回调', async () => {
    const onQuickStart = vi.fn();
    render(<ProjectList projects={projects} onOpen={vi.fn()} onCreate={vi.fn()} local onQuickStart={onQuickStart} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '快速开始' }));
    expect(onQuickStart).toHaveBeenCalledTimes(1);
  });
  it('未提供入口或归档视图时不显示', () => {
    const { rerender } = render(<ProjectList projects={projects} onOpen={vi.fn()} onCreate={vi.fn()} local />);
    expect(screen.queryByRole('button', { name: '快速开始' })).not.toBeInTheDocument();
    rerender(<ProjectList projects={projects} onOpen={vi.fn()} onCreate={vi.fn()} local archived onQuickStart={vi.fn()} />);
    expect(screen.queryByRole('button', { name: '快速开始' })).not.toBeInTheDocument();
  });
});
