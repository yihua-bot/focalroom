import { render, screen, waitFor } from '@testing-library/react';
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
    { id: 1, projectId: 'p1', photoId: 'a', kind: 'keep', detail: {}, createdAt: '2026-10-14T01:00:00Z' },
    { id: 2, projectId: 'p1', photoId: 'b', kind: 'note', detail: {}, createdAt: '2026-10-14T01:01:00Z' },
    { id: 3, projectId: 'p1', photoId: 'c', kind: 'keep', detail: {}, createdAt: '2026-10-14T01:02:00Z' },
    { id: 4, projectId: 'p1', photoId: null, kind: 'comparison', detail: {}, createdAt: '2026-10-14T01:03:00Z' },
  ],
  hypotheses: [hypothesis],
  notice: '证据仅来自人工命令；Agent 草案不构成偏好。假设为规则推断，可纠正或停用。',
};

function makeRepository() {
  return {
    memory: {
      query: vi.fn().mockResolvedValue(projection),
      correct: vi.fn().mockResolvedValue({ ...hypothesis, status: 'corrected', disabled: true, userCorrection: '不是我的偏好' }),
      setEnabled: vi.fn().mockImplementation((_p: string, _id: number, enabled: boolean) => Promise.resolve({ ...hypothesis, disabled: !enabled })),
    },
  } as unknown as { memory: { query: Mock; correct: Mock; setEnabled: Mock } };
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
  it('渲染假设卡与证据时间线，声明 Agent 选择不计证据', async () => {
    mount(makeRepository());
    expect(await screen.findByText(hypothesis.statement)).toBeInTheDocument();
    expect(screen.getByText(/证据 3 条 · 反例 1 条/)).toBeInTheDocument();
    expect(screen.getByText(/Agent 的草案与建议不会出现在这里/)).toBeInTheDocument();
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
});
