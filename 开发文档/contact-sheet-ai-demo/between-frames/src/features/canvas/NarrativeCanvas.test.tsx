import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { NarrativeCanvas } from './NarrativeCanvas';
import type { CanvasRepository, CanvasPhoto } from '../../domain/canvas';
import { useUIStore } from '../../stores/uiStore';
afterEach(() => { cleanup(); useUIStore.getState().reset(); });

it('方案显示真实变化与原布局叠层，原版更新有提示', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '候选', state: 'draft', owningTaskId: 'task', revision: 3 }]);
  repo.compareDraft = vi.fn().mockResolvedValue({ draftRevision: 3, sourceBoardId: 'source', sourceChanged: true, baselineNodes: [node('old', 'source')], changes: [{ id: 'change', nodeId: 'new', kind: 'change', nodeType: 'photo' }] });
  const user = userEvent.setup();
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await screen.findByText('方案变化');
  expect(screen.getByText('调整 1')).toBeInTheDocument();
  expect(screen.getByText(/原画布已有更新/)).toBeInTheDocument();
  expect(screen.getAllByTestId('canvas-baseline-node')).toHaveLength(1);
  await user.click(screen.getByRole('checkbox', { name: '叠加原布局' }));
  expect(screen.queryByTestId('canvas-baseline-node')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: '采纳画布' })).toBeEnabled();
});

it('方案变化读取失败时不能跳过预览直接采纳', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '候选', state: 'draft', owningTaskId: 'task', revision: 3 }]);
  repo.compareDraft = vi.fn().mockRejectedValue(new Error('方案暂时无法读取'));
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  expect(await screen.findByRole('status')).toHaveTextContent('方案暂时无法读取');
  expect(screen.getByRole('button', { name: '采纳画布' })).toBeDisabled();
  expect(repo.adoptBoard).not.toHaveBeenCalled();
});

it('画布 AI 入口使用当前快照版本并保留画布工作面', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '当前', state: 'official', owningTaskId: null, revision: 2 }]);
  repo.getSnapshot = vi.fn().mockResolvedValue({ board: { id: 'board-1', projectId: 'project', title: '当前', state: 'official', owningTaskId: null, revision: 4 }, nodes: [], annotations: [] });
  const user = userEvent.setup();
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(screen.getByRole('button', { name: 'AI 编排' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'AI 编排' }));
  expect(useUIStore.getState()).toMatchObject({ view: 'canvas', aiOpen: true, canvasTaskSource: { boardId: 'board-1', revision: 4, selectedNodeIds: [] } });
});

it('中断操作经确认后刷新画布，确认前阻止编辑', async () => {
  const repo = repository();
  repo.pendingCommands = vi.fn().mockReturnValue([{ boardId: 'board-1' }]);
  repo.retryPending = vi.fn().mockImplementation(async () => {
    vi.mocked(repo.pendingCommands!).mockReturnValue([]);
    vi.mocked(repo.getNodes).mockResolvedValue([node('restored', 'board-1')]);
  });
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={['photo']} />);
  await screen.findByRole('button', { name: '确认上次操作' });
  expect(screen.getByRole('button', { name: /放入选中照片/ })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '确认上次操作' }));
  expect(repo.retryPending).toHaveBeenCalledWith('project', 'board-1');
  await waitFor(() => expect(container.querySelector('[data-node-id="restored"]')).toBeInTheDocument());
  expect(screen.queryByRole('button', { name: '确认上次操作' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: /放入选中照片/ })).toBeEnabled();
});

function repository(): CanvasRepository {
  return {
    listBoards: vi.fn().mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '第一章', state: 'draft' }]),
    createBoard: vi.fn(), getNodes: vi.fn().mockResolvedValue([]), getAnnotations: vi.fn().mockResolvedValue([]),
    placePhoto: vi.fn(), moveNodes: vi.fn(), addText: vi.fn(), deleteNode: vi.fn(),
    addAnnotation: vi.fn(), resolveAnnotation: vi.fn(), addArrow: vi.fn(), undo: vi.fn(), redo: vi.fn(), adoptBoard: vi.fn(), addInk: vi.fn(), groupNodes: vi.fn(), resizeNode: vi.fn(), setNodeLocked: vi.fn(),
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

it('摄影师可撤销、重做并看到空历史提示，失败不会显示成功', async () => {
  const repo = repository();
  vi.mocked(repo.undo).mockResolvedValue(true);
  vi.mocked(repo.redo).mockResolvedValue(false);
  const user = userEvent.setup();
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(screen.getByRole('button', { name: '撤销' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: '撤销' }));
  expect(repo.undo).toHaveBeenCalledWith('project', 'board-1');
  await user.click(screen.getByRole('button', { name: '重做' }));
  expect(repo.redo).toHaveBeenCalledWith('project', 'board-1');
  expect(await screen.findByRole('status')).toHaveTextContent('没有可重做的操作');
  vi.mocked(repo.undo).mockRejectedValueOnce(new Error('节点已锁定'));
  await user.click(screen.getByRole('button', { name: '撤销' }));
  expect(await screen.findByRole('status')).toHaveTextContent('节点已锁定');
});

it('编辑者先查看任务草案，确认采纳后才可编辑，取消不提交', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: 'AI 草案', state: 'draft', owningTaskId: 'task-1', revision: 3 }]);
  vi.mocked(repo.adoptBoard).mockImplementation(async () => {
    vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: 'AI 草案', state: 'draft', owningTaskId: null, revision: 4 }]);
  });
  const user = userEvent.setup();
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={['photo']} />);
  await user.click(await screen.findByRole('button', { name: '采纳画布' }));
  expect(screen.getByRole('dialog')).toBeInTheDocument();
  expect(screen.getByRole('dialog')).toHaveTextContent('采纳操作无法撤销');
  await user.click(screen.getByRole('button', { name: '取消' }));
  expect(repo.adoptBoard).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: /放入选中照片/ })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '采纳画布' }));
  await user.click(screen.getByRole('button', { name: '确认采纳' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(repo.adoptBoard).toHaveBeenCalledWith('project', 'board-1');
  expect(screen.getByRole('button', { name: /放入选中照片/ })).toBeEnabled();
});

it('快捷键仅作用于当前画布，文本输入保留自己的撤销行为', async () => {
  const repo = repository();
  vi.mocked(repo.undo).mockResolvedValue(true);
  vi.mocked(repo.redo).mockResolvedValue(true);
  const user = userEvent.setup();
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(screen.getByRole('button', { name: '便签' })).toBeEnabled());
  screen.getByLabelText('叙事画布').focus();
  await user.keyboard('{Control>}z{/Control}');
  await waitFor(() => expect(repo.undo).toHaveBeenCalledTimes(1));
  await user.keyboard('{Control>}{Shift>}z{/Shift}{/Control}');
  await waitFor(() => expect(repo.redo).toHaveBeenCalledTimes(1));
  await user.click(screen.getByRole('button', { name: '便签' }));
  await user.click(screen.getByRole('textbox', { name: '便签内容' }));
  await user.keyboard('{Control>}z{/Control}');
  expect(repo.undo).toHaveBeenCalledTimes(1);
});
