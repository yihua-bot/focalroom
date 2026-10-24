import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import type { CanvasRepository, RevertSimulation } from '../../domain/canvas';
import { EditorialHistoryDialog } from './EditorialHistoryDialog';

const entry = { cursor: 1, commandId: 'merge', draftId: 'draft', changeCount: 1, createdAt: '2026-10-24T00:00:00+08:00', undone: false, compensated: false, isCompensation: false, nodeTypes: ['photo'] };
const simulation: RevertSimulation = { mergeCommandId: 'merge', targetRevision: 8, simulationId: 'preview', currentNodes: [], mergedNodes: [], conflicts: [], canApply: true };
function setup(overrides: Partial<CanvasRepository> = {}, readOnly = false) {
  const repository = { editorialHistory: vi.fn().mockResolvedValue([entry]), simulateRevert: vi.fn().mockResolvedValue(simulation), revertMerge: vi.fn().mockResolvedValue(undefined), ...overrides } as unknown as CanvasRepository;
  const onChanged = vi.fn().mockResolvedValue(undefined);
  render(<EditorialHistoryDialog repository={repository} projectId="p" boardId="board" readOnly={readOnly} onClose={vi.fn()} onChanged={onChanged} />);
  return { repository, onChanged };
}
it('先预览再确认撤回，预览本身不写入作品', async () => {
  const { repository, onChanged } = setup(); const user = userEvent.setup();
  await user.click(await screen.findByRole('button', { name: '预览撤回' }));
  await screen.findByRole('button', { name: '确认撤回' });
  expect(repository.revertMerge).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: '确认撤回' }));
  expect(repository.revertMerge).toHaveBeenCalledWith('p', 'board', simulation);
  await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
});
it('只读仍能比较历史，但不能提交；冲突同样阻止撤回', async () => {
  const { repository } = setup({}, true); const user = userEvent.setup();
  await user.click(await screen.findByRole('button', { name: '预览撤回' }));
  expect(await screen.findByRole('button', { name: '确认撤回' })).toBeDisabled();
  expect(repository.revertMerge).not.toHaveBeenCalled();
});
it('重复确认只提交一次，底层异常不直接显示给用户', async () => {
  let reject!: (reason: unknown) => void;
  const revertMerge = vi.fn(() => new Promise<void>((_, fail) => { reject = fail; }));
  setup({ revertMerge }); const user = userEvent.setup();
  await user.click(await screen.findByRole('button', { name: '预览撤回' }));
  const confirm = await screen.findByRole('button', { name: '确认撤回' });
  await user.dblClick(confirm);
  expect(revertMerge).toHaveBeenCalledOnce();
  reject(new Error('SQLITE database connection failed'));
  expect(await screen.findByRole('alert')).toHaveTextContent('读取采纳历史失败，请重新打开。');
  expect(screen.queryByText(/SQLITE/)).not.toBeInTheDocument();
});
