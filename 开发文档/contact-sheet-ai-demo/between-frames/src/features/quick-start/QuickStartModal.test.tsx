import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { QuickStartModal, parseGoal } from './QuickStartModal';

describe('parseGoal 自然语言解析', () => {
  it('提取张数与主题', () => {
    const parsed = parseGoal('帮我做一个夏日家庭摄影集，选出 20 张，突出自然互动');
    expect(parsed.count).toBe(20);
    expect(parsed.title).toContain('夏日家庭摄影集');
    expect(parsed.goal).toContain('选出 20 张');
  });
  it('中文数字与缺省回退', () => {
    expect(parseGoal('选出30张街头摄影').count).toBe(30);
    expect(parseGoal('整理我的照片').count).toBe(20);
    expect(parseGoal('影'.repeat(2000)).goal).toHaveLength(1500);
  });
});

describe('QuickStartModal 快速开始流程', () => {
  it('描述→确认→创建回调携带解析参数', async () => {
    const onCreate = vi.fn();
    const onClose = vi.fn();
    render(<QuickStartModal onCreate={onCreate} onClose={onClose} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText('项目描述'), '我想做旅行相册，选出 12 张');
    expect(screen.getByText(/目标张数/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '下一步' }));
    await user.click(screen.getByRole('button', { name: '创建项目并导入照片' }));
    expect(onCreate).toHaveBeenCalledTimes(1);
    const [title, goal, count] = onCreate.mock.calls[0];
    expect(count).toBe(12);
    expect(goal).toContain('选出 12 张');
    expect(title.length).toBeGreaterThan(0);
    expect(onClose).not.toHaveBeenCalled();
  });
  it('空描述时不能进入确认步', async () => {
    render(<QuickStartModal onCreate={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByRole('button', { name: '下一步' })).toBeDisabled();
  });
  it('创建期间保留对话框且阻止重复创建，导入由创建成功后触发', async () => {
    const onCreate = vi.fn();
    const onClose = vi.fn();
    const { rerender } = render(<QuickStartModal onCreate={onCreate} onClose={onClose} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText('项目描述'), '旅行相册，选出 12 张');
    await user.click(screen.getByRole('button', { name: '下一步' }));
    await user.click(screen.getByRole('button', { name: '创建项目并导入照片' }));
    rerender(<QuickStartModal busy onCreate={onCreate} onClose={onClose} />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '创建项目并导入照片' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: '关闭对话框' }));
    expect(onClose).not.toHaveBeenCalled();
    expect(onCreate).toHaveBeenCalledTimes(1);
    expect(parseGoal('选出500张旅行照片').count).toBe(100);
  });
});
