import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SharingControls } from './SharingControls';
import type { SharingRepository, SharingStatus } from '../../domain/sharing';
describe('桌面共享迁移授权', () => {
  it('明确授权后才发起请求，完成后清除密码并呈现只读权威', async () => {
    let completed = false;
    const repository: SharingRepository = { status: vi.fn(async (): Promise<SharingStatus> => ({ project_id: 'p', authority: completed ? 'shared' : 'local' })), share: vi.fn(async (): Promise<SharingStatus> => { completed = true; return { project_id: 'p', authority: 'shared' }; }) };
    const changed = vi.fn(), busy = vi.fn(); render(<SharingControls repository={repository} projectId="p" revision={4} onChanged={changed} onBusyChange={busy} />);
    await waitFor(() => expect(screen.getByText('状态：本地项目')).toBeVisible());
    fireEvent.change(screen.getByLabelText('API 服务地址'), { target: { value: 'http://127.0.0.1:3002' } }); fireEvent.change(screen.getByLabelText('Web 评片地址'), { target: { value: 'http://127.0.0.1:1424' } }); fireEvent.change(screen.getByLabelText('已注册邮箱'), { target: { value: 'author@example.com' } }); fireEvent.change(screen.getByLabelText('账户密码'), { target: { value: 'private-password' } });
    const submit = screen.getByRole('button', { name: '确认迁移为共享项目' }); expect(submit).toBeDisabled(); fireEvent.click(screen.getByRole('checkbox')); expect(submit).toBeEnabled(); fireEvent.click(submit);
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('已迁移至共享评片'));
    expect(repository.share).toHaveBeenCalledWith(expect.objectContaining({ expectedRevision: 4, previewUploadAuthorized: true, action: 'resume' })); expect(busy.mock.calls).toEqual([[true], [false]]); expect(changed).toHaveBeenCalledOnce(); expect(screen.queryByLabelText('账户密码')).not.toBeInTheDocument();
  });
  it('中断迁移恢复绑定原服务，并提供需要云端核验的取消入口', async () => {
    const repository: SharingRepository = { status: vi.fn(async (): Promise<SharingStatus> => ({ project_id: 'p', authority: 'local-promoting', state: 'verifying', remote: { binding: { baseUrl: 'https://api.example.com', webUrl: 'https://app.example.com', userId: 'u', workspaceId: 'w' } } })), share: vi.fn() };
    render(<SharingControls repository={repository} projectId="p" revision={2} />); await waitFor(() => expect(screen.getByLabelText('API 服务地址')).toHaveValue('https://api.example.com')); expect(screen.getByLabelText('API 服务地址')).toBeDisabled(); expect(screen.getByRole('button', { name: '验证云端未提交并取消迁移' })).toBeDisabled(); expect(repository.share).not.toHaveBeenCalled();
  });
});
