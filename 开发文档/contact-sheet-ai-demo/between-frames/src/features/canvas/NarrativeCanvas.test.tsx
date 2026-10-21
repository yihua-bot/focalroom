import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { NarrativeCanvas } from './NarrativeCanvas';
import type { CanvasRepository, CanvasPhoto } from '../../domain/canvas';
afterEach(cleanup);

function repository(): CanvasRepository {
  return {
    listBoards: vi.fn().mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '第一章', state: 'draft' }]),
    createBoard: vi.fn(), getNodes: vi.fn().mockResolvedValue([]), getAnnotations: vi.fn().mockResolvedValue([]),
    placePhoto: vi.fn(), moveNodes: vi.fn(), addText: vi.fn(), deleteNode: vi.fn(),
    addAnnotation: vi.fn(), resolveAnnotation: vi.fn(), addArrow: vi.fn(), undo: vi.fn(), addInk: vi.fn(), groupNodes: vi.fn(),
  };
}
const node = (id: string, boardId: string): CanvasPhoto => ({
  id, boardId, nodeType: 'photo', x: 0, y: 0, width: 100, height: 100,
  rotation: 0, zOrder: 1, assetId: id, renditionRef: null,
});

it('新建第二张画布后只加载新画布节点，旧节点不能覆盖新画布', async () => {
  const repo = repository();
  vi.mocked(repo.getNodes).mockImplementation(async id => [node(id + '-photo', id)]);
  vi.mocked(repo.createBoard).mockResolvedValue({ id: 'board-2', projectId: 'project', title: '第二章', state: 'draft' });
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(container.querySelector('[data-node-id="board-1-photo"]')).toBeInTheDocument());
  vi.mocked(repo.listBoards).mockResolvedValue([
    { id: 'board-1', projectId: 'project', title: '第一章', state: 'draft' },
    { id: 'board-2', projectId: 'project', title: '第二章', state: 'draft' },
  ]);
  await user.click(screen.getByRole('button', { name: '新建画布' }));
  await waitFor(() => expect(container.querySelector('[data-node-id="board-2-photo"]')).toBeInTheDocument());
  expect(container.querySelector('[data-node-id="board-1-photo"]')).not.toBeInTheDocument();
  expect(vi.mocked(repo.getNodes).mock.calls.filter(([id]) => id === 'board-1')).toHaveLength(1);
  expect(screen.getByRole('combobox', { name: '选择画布' })).toHaveValue('board-2');
});

it('共享项目只读，加载失败可见且不伪装为空项目', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockRejectedValue(new Error('项目访问权已撤销'));
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={['photo']} readOnly />);
  expect(await screen.findByRole('status')).toHaveTextContent('项目访问权已撤销');
  expect(screen.getByRole('button', { name: '新建画布' })).toBeDisabled();
  expect(screen.getByRole('button', { name: /放入选中照片/ })).toBeDisabled();
  expect(screen.getByRole('button', { name: '便签' })).toBeDisabled();
  expect(repo.createBoard).not.toHaveBeenCalled();
});
