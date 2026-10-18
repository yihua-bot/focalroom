import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi, type Mock } from 'vitest';
import { PreferencePanel } from './PreferencePanel';
import type { MemoryHypothesis, MemoryProjection } from '../../domain/memory';
import type { PreviewEvent, Project, DesktopProjectRepository } from '../../domain/models';

const project = { id: 'p1', title: 't', brief: '', revision: 3, photos: [], sequence: [], events: [] } as unknown as Project;
const events: PreviewEvent[] = [];
const hypothesis: MemoryHypothesis = {
  id: 7, projectId: 'p1', scope: 'project_local',
  statement: '本项目多次人工判断的说明中出现「留白」（3 条证据）',
  evidenceIds: [1, 2, 3], counterexampleIds: [4],
  status: 'unconfirmed', confidence: 'tentative', userCorrection: null,
  disabled: false, createdAt: '2026-10-14T01:00:00Z', updatedAt: '2026-10-14T01:00:00Z',
};
const projection: MemoryProjection = {
  evidence: [
    { id: 1, projectId: 'p1', photoId: 'a', kind: 'keep', detail: {}, excluded: false, createdAt: '2026-10-14T01:00:00Z' },
    { id: 2, projectId: 'p1', photoId: 'b', kind: 'note', detail: {}, excluded: false, createdAt: '2026-10-14T01:01:00Z' },
    { id: 3, projectId: 'p1', photoId: 'c', kind: 'keep', detail: {}, excluded: false, createdAt: '2026-10-14T01:02:00Z' },
    { id: 4, projectId: 'p1', photoId: null, kind: 'comparison', detail: {}, excluded: false, createdAt: '2026-10-14T01:03:00Z' },
  ],
  hypotheses: [hypothesis],
  notice: '证据仅来自人工命令；Agent 草案不构成偏好。假设为规则推断，可纠正或停用。',
};

function makeRepository() {
  return {
    memory: {
      query: vi.fn().mockResolvedValue(projection),
      request: vi.fn().mockResolvedValue({ changed: true, memoryRevision: 1, requestId: 'm1' }),
      correct: vi.fn().mockResolvedValue({ ...hypothesis, status: 'corrected', disabled: true, userCorrection: '不是我的偏好' }),
      setEnabled: vi.fn().mockImplementation((_p: string, _id: number, enabled: boolean) => Promise.resolve({ ...hypothesis, disabled: !enabled })),
    },
  } as unknown as { memory: { query: Mock; request: Mock; correct: Mock; setEnabled: Mock } };
}
function mount(repository?: unknown) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <PreferencePanel events={events} repository={repository as DesktopProjectRepository | undefined} project={project} />
    </QueryClientProvider>,
  );
}

describe('Memory Studio', () => {
  it('渲染假设卡、证据和本机记忆范围', async () => {
    mount(makeRepository());
    expect(await screen.findByText(hypothesis.statement)).toBeInTheDocument();
    expect(screen.getByText(/证据 3 条 · 反例 1 条/)).toBeInTheDocument();
    expect(screen.getByText('仅用于本机任务，不发送给云端模型。')).toBeVisible();
    // 证据时间线渲染（kindLabel 与 photoId 拼接会拆分文本节点，断言结构而非整串文本）。
    expect(document.querySelectorAll('.timeline li').length).toBeGreaterThanOrEqual(4);
  });
  it('纠正假设提交文本并调用 correct', async () => {
    const repository = makeRepository();
    mount(repository);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: '纠正此假设' }));
    await user.type(screen.getByRole('textbox', { name: '纠正说明' }), '不是我的偏好');
    await user.click(screen.getByRole('button', { name: '提交纠正' }));
    await waitFor(() => expect(repository.memory.correct).toHaveBeenCalledWith('p1', 7, '不是我的偏好'));
  });
  it('停用与恢复走 setEnabled', async () => {
    const repository = makeRepository();
    mount(repository);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: '停用检索' }));
    await waitFor(() => expect(repository.memory.setEnabled).toHaveBeenCalledWith('p1', 7, false));
  });
  it('浏览器模式无 repository 时降级显示事件时间线', () => {
    mount(undefined);
    expect(screen.queryByText(hypothesis.statement)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '纠正此假设' })).not.toBeInTheDocument();
  });
  it('确认、移除、恢复与证据排除走类型化命令，搜索传递关键词', async () => {
    const repository = makeRepository();
    mount(repository);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: '确认此假设' }));
    await waitFor(() => expect(repository.memory.request).toHaveBeenCalledWith('p1', { type: 'ConfirmHypothesis', hypothesisId: 7 }));
    await user.click(screen.getByRole('button', { name: '移除此假设' }));
    await waitFor(() => expect(repository.memory.request).toHaveBeenCalledWith('p1', { type: 'RetireHypothesis', hypothesisId: 7 }));
    await user.click(screen.getByRole('button', { name: '排除证据 #1' }));
    await waitFor(() => expect(repository.memory.request).toHaveBeenCalledWith('p1', { type: 'SetEvidenceExcluded', evidenceId: 1, excluded: true }));
    await user.type(screen.getByRole('textbox', { name: '搜索偏好关键词' }), '逆光');
    await user.click(screen.getByRole('button', { name: '搜索' }));
    await waitFor(() => expect(repository.memory.query).toHaveBeenCalledWith('p1', '逆光'));
  });
  it('已移除假设默认隐藏，展开后可恢复；展示反例与历史理由', async () => {
    const repository = makeRepository();
    repository.memory.query.mockResolvedValue({
      ...projection, hypotheses: [{ ...hypothesis, status: 'retired', disabled: true }],
      evidence: projection.evidence.map(item => item.id === 4 ? { ...item, detail: { reason: '留白太多', preference: 'right' }, excluded: true } : item),
    });
    mount(repository);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '恢复证据 #4' });
    expect(screen.queryByText(hypothesis.statement)).not.toBeInTheDocument();
    await user.click(screen.getByRole('checkbox', { name: '显示已移除假设' }));
    expect(screen.getByText(hypothesis.statement)).toBeInTheDocument();
    expect(screen.getByText('查看反例')).toBeInTheDocument();
    expect(screen.getAllByText('留白太多').length).toBeGreaterThan(0);
    await user.click(screen.getByRole('button', { name: '恢复此假设' }));
    await waitFor(() => expect(repository.memory.request).toHaveBeenCalledWith('p1', { type: 'RestoreHypothesis', hypothesisId: 7 }));
    await user.click(screen.getByRole('button', { name: '恢复证据 #4' }));
    await waitFor(() => expect(repository.memory.request).toHaveBeenCalledWith('p1', { type: 'SetEvidenceExcluded', evidenceId: 4, excluded: false }));
  });
  it('失败的记忆操作显示错误，查询失败也可见', async () => {
    const repository = makeRepository();
    repository.memory.request.mockRejectedValue(new Error('回执未保存'));
    mount(repository);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: '确认此假设' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('回执未保存');
  });

});

describe('v1 收尾评审：已纠正假设的删除入口', () => {
  it('corrected 状态仍提供删除按钮且不提供纠正/停用', async () => {
    const corrected = { ...hypothesis, status: 'corrected' as const, userCorrection: '不是偏好' };
    const repository = {
      memory: {
        query: vi.fn().mockResolvedValue({ ...projection, hypotheses: [corrected] }),
        correct: vi.fn(), setEnabled: vi.fn(),
        deleteHypothesis: vi.fn().mockResolvedValue(undefined),
        deleteEvidence: vi.fn(), purgeObservationCache: vi.fn(),
      },
    };
    mount(repository);
    expect(await screen.findByRole('button', { name: '删除假设' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '纠正此假设' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '停用检索' })).not.toBeInTheDocument();
  });
});


it('作者记忆开启需要确认，取消不发命令，关闭无需再次确认', async () => {
  const repository = makeRepository();
  const consent = vi.spyOn(window, 'confirm').mockReturnValue(false);
  mount(repository);
  const user = userEvent.setup();
  await screen.findByText(hypothesis.statement);
  const toggle = screen.getByRole('checkbox', { name: '允许离线 Agent 引用其他本地项目记忆' });
  expect(toggle).not.toBeChecked();
  await user.click(toggle);
  expect(repository.memory.request).not.toHaveBeenCalled();
  consent.mockReturnValue(true);
  repository.memory.query.mockResolvedValue({ ...projection, crossProjectEnabled: true, authorHypotheses: [] });
  await user.click(toggle);
  await waitFor(() => expect(toggle).toBeChecked());
  expect(repository.memory.request).toHaveBeenLastCalledWith('p1', { type: 'SetCrossProjectMemory', enabled: true });
  repository.memory.query.mockResolvedValue(projection);
  await user.click(toggle);
  await waitFor(() => expect(toggle).not.toBeChecked());
  expect(repository.memory.request).toHaveBeenLastCalledWith('p1', { type: 'SetCrossProjectMemory', enabled: false });
  expect(consent).toHaveBeenCalledTimes(2);
  consent.mockRestore();
});
it('其他项目假设只读，来源链接独立于当前证据和编辑卡', async () => {
  const repository = makeRepository();
  repository.memory.query.mockResolvedValue({ ...projection, crossProjectEnabled: true,
    authorHypotheses: [{ ...hypothesis, id: 19, projectId: 'source-project', scope: 'cross_project', status: 'confirmed' }] });
  mount(repository);
  const section = await screen.findByRole('region', { name: '其他本地项目的已确认线索' });
  expect(within(section).getByRole('link')).toHaveAttribute('href', '#/project/source-project');
  expect(within(section).queryByRole('button')).not.toBeInTheDocument();
  expect(within(section).queryByText('查看证据')).not.toBeInTheDocument();
  expect(screen.getAllByRole('button', { name: '确认此假设' })).toHaveLength(1);
});
it('查询失败不允许用默认值覆盖作者授权', async () => {
  const repository = makeRepository();
  repository.memory.query.mockRejectedValue(new Error('本地存储不可读'));
  mount(repository);
  await screen.findByRole('alert');
  expect(screen.getByRole('checkbox', { name: '允许离线 Agent 引用其他本地项目记忆' })).toBeDisabled();
});
