import { beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultAgentBudget } from '../domain/agent';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc, convertFileSrc: (path: string) => `local:${path}` }));
import { TauriAgentRepository } from './TauriAgentRepository';

const task = {
 id: 'vision-task', goal: '视觉选片', baseRevision: 1, providerId: 'openai', state: 'paused', reason: '预算',
 budget: { ...defaultAgentBudget, maxCloudImages: 2 },
 usage: { modelTurns: 2, toolCalls: 2, inputEstimate: 1200, outputEstimate: 500, cloudImages: 2, costMinorUsd: 5 },
 elapsedMs: 2000, question: null, finalDraftIds: [], steps: [], createdAt: 'now',
};
beforeEach(() => { ipc.mockReset(); });
const selectionContext = {draftId:'child',parent:{kind:'selection',id:'parent',revision:2},parentTitle:'旧版',parentSummary:'保留开篇',parentState:'applied',parentBaseRevision:1,items:[{photoId:'a',photoHash:'a'.repeat(64),photoName:'a.jpg',position:0,action:'keep',rationale:'建议',currentHashMatches:true,thumbnailPath:'/cache/a.jpg'}],feedback:[],reviews:[{commandId:'review',kind:'apply',createdAt:'now',mode:'partial',photoIds:['a'],photoNames:['a.jpg'],partial:false,undoneAtDelegation:false}],observations:[],totalItems:1,totalFeedback:0,totalReviews:1,totalObservations:0,nextOffset:null,hasEarlier:false};
it('选片修订依据保留部分采纳、父版和本地图片来源，旧方案没有记录时返回空',async()=>{
 ipc.mockResolvedValue(selectionContext);const repository=new TauriAgentRepository();const page=await repository.selectionRevisionContext('p','child');
 expect(page!.reviews[0].mode).toBe('partial');expect(page!.items[0].thumbnailPath).toBe('local:/cache/a.jpg');expect(ipc).toHaveBeenCalledWith('selection_revision_context',{projectId:'p',draftId:'child',offset:0});
 ipc.mockResolvedValue(null);expect(await repository.selectionRevisionContext('p','child')).toBeNull();
});
it('选片依据拒绝串方案、错误种类、未知人工操作、倒退分页及不一致的采纳照片',async()=>{
 const repository=new TauriAgentRepository();
 for(const value of [{...selectionContext,draftId:'other'},{...selectionContext,parent:{...selectionContext.parent,kind:'board'}},{...selectionContext,nextOffset:0},{...selectionContext,reviews:[{...selectionContext.reviews[0],photoNames:[]}]},{...selectionContext,reviews:[{...selectionContext.reviews[0],kind:'unsafe'}]}]){
  ipc.mockResolvedValue(value);await expect(repository.selectionRevisionContext('p','child')).rejects.toThrow();
 }
});
it('选片依据中的照片版本不一致时隐藏当前像素，不能冒充旧照片',async()=>{
 ipc.mockResolvedValue({...selectionContext,items:[{...selectionContext.items[0],currentHashMatches:false}]});
 expect((await new TauriAgentRepository().selectionRevisionContext('p','child'))!.items[0].thumbnailPath).toBeNull();
});
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

it('原分析查询只调用专用入口，拒绝非法状态和重复IPC重试', async () => {
 const repo=new TauriAgentRepository();
 ipc.mockResolvedValueOnce({state:'pending',images:2});
 expect(await repo.queryVisionResult('p','t')).toEqual({state:'pending',images:2});
 expect(ipc).toHaveBeenCalledWith('query_vision_result',{projectId:'p',taskId:'t'});
 ipc.mockResolvedValueOnce({state:'retry_upload',images:2});
 await expect(repo.queryVisionResult('p','t')).rejects.toThrow();
 ipc.mockRejectedValueOnce(new Error('lost reply'));
 await expect(repo.queryVisionResult('p','t')).rejects.toThrow('lost reply');
 expect(ipc).toHaveBeenCalledTimes(3);
});

it('恢复能力必须与已知策略和期限匹配，缺少能力时保持旧协议',async()=>{
 const base={observation:null,consentProvider:'openai@'+'a'.repeat(64),providerId:'openai',modelId:'deepseek-flash',ready:true,apiKeyTail:null,keySource:'focalroom',baseUrl:'https://team.focroom.cc/api/v2/ai/deepseek-flash-v1',promptVersion:'test',capabilities:['vision','structured_output','instruction_following'],capabilitySource:'official_documentation',configurationError:null,consent:null,
 hostedAccount:{enabled:true,verified:true,policy:'deepseek-flash-v1',provider:'DeepSeek',model:'deepseek-flash',unit:'tokens',granted:1,spent:0,reserved:0,available:1}};
 const repo=new TauriAgentRepository();
 ipc.mockResolvedValue(base);expect((await repo.aiConfig('p')).hostedAccount).not.toHaveProperty('recovery');
 ipc.mockResolvedValue({...base,hostedAccount:{...base.hostedAccount,recovery:{enabled:true,policy:'encrypted-queue-v1',maxQueueSeconds:600}}});
 expect((await repo.aiConfig('p')).hostedAccount?.recovery).toEqual({enabled:true,policy:'encrypted-queue-v1',maxQueueSeconds:600});
 ipc.mockResolvedValue({...base,hostedAccount:{...base.hostedAccount,recovery:{enabled:true,policy:'other',maxQueueSeconds:600}}});
 await expect(repo.aiConfig('p')).rejects.toThrow();
});
