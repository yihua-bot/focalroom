import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { CanvasComparison, CanvasRepository, EditorialSelection, MergeSimulation } from '../../domain/canvas';
import { EditorialReviewWorkspace } from './EditorialReviewWorkspace';

beforeEach(() => {
  const values = new Map<string, string>();
  vi.stubGlobal('localStorage', { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key, value), removeItem: (key: string) => values.delete(key) });
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
const board = { id: 'draft', projectId: 'project', title: '方案 A', state: 'draft', owningTaskId: 'task', revision: 3 };
const node = (id: string, x = 0) => ({ id, boardId: 'source', nodeType: 'sticky', x, y: 0, width: 200, height: 120, rotation: 0, zOrder: 0, assetId: null, renditionRef: null, contentJson: '{"text":"节奏"}' });
const comparison: CanvasComparison = { draftRevision: 3, sourceBoardId: 'source', sourceRevision: 12, currentSourceRevision: 12, sourceChanged: false,
  baselineNodes: [node('a'), node('b', 240)], draftNodes: [node('da', 60), node('db', 300)], changes: [
    { id: 'change-a', nodeId: 'da', sourceNodeId: 'a', kind: 'change', nodeType: 'sticky', kinds: ['move'], before: node('a'), after: node('da', 60), requiredDependencies: [] },
    { id: 'change-b', nodeId: 'db', sourceNodeId: 'b', kind: 'change', nodeType: 'sticky', kinds: ['move'], before: node('b', 240), after: node('db', 300), requiredDependencies: [] },
  ] };
function setup(overrides: Partial<CanvasRepository> = {}, diff = comparison, readOnly = false) {
  const simulateMerge = vi.fn(async (_p: string, s: EditorialSelection): Promise<MergeSimulation> => ({ simulationId: JSON.stringify(s.selectedChangeIds), draftRevision: 3, targetRevision: 12, effectiveChangeIds: s.selectedChangeIds, autoIncludedChangeIds: [], conflicts: [], mergedNodes: diff.baselineNodes.map((n, i) => s.selectedChangeIds.includes(`change-${i === 0 ? 'a' : 'b'}`) ? { ...n, x: n.x + 60 } : n), canApply: s.selectedChangeIds.length > 0 }));
  const repository = { simulateMerge, applyMerge: vi.fn().mockResolvedValue(undefined), ...overrides } as unknown as CanvasRepository;
  const onApplied = vi.fn().mockResolvedValue(undefined), onClose = vi.fn(), onSaveIndependent = vi.fn();
  render(<EditorialReviewWorkspace repository={repository} projectId="project" board={board} comparison={diff} readOnly={readOnly} onApplied={onApplied} onClose={onClose} onSaveIndependent={onSaveIndependent} />);
  return { repository, onApplied, onClose, onSaveIndependent };
}

it('点击变更正文只定位，勾选才改变模拟，确认前不写入正式画布', async () => {
  const { repository, onApplied } = setup(); const user = userEvent.setup();
  expect(await screen.findByText('尚未选择改动')).toBeInTheDocument();
  await user.click(screen.getAllByRole('button', { name: /移动位置/ })[1]);
  expect(screen.getByRole('checkbox', { name: '选择改动 2' })).not.toBeChecked();
  expect(repository.applyMerge).not.toHaveBeenCalled();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  const apply = screen.getByRole('button', { name: '采纳选中改动' });
  await waitFor(() => expect(apply).toBeEnabled());
  expect(repository.simulateMerge).toHaveBeenLastCalledWith('project', expect.objectContaining({ selectedChangeIds: ['change-a'], expectedTargetRevision: 12 }));
  await user.click(apply);
  await waitFor(() => expect(onApplied).toHaveBeenCalledWith('source'));
  expect(repository.applyMerge).toHaveBeenCalledTimes(1);
  expect(repository.applyMerge).toHaveBeenCalledWith('project', expect.objectContaining({ selectedChangeIds: ['change-a'] }), '["change-a"]');
});

it('异步旧预览不得覆盖新选择，未完成模拟时不允许提交', async () => {
  let finish!: (value: MergeSimulation) => void;
  const simulateMerge = vi.fn((_p: string, s: EditorialSelection) => s.selectedChangeIds.length === 1 ? new Promise<MergeSimulation>(resolve => { finish = resolve; }) : Promise.resolve({ simulationId: 'empty', draftRevision: 3, targetRevision: 12, effectiveChangeIds: [], autoIncludedChangeIds: [], conflicts: [], mergedNodes: [], canApply: false }));
  setup({ simulateMerge }); const user = userEvent.setup();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeDisabled();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  finish({ simulationId: 'old', draftRevision: 3, targetRevision: 12, effectiveChangeIds: ['change-a'], autoIncludedChangeIds: [], conflicts: [], mergedNodes: [], canApply: true });
  await screen.findByText('尚未选择改动');
  expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeDisabled();
});

it('冲突可读、只读不允许合并，另存不冒充应用回原版', async () => {
  const { onSaveIndependent, repository } = setup({ simulateMerge: vi.fn().mockResolvedValue({ simulationId: 'blocked', draftRevision: 3, targetRevision: 12, effectiveChangeIds: [], autoIncludedChangeIds: [], conflicts: ['原画布已有修改'], mergedNodes: [], canApply: false }) });
  expect(await screen.findByText('原画布已有修改')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: '另存为独立方案' }));
  expect(onSaveIndependent).toHaveBeenCalledOnce(); expect(repository.applyMerge).not.toHaveBeenCalled();
});

it('预览模式切换保留缩放和平移，文本输入不会触发比较快捷键', async () => {
  setup(); const user = userEvent.setup();
  const pane = document.querySelector('.editorial-preview-pane')!;
  fireEvent.wheel(pane, { deltaY: -1, clientX: 100, clientY: 100 });
  const matrix = screen.getByTestId('editorial-transform').style.transform;
  await user.click(screen.getByRole('button', { name: '并排对比' }));
  expect(screen.getAllByTestId('editorial-transform')).toHaveLength(2);
  screen.getAllByTestId('editorial-transform').forEach(n => expect(n.style.transform).toBe(matrix));
  await user.click(screen.getByRole('button', { name: '差异叠层' }));
  expect(screen.getByTestId('editorial-transform').style.transform).toBe(matrix);
  const select = screen.getByRole('combobox'); fireEvent.keyDown(select, { ctrlKey: true, key: '3' });
  expect(screen.getByRole('button', { name: '差异叠层' })).toHaveAttribute('aria-pressed', 'true');
});

it('审稿选择按草案版本保留，键盘导航不会自动勾选', async () => {
  setup(); const user = userEvent.setup();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  const list = screen.getByRole('complementary', { name: '改动清单' });
  const buttons = within(list).getAllByRole('button', { name: /移动位置/ });
  buttons[0].focus(); await user.keyboard('{ArrowDown}'); expect(buttons[1]).toHaveFocus();
  expect(screen.getByRole('checkbox', { name: '选择改动 2' })).not.toBeChecked();
  cleanup(); setup(); expect(screen.getByRole('checkbox', { name: '选择改动 1' })).toBeChecked();
  cleanup(); setup({}, { ...comparison, draftRevision: 4 }); expect(screen.getByRole('checkbox', { name: '选择改动 1' })).not.toBeChecked();
});

it('同一属性冲突须明确选择，选择会绑定新的模拟与正式提交', async () => {
  const simulateMerge = vi.fn(async (_project: string, s: EditorialSelection): Promise<MergeSimulation> => ({
    simulationId: 'resolved', draftRevision: 3, targetRevision: 12, effectiveChangeIds: s.selectedChangeIds,
    autoIncludedChangeIds: [], conflicts: [], mergedNodes: comparison.baselineNodes, canApply: !!s.selectedChangeIds.length && !!s.resolutions?.length,
    attributeConflicts: s.selectedChangeIds.length && !s.resolutions?.length ? [{ changeId: 'change-a', nodeId: 'a', field: 'position', baseline: [0,0], current: [15,0], proposed: [60,0] }] : [],
  }));
  const { repository } = setup({ simulateMerge }); const user = userEvent.setup();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  const conflicts = await screen.findByRole('region', { name: '选择冲突处理方式' });
  expect(within(conflicts).getByText('当前版本: 15, 0')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeDisabled();
  await user.click(within(conflicts).getByRole('button', { name: '使用建议' }));
  await waitFor(() => expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: '采纳选中改动' }));
  expect(repository.applyMerge).toHaveBeenCalledWith('project', expect.objectContaining({ resolutions: [{ changeId: 'change-a', field: 'position', choice: 'use_draft' }] }), 'resolved');
});

it('分批采纳后留在审稿页，已采纳项禁选，下一批使用最新基线', async () => {
  const updated: CanvasComparison = { ...comparison, currentSourceRevision: 13, sourceChanged: true, changes: comparison.changes.map((c,i) => ({ ...c, decision: i ? 'pending' : 'accepted' })) };
  const compareDraft = vi.fn().mockResolvedValue(updated);
  const { repository, onApplied } = setup({ compareDraft }); const user = userEvent.setup();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  await waitFor(() => expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: '采纳选中改动' }));
  await waitFor(() => expect(screen.getByRole('checkbox', { name: '选择改动 1' })).toBeDisabled());
  expect(onApplied).not.toHaveBeenCalled();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 2' }));
  await waitFor(() => expect(repository.simulateMerge).toHaveBeenLastCalledWith('project', expect.objectContaining({ expectedTargetRevision: 13, selectedChangeIds: ['change-b'] })));
  await user.click(screen.getByRole('button', { name: '查看原画布' }));
  expect(onApplied).toHaveBeenCalledWith('source');
});

it('拒绝和暂缓经持久命令保存，不使用正式采纳接口', async () => {
  const recordDecision = vi.fn().mockResolvedValue(undefined);
  const compareDraft = vi.fn().mockResolvedValue({ ...comparison, changes: comparison.changes.map((c,i) => ({ ...c, decision: i ? 'pending' : 'rejected' })) });
  const { repository } = setup({ recordDecision, compareDraft }); const user = userEvent.setup();
  await user.click(screen.getByRole('checkbox', { name: '选择改动 1' }));
  await user.click(screen.getByRole('button', { name: '不采纳' }));
  await waitFor(() => expect(recordDecision).toHaveBeenCalledWith('project', expect.objectContaining({ selectedChangeIds: ['change-a'] }), 'rejected'));
  expect(await screen.findByText(/未采纳/)).toBeInTheDocument();
  expect(repository.applyMerge).not.toHaveBeenCalled();
});

it('同一节点可以只选尺寸，已采纳的位置保留并禁用', async () => {
  const diff: CanvasComparison = { ...comparison, changes: [{ ...comparison.changes[0], fields: ['position','size'], acceptedFields: ['position'], decision: 'partial' }] };
  const { repository } = setup({}, diff); const user = userEvent.setup();
  await user.click(screen.getAllByRole('button', { name: /移动位置/ })[0]);
  expect(screen.getByRole('checkbox', { name: '位置' })).toBeDisabled();
  expect(screen.getByRole('checkbox', { name: '位置' })).toBeChecked();
  await user.click(screen.getByRole('checkbox', { name: '尺寸' }));
  await waitFor(() => expect(repository.simulateMerge).toHaveBeenLastCalledWith('project', expect.objectContaining({ selectedChangeIds: ['change-a'], fieldsByChange: { 'change-a': ['size'] } })));
  await waitFor(() => expect(screen.getByRole('button', { name: '采纳选中改动' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: '采纳选中改动' }));
  expect(repository.applyMerge).toHaveBeenCalledWith('project', expect.objectContaining({ fieldsByChange: { 'change-a': ['size'] } }), '["change-a"]');
});
