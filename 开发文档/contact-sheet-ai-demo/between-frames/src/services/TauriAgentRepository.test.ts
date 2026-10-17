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
beforeEach(() => { ipc.mockReset(); });
it('独立观察配置按角色保留能力，并拒绝残缺的观察能力投影', async () => {
 const config = { observation: { modelId: 'observer', capabilities: ['vision', 'structured_output', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null }, consentProvider: 'openai@' + 'a'.repeat(64), providerId: 'openai', modelId: 'curator', ready: true, apiKeyTail: null, keySource: null, baseUrl: 'https://example.invalid', promptVersion: 'm3-openai-9', consent: null, capabilities: ['tool_calling', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null };
 ipc.mockResolvedValue(config);
 expect(await new TauriAgentRepository().aiConfig('p')).toMatchObject({ modelId: 'curator', observation: config.observation });
 ipc.mockResolvedValue({ ...config, observation: { ...config.observation, capabilities: ['embedding_or_multimodal'] } });
 await expect(new TauriAgentRepository().aiConfig('p')).rejects.toThrow();
 const incomplete = { ...config.observation } as Record<string, unknown>;
 delete incomplete.capabilities;
 ipc.mockResolvedValue({ ...config, observation: incomplete });
 await expect(new TauriAgentRepository().aiConfig('p')).rejects.toThrow();
});
it('能力投影保留声明来源，拒绝缺失能力字段及不支持的能力', async () => {
 const repository = new TauriAgentRepository();
 const config = { observation: null, consentProvider: 'openai@' + 'a'.repeat(64), providerId: 'openai', modelId: 'custom', ready: true, apiKeyTail: 'abcd', keySource: 'env', baseUrl: 'https://example.invalid', promptVersion: 'm3-openai-6', consent: null, capabilities: ['tool_calling', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null };
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

it('授权请求绑定界面收件人，通信重试保持身份，错误收件人回执拒绝', async () => {
 const recipient = 'openai@' + 'a'.repeat(64);
 ipc.mockRejectedValueOnce(new Error('lost reply')).mockImplementationOnce((_name, { request }) => ({
  id: 1, projectId: request.projectId, mode: request.mode, provider: request.provider, quota: request.quota, used: 0, grantedAt: 'now', revokedAt: null,
 }));
 const repo = new TauriAgentRepository();
 expect((await repo.setConsent('p', 'task', 4, recipient)).provider).toBe(recipient);
 expect(ipc.mock.calls[0]).toEqual(ipc.mock.calls[1]);
 expect(ipc.mock.calls[0][1].request.provider).toBe(recipient);
 ipc.mockResolvedValue({ id: 1, projectId: 'p', mode: 'task', provider: 'openai@' + 'b'.repeat(64), quota: 4, used: 0, grantedAt: 'now', revokedAt: null });
 await expect(repo.setConsent('p', 'task', 4, recipient)).rejects.toThrow('身份不匹配');
});
it('缺失收件人不能创建授权，禁止图像不依赖服务配置，业务拒绝不重试', async () => {
 const repo = new TauriAgentRepository();
 await expect(repo.setConsent('p', 'project', 100)).rejects.toThrow('授权标识缺失');
 expect(ipc).not.toHaveBeenCalled();
 ipc.mockImplementationOnce((_name, { request }) => ({
  id: 1, projectId: request.projectId, mode: request.mode, provider: request.provider, quota: request.quota, used: 0, grantedAt: 'now', revokedAt: null,
 }));
 await repo.setConsent('p', 'deny', 0, null);
 expect(ipc.mock.calls[0][1].request.provider).toBe('openai');
 ipc.mockReset().mockRejectedValue({ code: 'CONSENT_TARGET_CHANGED', message: '服务已变化' });
 await expect(repo.setConsent('p', 'task', 4, 'openai@' + 'a'.repeat(64))).rejects.toMatchObject({ code: 'CONSENT_TARGET_CHANGED', message: '服务已变化' });
 expect(ipc).toHaveBeenCalledTimes(1);
});
