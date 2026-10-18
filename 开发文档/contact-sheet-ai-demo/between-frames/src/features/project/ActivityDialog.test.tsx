import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { ActivityDialog } from './ActivityDialog';
import type { Project } from '../../domain/models';

const base = new Date('2026-10-18T10:00:00Z');
const at = (minutes: number) => new Date(base.getTime() + minutes * 60_000).toISOString();
const project = {
  id: 'p1', title: '家的日常', brief: '', photos: [
    { id: 'ph1', name: 'DSC_001.jpg', status: 'keep' as const, rating: 3, note: '' },
  ], sequence: ['ph1'], revision: 4,
  events: [
    { id: 'e1', type: 'decision', photoId: 'ph1', description: '修改选片状态', at: at(0), actor: 'user' as const },
    { id: 'e2', type: 'import', description: '导入照片', at: at(5), actor: 'system' as const },
    { id: 'e3', type: 'draft_apply', description: '采纳草案', at: at(10), actor: 'user' as const },
    { id: 'e4', type: 'version_save', description: '保存作品版本', at: at(15), actor: 'user' as const },
    { id: 'e5', type: 'rating', photoId: 'ph1', description: '保存人工评分', at: at(20), actor: 'user' as const },
  ],
  versions: [{ id: 'v1', label: '初稿', basedOnRevision: 3, createdAt: at(15), photoIds: ['ph1'], photos: [] }],
} as unknown as Project;

describe('操作历史入口', () => {
  it('默认倒序列出全部操作并标注操作者与恢复点', () => {
    render(<ActivityDialog project={project} onClose={vi.fn()} />);
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(5);
    expect(items[0]).toHaveTextContent('保存人工评分');
    expect(screen.getByText('恢复点')).toBeInTheDocument();
    expect(screen.getByText('系统', { selector: '.activity-actor' })).toBeInTheDocument();
    expect(screen.getByText('Agent', { selector: '.activity-actor' })).toBeInTheDocument();
    expect(screen.getAllByText(/DSC_001\.jpg/, { selector: '.activity-body small' })).toHaveLength(2);
  });
  it('按人工筛选只剩 user 事件', () => {
    render(<ActivityDialog project={project} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('按操作者筛选'), { target: { value: 'user' } });
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(4);
    expect(screen.queryByText('导入照片', { selector: '.activity-body strong' })).not.toBeInTheDocument();
  });
  it('Agent 草案筛选只显示 draft_ 事件', () => {
    render(<ActivityDialog project={project} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('按操作者筛选'), { target: { value: 'agent' } });
    expect(screen.getAllByRole('listitem')).toHaveLength(1);
    expect(screen.getByText('采纳草案', { selector: '.activity-body strong' })).toBeInTheDocument();
  });
  it('按版本恢复点筛选只显示该版本保存以来的操作', () => {
    render(<ActivityDialog project={project} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('按版本恢复点筛选'), { target: { value: 'v1' } });
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent('保存人工评分');
    expect(items[1]).toHaveTextContent('保存作品版本');
  });
  it('组合筛选无结果时给出空态', () => {
    render(<ActivityDialog project={project} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('按操作者筛选'), { target: { value: 'system' } });
    fireEvent.change(screen.getByLabelText('按类型筛选'), { target: { value: 'rating' } });
    expect(screen.getByText('当前筛选没有匹配的操作。')).toBeInTheDocument();
  });
});
