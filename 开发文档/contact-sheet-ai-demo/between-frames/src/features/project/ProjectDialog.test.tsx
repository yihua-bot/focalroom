import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ProjectDialog, PROJECT_TEMPLATES } from './ProjectDialog';

describe('v1.1 项目模板', () => {
  it('新建时提供模板快选并预填空字段', async () => {
    const onSave = vi.fn();
    render(<ProjectDialog onClose={vi.fn()} onSave={onSave} busy={false} />);
    const user = userEvent.setup();
    expect(screen.getByText('家庭摄影书')).toBeInTheDocument();
    expect(PROJECT_TEMPLATES.length).toBeGreaterThanOrEqual(4);
    await user.click(screen.getByRole('button', { name: /街头纪实/ }));
    const brief = screen.getByLabelText(/表达什么|意图/) as HTMLTextAreaElement;
    expect(brief.value).toContain('街头');
    const title = screen.getByLabelText(/名称|标题/) as HTMLInputElement;
    expect(title.value).toBe('街道切片');
  });
  it('不覆盖用户已输入的标题与意图', async () => {
    render(<ProjectDialog onClose={vi.fn()} onSave={vi.fn()} busy={false} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/名称|标题/), '我的名字');
    await user.type(screen.getByLabelText(/表达什么|意图/), '我自己的意图');
    await user.click(screen.getByRole('button', { name: /旅行编辑/ }));
    expect((screen.getByLabelText(/名称|标题/) as HTMLInputElement).value).toBe('我的名字');
    expect((screen.getByLabelText(/表达什么|意图/) as HTMLTextAreaElement).value).toBe('我自己的意图');
  });
  it('编辑既有项目时不显示模板', () => {
    render(<ProjectDialog project={{ id: 'p', title: '既有', brief: '' } as never} onClose={vi.fn()} onSave={vi.fn()} busy={false} />);
    expect(screen.queryByText('从模板开始（可选）')).not.toBeInTheDocument();
  });
});
