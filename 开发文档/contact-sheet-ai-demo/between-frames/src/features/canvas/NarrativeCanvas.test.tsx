import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { NarrativeCanvas } from './NarrativeCanvas';
import type { CanvasRepository, CanvasPhoto } from '../../domain/canvas';
import { useUIStore } from '../../stores/uiStore';
afterEach(() => { cleanup(); useUIStore.getState().reset(); });
// jsdom 不实现指针捕获，画布拖拽/框选依赖它，缺失会让 pointerdown 直接抛错。
beforeEach(() => {
  Element.prototype.setPointerCapture ??= () => {};
  Element.prototype.releasePointerCapture ??= () => {};
});

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
  expect(screen.getByRole('button', { name: '另存为独立方案' })).toBeEnabled();
});

it('方案变化读取失败时不能跳过预览直接采纳', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '候选', state: 'draft', owningTaskId: 'task', revision: 3 }]);
  repo.compareDraft = vi.fn().mockRejectedValue(new Error('方案暂时无法读取'));
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  expect(await screen.findByRole('status')).toHaveTextContent('方案暂时无法读取');
  expect(screen.getByRole('button', { name: '另存为独立方案' })).toBeDisabled();
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
    placePhoto: vi.fn(), moveNodes: vi.fn(), addText: vi.fn(), setNodeText: vi.fn(), deleteNode: vi.fn(),
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
  await user.click(await screen.findByRole('button', { name: '另存为独立方案' }));
  expect(screen.getByRole('dialog')).toBeInTheDocument();
  expect(screen.getByRole('dialog')).toHaveTextContent('原画布');
  await user.click(screen.getByRole('button', { name: '取消' }));
  expect(repo.adoptBoard).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: /放入选中照片/ })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '另存为独立方案' }));
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: '另存为独立方案' }));
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
  fireEvent.pointerDown(screen.getByLabelText('叙事画布'), { button: 0, clientX: 20, clientY: 20 });
  await user.click(screen.getByRole('textbox', { name: '便签内容' }));
  await user.keyboard('{Control>}z{/Control}');
  expect(repo.undo).toHaveBeenCalledTimes(1);
});

it('缩放超过 1.25× 后按最粗满足级请求瓦片并叠加图层，回落即清除', async () => {
  const repo = repository();
  vi.mocked(repo.listBoards).mockResolvedValue([{ id: 'board-1', projectId: 'project', title: '第一章', state: 'draft', owningTaskId: null, revision: 1 }]);
  // 1600×1600 源、400×400 节点：zoom≈1.33 → need≈532 → 最粗满足级 level 1（800px）；
  // 但 level 1 瓦片 4×4=16 超上限 9 → 自动粗化到 level 2（400px、2×2=4 张）。
  vi.mocked(repo.getNodes).mockResolvedValue([{ ...node('n1', 'board-1'), width: 400, height: 400, sourceWidth: 1600, sourceHeight: 1600 }]);
  repo.photoTile = vi.fn().mockResolvedValue({ url: 'asset://tile.jpg', width: 256, height: 256 });
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  const surface = await screen.findByLabelText('叙事画布');
  await waitFor(() => expect(screen.getByText('100%')).toBeInTheDocument());
  expect(repo.photoTile).not.toHaveBeenCalled();
  // 三次滚轮放大：1.0 → 1.331 越过 1.25 阈值。
  fireEvent.wheel(surface, { deltaY: -100 });
  fireEvent.wheel(surface, { deltaY: -100 });
  fireEvent.wheel(surface, { deltaY: -100 });
  await waitFor(() => expect(screen.getByText('133%')).toBeInTheDocument());
  await waitFor(() => expect(repo.photoTile).toHaveBeenCalledTimes(4));
  expect((repo.photoTile as ReturnType<typeof vi.fn>).mock.calls.every(call => call[2] === 2)).toBe(true);
  const layer = await waitFor(() => {
    const el = document.querySelector('.canvas-tile-layer');
    expect(el).toBeTruthy();
    return el as HTMLElement;
  });
  expect(layer.querySelectorAll('img')).toHaveLength(4);
  // 三次滚轮回落后瓦片图层清除（缩略图恢复）。
  fireEvent.wheel(surface, { deltaY: 100 });
  fireEvent.wheel(surface, { deltaY: 100 });
  fireEvent.wheel(surface, { deltaY: 100 });
  await waitFor(() => expect(document.querySelector('.canvas-tile-layer')).toBeNull());
});

const sticky = (id: string, text: string): CanvasPhoto => ({
  id, boardId: 'board-1', nodeType: 'sticky', x: 10, y: 20, width: 200, height: 120,
  rotation: 0, zOrder: 1, assetId: null, renditionRef: null, contentJson: JSON.stringify({ text }),
});

it('便签先落点再就地输入，空白落点与取消都不留节点', async () => {
  const repo = repository();
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(screen.getByRole('button', { name: '便签' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: '便签' }));
  // 旧版是先弹输入框再落节点，用户不知道便签会在哪里出现；现在落点前不应有任何输入框。
  expect(screen.queryByRole('textbox', { name: '便签内容' })).not.toBeInTheDocument();
  fireEvent.pointerDown(container.querySelector('.canvas-surface')!, { button: 0, clientX: 30, clientY: 40 });
  await user.type(screen.getByRole('textbox', { name: '便签内容' }), '开场用逆光');
  await user.click(screen.getByRole('button', { name: '保存' }));
  await waitFor(() => expect(repo.addText).toHaveBeenCalledWith('project', 'board-1', 'sticky', 30, 40, 200, 120, '开场用逆光', '#ffe082'));
  expect(container.querySelector('.canvas-tool-status')).toBeNull();
  vi.mocked(repo.addText).mockClear();
  await user.click(screen.getByRole('button', { name: '便签' }));
  fireEvent.pointerDown(container.querySelector('.canvas-surface')!, { button: 0, clientX: 60, clientY: 60 });
  await user.click(screen.getByRole('button', { name: '取消' }));
  expect(repo.addText).not.toHaveBeenCalled();
  expect(screen.queryByRole('textbox', { name: '便签内容' })).not.toBeInTheDocument();
});

it('双击已有便签就地改写，清空正文则删除节点', async () => {
  const repo = repository();
  vi.mocked(repo.getNodes).mockResolvedValue([sticky('s1', '初稿')]);
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(container.querySelector('[data-node-id="s1"]')).toBeInTheDocument());
  await user.dblClick(container.querySelector('[data-node-id="s1"]')!);
  const input = screen.getByRole('textbox', { name: '便签内容' });
  expect(input).toHaveValue('初稿');
  await user.clear(input);
  await user.type(input, '逆光开场');
  await user.click(screen.getByRole('button', { name: '保存' }));
  await waitFor(() => expect(repo.setNodeText).toHaveBeenCalledWith('project', 'board-1', 's1', '逆光开场'));
  expect(repo.deleteNode).not.toHaveBeenCalled();
  await user.dblClick(container.querySelector('[data-node-id="s1"]')!);
  await user.clear(screen.getByRole('textbox', { name: '便签内容' }));
  await user.click(screen.getByRole('button', { name: '保存' }));
  await waitFor(() => expect(repo.deleteNode).toHaveBeenCalledWith('project', 'board-1', 's1'));
});

it('连接模式给出进行中提示，并有取消、Esc、点空白三条退出路径', async () => {
  const repo = repository();
  vi.mocked(repo.getNodes).mockResolvedValue([node('n1', 'board-1'), { ...node('n2', 'board-1'), x: 300 }]);
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(container.querySelector('[data-node-id="n2"]')).toBeInTheDocument());
  const connect = screen.getByRole('button', { name: '连接' });
  const surface = container.querySelector<HTMLElement>('.canvas-surface')!;
  await user.click(connect);
  expect(connect).toHaveAttribute('aria-pressed', 'true');
  expect(container.querySelector('.canvas-tool-status')?.textContent).toContain('先点击起点节点');
  await user.click(container.querySelector('[data-node-id="n1"]')!);
  expect(container.querySelector('.canvas-tool-status')?.textContent).toContain('再点击终点节点');
  expect(container.querySelector('[data-node-id="n1"]')!.className).toContain('link-source');
  await user.click(container.querySelector('[data-node-id="n2"]')!);
  await waitFor(() => expect(repo.addArrow).toHaveBeenCalledWith('project', 'board-1', 'n1', 'n2'));
  expect(connect).toHaveAttribute('aria-pressed', 'false');
  await user.click(connect);
  await user.click(container.querySelector('[data-node-id="n1"]')!);
  await user.click(screen.getByRole('button', { name: '取消' }));
  expect(connect).toHaveAttribute('aria-pressed', 'false');
  await user.click(connect);
  await user.click(container.querySelector('[data-node-id="n1"]')!);
  surface.focus();
  await user.keyboard('{Escape}');
  expect(connect).toHaveAttribute('aria-pressed', 'false');
  await user.click(connect);
  expect(connect).toHaveAttribute('aria-pressed', 'true');
  fireEvent.pointerDown(surface, { button: 0, clientX: 500, clientY: 500 });
  expect(connect).toHaveAttribute('aria-pressed', 'false');
  expect(container.querySelector('.canvas-tool-status')).toBeNull();
});

it('删除只作用于选中节点，锁定节点跳过并说明原因', async () => {
  const repo = repository();
  vi.mocked(repo.getNodes).mockResolvedValue([node('n1', 'board-1'), { ...node('n2', 'board-1'), x: 300, locked: true }]);
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(container.querySelector('[data-node-id="n2"]')).toBeInTheDocument());
  await user.click(container.querySelector('[data-node-id="n1"]')!);
  fireEvent.pointerDown(container.querySelector('[data-node-id="n2"]')!, { button: 0, shiftKey: true });
  const remove = screen.getByRole('button', { name: '删除' });
  expect(remove).toBeEnabled();
  await user.click(remove);
  await waitFor(() => expect(repo.deleteNode).toHaveBeenCalledWith('project', 'board-1', 'n1'));
  expect(repo.deleteNode).toHaveBeenCalledTimes(1);
  expect(await screen.findByRole('status')).toHaveTextContent('1 个锁定节点已跳过');
});

it('右键落评论、图钉可读并可标记已解决', async () => {
  const repo = repository();
  const pins = [{ id: 'a1', boardId: 'board-1', anchorType: 'board', x: 20, y: 20, author: '摄影师', body: '这里换成日落那张', resolved: false, createdAt: 'now' }];
  vi.mocked(repo.getAnnotations).mockImplementation(async () => pins.map(pin => ({ ...pin })));
  vi.mocked(repo.resolveAnnotation).mockImplementation(async (_project, annotationId, resolved) => {
    const pin = pins.find(item => item.id === annotationId);
    if (pin) pin.resolved = resolved;
  });
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(container.querySelector('.canvas-annotation-pin')).toBeInTheDocument());
  fireEvent.contextMenu(container.querySelector('.canvas-surface')!, { clientX: 20, clientY: 20 });
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByRole('heading', { name: '添加画布评论' })).toBeInTheDocument();
  await user.type(within(dialog).getByRole('textbox', { name: '评论内容' }), '第二格太挤');
  await user.click(within(dialog).getByRole('button', { name: '添加评论' }));
  await waitFor(() => expect(repo.addAnnotation).toHaveBeenCalledWith('project', 'board-1', 20, 20, '第二格太挤'));
  await user.click(container.querySelector('.canvas-annotation-pin')!);
  const comment = await screen.findByRole('dialog');
  expect(within(comment).getByText('这里换成日落那张')).toBeInTheDocument();
  await user.click(within(comment).getByRole('button', { name: '标记已解决' }));
  await waitFor(() => expect(repo.resolveAnnotation).toHaveBeenCalledWith('project', 'a1', true));
  await waitFor(() => expect(container.querySelector('.canvas-annotation-pin')).toBeNull());
});

it('分组改走应用内输入框，不再依赖浏览器 prompt', async () => {
  const repo = repository();
  vi.mocked(repo.getNodes).mockResolvedValue([node('n1', 'board-1'), { ...node('n2', 'board-1'), x: 300 }]);
  const prompt = vi.spyOn(window, 'prompt').mockImplementation(() => null);
  const user = userEvent.setup();
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} />);
  await waitFor(() => expect(container.querySelector('[data-node-id="n2"]')).toBeInTheDocument());
  await user.click(container.querySelector('[data-node-id="n1"]')!);
  fireEvent.pointerDown(container.querySelector('[data-node-id="n2"]')!, { button: 0, shiftKey: true });
  await user.click(screen.getByRole('button', { name: '分组' }));
  const dialog = await screen.findByRole('dialog');
  expect(prompt).not.toHaveBeenCalled();
  await user.type(within(dialog).getByRole('textbox', { name: '分组名称' }), '开场段落');
  await user.click(within(dialog).getByRole('button', { name: '分组' }));
  await waitFor(() => expect(repo.groupNodes).toHaveBeenCalledWith('project', 'board-1', ['n1', 'n2'], '开场段落'));
});
