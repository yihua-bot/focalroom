import { beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultAgentBudget } from '../domain/agent';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
import { TauriAgentRepository } from './TauriAgentRepository';

const task = {
 id: 'vision-task', goal: '视觉选片', baseRevision: 1, providerId: 'openai', state: 'paused', reason: '预算',
 budget: { ...defaultAgentBudget, maxCloudImages: 2 },
 usage: { modelTurns: 2, toolCalls: 2, inputEstimate: 1200, outputEstimate: 500, cloudImages: 2, costMinorUsd: 5 },
 elapsedMs: 2000, question: null, finalDraftIds: [], steps: [], createdAt: 'now',
};
beforeEach(() => ipc.mockReset());
it('能力投影保留声明来源，拒绝缺失能力字段及不支持的能力', async () => {
 const repository = new TauriAgentRepository();
 const config = { providerId: 'openai', modelId: 'custom', ready: true, apiKeyTail: 'abcd', keySource: 'env', baseUrl: 'https://example.invalid', promptVersion: 'm3-openai-6', consent: null, capabilities: ['tool_calling', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null };
 ipc.mockResolvedValue(config);
 expect(await repository.aiConfig('p')).toMatchObject({ capabilities: config.capabilities, capabilitySource: 'operator_declaration' });
 ipc.mockResolvedValue({ ...config, capabilities: ['embedding_or_multimodal'] });
 await expect(repository.aiConfig('p')).rejects.toThrow();
 const missing = { ...config } as Record<string, unknown>;
 delete missing.capabilities;
 ipc.mockResolvedValue(missing);
 await expect(repository.aiConfig('p')).rejects.toThrow();
});
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
describe('视觉任务 IPC 投影', () => {
 it('接收非零图像预算、外发数量与费用，允许任务暂停后继续显示', async () => {
  ipc.mockResolvedValue({ tasks: [task], drafts: [] });
  const data = await new TauriAgentRepository().query('p');
  expect(data.tasks[0].usage).toEqual(task.usage);
  expect(data.tasks[0].budget.maxCloudImages).toBe(2);
  expect(ipc).toHaveBeenCalledWith('get_agent_projection', { projectId: 'p' });
 });
 it('拒绝负数用量与超出后端范围的图像预算', async () => {
  const repository = new TauriAgentRepository();
  ipc.mockResolvedValue({ tasks: [{ ...task, usage: { ...task.usage, costMinorUsd: -1 } }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  ipc.mockResolvedValue({ tasks: [{ ...task, budget: { ...task.budget, maxCloudImages: 101 } }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
 });
});
