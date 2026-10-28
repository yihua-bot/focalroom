import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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
  const { container } = render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={['photo']} photos={[{ id: 'photo', index: 1, name: '照片', src: '/photo.jpg', status: 'undecided', note: '', rating: 0 }]} />);
  await screen.findByRole('button', { name: '确认上次操作' });
  expect(screen.getByRole('button', { name: /添加照片/ })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '确认上次操作' }));
  expect(repo.retryPending).toHaveBeenCalledWith('project', 'board-1');
  await waitFor(() => expect(container.querySelector('[data-node-id="restored"]')).toBeInTheDocument());
  expect(screen.queryByRole('button', { name: '确认上次操作' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: /添加照片/ })).toBeEnabled();
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

it('选中内容后按钮与删除键都先确认，取消不写入，确认批量移除后可以撤销',async()=>{
  const repo=repository(),user=userEvent.setup();
  vi.mocked(repo.getNodes).mockResolvedValue([node('a','board-1'),node('b','board-1')]);
  repo.deleteNodes=vi.fn().mockImplementation(async()=>{vi.mocked(repo.getNodes).mockResolvedValue([]);});
  const {container}=render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]}/>);
  await waitFor(()=>expect(container.querySelector('[data-node-id="a"]')).toBeInTheDocument());
  const select=async(id:string,shift=false)=>{const element=container.querySelector(`[data-node-id="${id}"]`)!;Object.assign(element,{setPointerCapture:vi.fn()});if(shift)await user.keyboard('{Shift>}');await user.click(element);if(shift)await user.keyboard('{/Shift}');};
  await select('a');await select('b',true);
  await user.click(screen.getByRole('button',{name:'从画布移除 (2)'}));
  expect(repo.deleteNodes).not.toHaveBeenCalled();
  expect(screen.getByRole('dialog')).toHaveTextContent('原图、作品序列和其他画布不受影响');
  await user.click(screen.getByRole('button',{name:'取消'}));
  await user.keyboard('{Backspace}');
  await user.click(within(screen.getByRole('dialog')).getByRole('button',{name:'移除选中内容'}));
  await waitFor(()=>expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(repo.deleteNodes).toHaveBeenCalledExactlyOnceWith('project','board-1',['a','b']);
  expect(screen.getByRole('status')).toHaveTextContent('已从画布移除');
  await user.click(screen.getByRole('button',{name:'撤销'}));
  expect(repo.undo).toHaveBeenCalledWith('project','board-1');
});

it('锁定内容无法误删，文字输入里的删除键只编辑文字',async()=>{
  const repo=repository(),user=userEvent.setup();
  vi.mocked(repo.getNodes).mockResolvedValue([{...node('locked','board-1'),locked:true}]);
  const {container}=render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]}/>);
  await waitFor(()=>expect(container.querySelector('[data-node-id="locked"]')).toBeInTheDocument());
  await user.click(container.querySelector('[data-node-id="locked"]')!);
  expect(screen.getByRole('button',{name:'从画布移除 (1)'})).toBeDisabled();
  await user.keyboard('{Delete}');expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await user.click(screen.getByRole('button',{name:'便签'}));
  await user.type(screen.getByRole('textbox',{name:'便签内容'}),'保留文字{Backspace}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(repo.deleteNode).not.toHaveBeenCalled();
});

it('滚动平移、组合键缩放和工具退出不修改内容，分组用应用内表单明确提交',async()=>{
  const repo=repository(),user=userEvent.setup();vi.mocked(repo.getNodes).mockResolvedValue([node('a','board-1'),node('b','board-1')]);
  const {container}=render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]}/>);
  await waitFor(()=>expect(container.querySelector('[data-node-id="a"]')).toBeInTheDocument());
  const surface=screen.getByLabelText('叙事画布');fireEvent.wheel(surface,{deltaY:100});expect(container.querySelector('.canvas-board')).toHaveStyle({transform:'translate(0px, -100px) scale(1)'});
  fireEvent.wheel(surface,{deltaY:-100,ctrlKey:true});expect(screen.getByRole('button',{name:'恢复 100%'})).toHaveTextContent('110%');
  await user.click(screen.getByRole('button',{name:'恢复 100%'}));expect(screen.getByRole('button',{name:'恢复 100%'})).toHaveTextContent('100%');
  await user.click(screen.getByRole('button',{name:'画笔'}));expect(screen.getByRole('button',{name:'画笔'})).toHaveAttribute('aria-pressed','true');await user.keyboard('{Escape}');expect(screen.getByRole('button',{name:'选择'})).toHaveAttribute('aria-pressed','true');
  surface.focus();await user.keyboard('{Control>}a{/Control}');
  await user.click(screen.getByRole('button',{name:'分组'}));expect(repo.groupNodes).not.toHaveBeenCalled();
  await user.type(screen.getByRole('textbox',{name:'分组名称'}),'开场');await user.click(screen.getByRole('button',{name:'创建分组'}));
  await waitFor(()=>expect(repo.groupNodes).toHaveBeenCalledExactlyOnceWith('project','board-1',['a','b'],'开场'));
  expect(repo.moveNodes).not.toHaveBeenCalled();
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
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={['photo']} photos={[{ id: 'photo', index: 1, name: '照片', src: '/photo.jpg', status: 'undecided', note: '', rating: 0 }]} readOnly />);
  expect(await screen.findByRole('status')).toHaveTextContent('项目访问权已撤销');
  expect(screen.getByRole('button', { name: '新建画布' })).toBeDisabled();
  expect(screen.getByRole('button', { name: /添加照片/ })).toBeDisabled();
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
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={['photo']} photos={[{ id: 'photo', index: 1, name: '照片', src: '/photo.jpg', status: 'undecided', note: '', rating: 0 }]} />);
  await user.click(await screen.findByRole('button', { name: '另存为独立方案' }));
  expect(screen.getByRole('dialog')).toBeInTheDocument();
  expect(screen.getByRole('dialog')).toHaveTextContent('原画布');
  await user.click(screen.getByRole('button', { name: '取消' }));
  expect(repo.adoptBoard).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: /添加照片/ })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '另存为独立方案' }));
  await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: '另存为独立方案' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(repo.adoptBoard).toHaveBeenCalledWith('project', 'board-1');
  expect(screen.getByRole('button', { name: /添加照片/ })).toBeEnabled();
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

it('混合锁定选区明确先全部锁定，再可全部解锁；取消选择返回画布焦点',async()=>{
  const repo=repository(),user=userEvent.setup();
  let current=[{...node('a','board-1'),locked:true},{...node('b','board-1'),locked:false}];
  vi.mocked(repo.getNodes).mockImplementation(async()=>structuredClone(current));
  repo.setNodesLocked=vi.fn(async(_project,_board,ids,locked)=>{current=current.map(item=>ids.includes(item.id)?{...item,locked}:item);});
  const {container}=render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]}/>);
  await waitFor(()=>expect(container.querySelector('[data-node-id="a"]')).toBeInTheDocument());
  screen.getByLabelText('叙事画布').focus();await user.keyboard('{Control>}a{/Control}');
  await user.click(screen.getByRole('button',{name:'锁定选中内容'}));
  await waitFor(()=>expect(repo.setNodesLocked).toHaveBeenCalledExactlyOnceWith('project','board-1',['a','b'],true));
  expect(await screen.findByRole('button',{name:'解锁选中内容'})).toBeEnabled();
  screen.getByLabelText('叙事画布').focus();await user.keyboard('l');
  await waitFor(()=>expect(repo.setNodesLocked).toHaveBeenLastCalledWith('project','board-1',['a','b'],false));
  await user.click(screen.getByRole('button',{name:'清除画布选区'}));
  expect(screen.getByLabelText('叙事画布')).toHaveFocus();
  expect(screen.queryByRole('group',{name:'选中内容'})).not.toBeInTheDocument();
  expect(repo.moveNodes).not.toHaveBeenCalled();
});

it('空画布提供实际选图和便签入口，读取未完成前不显示空内容',async()=>{
  const repo=repository(),user=userEvent.setup();let resolveNodes!:(nodes:CanvasPhoto[])=>void;
  vi.mocked(repo.getNodes).mockImplementation(()=>new Promise(resolve=>{resolveNodes=resolve;}));
  render(<NarrativeCanvas repository={repo} projectId="project" selectedPhotoIds={[]} photos={[{id:'photo',index:1,name:'我的照片',src:'/p.jpg',status:'undecided',note:'',rating:0}]}/>);
  await waitFor(()=>expect(repo.getNodes).toHaveBeenCalled());
  expect(screen.queryByRole('region',{name:'开始编排'})).not.toBeInTheDocument();
  resolveNodes([]);
  const start=await screen.findByRole('region',{name:'开始编排'});
  await user.click(within(start).getByRole('button',{name:'选择照片开始'}));
  expect(screen.getByRole('dialog',{name:'添加照片'})).toBeVisible();
  await user.click(within(screen.getByRole('dialog')).getByRole('button',{name:'取消'}));
  await user.click(within(start).getByRole('button',{name:'先写一张便签'}));
  expect(screen.getByRole('textbox',{name:'便签内容'})).toBeVisible();
  expect(repo.placePhoto).not.toHaveBeenCalled();expect(repo.moveNodes).not.toHaveBeenCalled();
});
