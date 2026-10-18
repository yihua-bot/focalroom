import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { CommandPalette, type CommandEntry } from './CommandPalette';

const commands: CommandEntry[] = [
  { id: 'keep', group: '决策', label: '保留选区', hint: 'K', run: vi.fn() },
  { id: 'grid', group: '视图', label: '切换到网格视图', run: vi.fn() },
  { id: 'sequence', group: '视图', label: '切换到作品序列', run: vi.fn() },
  { id: 'help', group: '帮助', label: '快捷键与帮助', run: vi.fn() },
];

describe('⌘K 命令面板', () => {
  it('展示全部命令并支持关键词过滤', () => {
    const onClose = vi.fn();
    render(<CommandPalette commands={commands} onClose={onClose} />);
    expect(screen.getByRole('option', { name: /保留选区/ })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('搜索命令'), { target: { value: '视图' } });
    expect(screen.getAllByRole('option')).toHaveLength(2);
    fireEvent.change(screen.getByLabelText('搜索命令'), { target: { value: '不存在' } });
    expect(screen.getByText('没有匹配的命令')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
  it('键盘导航选中并 Enter 执行，随后关闭面板', () => {
    const onClose = vi.fn();
    render(<CommandPalette commands={commands} onClose={onClose} />);
    const input = screen.getByLabelText('搜索命令');
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(screen.getAllByRole('option')[2]).toHaveAttribute('aria-selected', 'true');
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(commands[2].run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
  it('点击选项直接执行对应命令', () => {
    const onClose = vi.fn();
    render(<CommandPalette commands={commands} onClose={onClose} />);
    fireEvent.click(screen.getByRole('option', { name: /快捷键与帮助/ }));
    expect(commands[3].run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
