import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ProposalsPanel } from './ProposalsPanel';
import type { Project } from '../../domain/models';

const queryMock = vi.fn().mockResolvedValue([
  { id: 'p1', title: '克制版 25 张', state: 'submitted', author: 'agent:agent', revision: 2, approvalNonce: 'nonce-1', operations: [
    { operationId: 'op-1', photoId: 'photo-a1', op: 'add', toIndex: 0, reason: '开篇' },
    { operationId: 'op-2', photoId: 'photo-b2', op: 'remove', toIndex: null, reason: '' },
  ] },
  { id: 'p2', title: '已完成提案', state: 'applied', author: 'user:u', revision: 3, approvalNonce: 'n2', operations: [] },
]);
const applyMock = vi.fn();
beforeEach(() => { applyMock.mockReset().mockResolvedValue({ applied: 1, failed: 0, discarded: false }); });
vi.mock('../../services/TauriProposalsRepository', () => ({
  TauriProposalsRepository: class {
    query = queryMock;
    apply = applyMock;
  },
}));

const project = { id: 'proj', title: 't', revision: 7, photos: [{ id: 'photo-a1', name: 'photo-a.jpg' }, { id: 'photo-b2', name: 'photo-b.jpg' }] } as unknown as Project;
function mount(enabled = true) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={client}><ProposalsPanel project={project} enabled={enabled} /></QueryClientProvider>);
}

describe('v2 提案审批面板', () => {
  it('列出提案状态与作者', async () => {
    mount();
    expect(await screen.findByText('克制版 25 张')).toBeInTheDocument();
    expect(screen.getByText(/待审批 · 2 项调整 · AI/)).toBeInTheDocument();
    expect(screen.getByText(/已执行/)).toBeInTheDocument();
  });
  it('展开待审批提案：勾选调整并采纳，确认凭据不作为表单暴露', async () => {
    mount();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: /克制版/ }));
    expect(screen.getByLabelText(/加入序列/)).toBeInTheDocument();
    expect(screen.getByLabelText(/移出序列/)).toBeInTheDocument();
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
    await user.click(screen.getByLabelText(/移出序列/));
    await user.click(screen.getByRole('button', { name: '执行所选操作' }));
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('已采纳 1 项调整'));
    expect(applyMock).toHaveBeenCalledWith('proj', 7, 'p1', 2, ['op-1'], 'nonce-1', expect.any(String));
  });
  it('业务失败显示错误，不宣称执行成功', async () => {
    applyMock.mockRejectedValueOnce(new Error('项目版本冲突，请刷新'));
    mount();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: /克制版/ }));
    await user.click(screen.getByRole('button', { name: '执行所选操作' }));
    expect(await screen.findByRole('status')).toHaveTextContent('项目版本冲突');
    expect(screen.getByRole('status')).not.toHaveTextContent('已执行');
    expect(applyMock).toHaveBeenCalledWith('proj', 7, 'p1', 2, ['op-1', 'op-2'], 'nonce-1', expect.any(String));
  });
  it('成功提示使用后端实际执行数量', async () => {
    applyMock.mockResolvedValueOnce({ applied: 1, failed: 0, discarded: false });
    mount();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: /克制版/ }));
    await user.click(screen.getByRole('button', { name: '执行所选操作' }));
    expect(await screen.findByRole('status')).toHaveTextContent('已采纳 1 项调整');
  });
  it('全部不勾时按钮为驳回语义', async () => {
    mount();
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: /克制版/ }));
    await user.click(screen.getByLabelText(/加入序列/));
    await user.click(screen.getByLabelText(/移出序列/));
    expect(screen.getByRole('button', { name: '驳回提案' })).toBeInTheDocument();
  });
});
