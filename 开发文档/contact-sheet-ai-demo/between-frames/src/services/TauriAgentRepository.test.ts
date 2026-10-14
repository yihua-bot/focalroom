import { beforeEach, expect, it, vi } from 'vitest';
import { defaultAgentBudget } from '../domain/agent';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
import { TauriAgentRepository } from './TauriAgentRepository';
beforeEach(() => ipc.mockReset());
it('任务通信重试保留 requestId 和预算，不重复创建任务', async () => {
 ipc.mockRejectedValueOnce(new Error('lost reply')).mockResolvedValueOnce({ ok: true, data: { taskId: 'task' }, error: null, revision: null, requestId: 'ack' });
 const repo = new TauriAgentRepository();
 expect(await repo.request('project', { type: 'StartTask', goal: '两版', selectedPhotoIds: [], targetCount: 25, budget: defaultAgentBudget })).toBe('task');
 expect(ipc.mock.calls[0]).toEqual(ipc.mock.calls[1]);
 expect(ipc.mock.calls[0][1].request).not.toHaveProperty('actorType');
});
it('恢复业务冲突不自动换身份重试', async () => {
 ipc.mockResolvedValueOnce({ ok: false, data: null, error: { code: 'REVISION_CONFLICT', message: '项目已变', retryable: false }, revision: 2, requestId: 'ack' });
 await expect(new TauriAgentRepository().request('project', { type: 'ResumeTask', taskId: 'task', budget: defaultAgentBudget })).rejects.toMatchObject({ code: 'REVISION_CONFLICT' });
 expect(ipc).toHaveBeenCalledTimes(1);
});
it('查询拒绝未经校验的任务或虚构 projection 格式', async () => {
 ipc.mockResolvedValueOnce({ tasks: [{ id: 'fake', state: 'successful' }], drafts: [] });
 await expect(new TauriAgentRepository().query('project')).rejects.toThrow();
});
