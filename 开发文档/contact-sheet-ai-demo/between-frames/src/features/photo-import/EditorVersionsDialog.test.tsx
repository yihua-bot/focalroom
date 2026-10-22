import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { EditorVersionsDialog } from './EditorVersionsDialog';
import type { EditorVersion } from '../../domain/editorVersions';

const photo: EditorVersion = { id: 'r1', photoName: 'frame.jpg', provider: 'lightroom_classic', previewSrc: '/preview.jpg', createdAt: '2026-10-22' };
describe('调色版本', () => {
  it('import refreshes previews and preserves the formal-work boundary in its message', async () => {
    const repository = { list: vi.fn().mockResolvedValueOnce([]).mockResolvedValue([photo]), import: vi.fn().mockResolvedValue(1) };
    render(<EditorVersionsDialog repository={repository} projectId="p1" onClose={vi.fn()} />);
    const button = screen.getByRole('button', { name: '导入调色结果' });
    await waitFor(() => expect(button).toBeEnabled());
    await userEvent.click(button);
    expect(repository.import).toHaveBeenCalledWith('p1');
    expect(await screen.findByText('调色版本已保存。原照片与选片保持不变。')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /frame.jpg/ }));
    expect(screen.getByRole('button', { name: '收起大图' })).toBeInTheDocument();
  });
  it('cancel and repeated import do not invent a successful new import', async () => {
    const repository = { list: vi.fn().mockResolvedValue([]), import: vi.fn().mockResolvedValueOnce(null).mockResolvedValueOnce(0) };
    render(<EditorVersionsDialog repository={repository} projectId="p1" onClose={vi.fn()} />);
    const button = screen.getByRole('button', { name: '导入调色结果' });
    await waitFor(() => expect(button).toBeEnabled()); await userEvent.click(button);
    expect(repository.list).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    await userEvent.click(button);
    expect(await screen.findByText('这些调色版本已经导入，无需重复添加。')).toBeInTheDocument();
  });
  it('shared projects can view but cannot import', async () => {
    const repository = { list: vi.fn().mockResolvedValue([photo]), import: vi.fn() };
    render(<EditorVersionsDialog repository={repository} projectId="p1" readOnly onClose={vi.fn()} />);
    await screen.findByText('frame.jpg');
    expect(screen.getByRole('button', { name: '导入调色结果' })).toBeDisabled();
    expect(repository.import).not.toHaveBeenCalled();
  });
  it('shows an import error and allows another choice', async () => {
    const repository = { list: vi.fn().mockResolvedValue([]), import: vi.fn().mockRejectedValue(new Error('当前项目中没有对应的原照片。')) };
    render(<EditorVersionsDialog repository={repository} projectId="p1" onClose={vi.fn()} />);
    const button = screen.getByRole('button', { name: '导入调色结果' });
    await waitFor(() => expect(button).toBeEnabled()); await userEvent.click(button);
    expect(await screen.findByRole('alert')).toHaveTextContent('当前项目中没有对应的原照片。');
    expect(button).toBeEnabled();
  });
});
