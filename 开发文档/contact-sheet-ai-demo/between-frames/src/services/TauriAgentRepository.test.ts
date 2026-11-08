import { beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultAgentBudget } from '../domain/agent';
const ipc = vi.hoisted(() => vi.fn());
// 流式命令会 new Channel 并把 onmessage 接上去：夹具用内联的替身，避免 vi.mock 提升后引用未初始化的类。
vi.mock('@tauri-apps/api/core', () => ({
 invoke: ipc,
 convertFileSrc: (path: string) => `local:${path}`,
 Channel: class { onmessage?: (fragment: string) => void },
}));
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
it('本地重算只认计数：串方案、越界计数一律拒收，没记版本号仍算成功',async()=>{
 const receipt={draftId:'draft',draftRevision:2,count:5,observed:2,unobserved:3,omitted:1};
 const repository=new TauriAgentRepository();
 ipc.mockResolvedValue(receipt);
 expect(await repository.recheckEvidence!('p','draft')).toEqual(receipt);
 expect(ipc).toHaveBeenCalledWith('recheck_draft_evidence',{projectId:'p',draftId:'draft'});
 for(const value of [{...receipt,draftId:'other'},{...receipt,draftRevision:0},{...receipt,observed:-1},{...receipt,draftRevision:'现在'}]){
  ipc.mockResolvedValue(value);await expect(repository.recheckEvidence!('p','draft')).rejects.toThrow();
 }
 ipc.mockResolvedValue({...receipt,draftRevision:null});
 expect((await repository.recheckEvidence!('p','draft')).draftRevision).toBeNull();
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
 it('口径摘要只带计数过界：后端没这一栏算没核验，负数和文字一律整条拒收', async () => {
  const repository = new TauriAgentRepository();
  const digest = { draftId: 'draft', count: 5, observed: 2, unobserved: 3, pageTier: 1, detailTier: 1, otherVersion: 0, otherTool: 0, crossContractPairs: 1, omitted: 2, scanned: 2, draftRevision: 2, perPhoto: [{ photoId: 'p', contract: 'current_page_tier', used: true }, { photoId: 'q', contract: 'other_tool', used: false, pixelVersion: 'unreadable' }] };
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [digest] }], drafts: [] });
  expect((await repository.query('p')).tasks[0].draftEvidence).toEqual([digest]);
  ipc.mockResolvedValue({ tasks: [task], drafts: [] });
  expect((await repository.query('p')).tasks[0].draftEvidence).toEqual([]);
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [{ ...digest, observed: -1 }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [{ ...digest, pageTier: '全部' }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  // 陌生档位名一律整条拒收：界面不能拿到一个自己都不认的口径。
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [{ ...digest, perPhoto: [{ photoId: 'p', contract: '最准的档', used: true }] }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
 });
 it('逐张原图状态只接收四种已知结果，旧回执没这一栏仍可读取', async () => {
  const repository = new TauriAgentRepository();
  const perPhoto = ['missing', 'changed', 'matches', 'unchecked'].map(originalStatus => ({
   photoId: originalStatus, contract: null, used: false, pixelVersion: 'unreadable', originalStatus,
  }));
  const digest = { draftId: 'draft', count: 4, observed: 0, unobserved: 0, pageTier: 0, detailTier: 0, otherVersion: 0, otherTool: 0, crossContractPairs: 0, omitted: 4, scanned: 4, draftRevision: 2, perPhoto };
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [digest] }], drafts: [] });
  expect((await repository.query('p')).tasks[0].draftEvidence).toEqual([digest]);
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [{ ...digest, perPhoto: [{ photoId: 'old', contract: null, used: false, pixelVersion: 'unrecorded' }] }] }], drafts: [] });
  expect((await repository.query('p')).tasks[0].draftEvidence?.[0].perPhoto[0].originalStatus).toBeUndefined();
  ipc.mockResolvedValue({ tasks: [{ ...task, draftEvidence: [{ ...digest, perPhoto: [{ ...perPhoto[0], originalStatus: 'probably-missing' }] }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
 });
 it('拒绝负数用量与超出后端范围的图像预算', async () => {
  const repository = new TauriAgentRepository();
  ipc.mockResolvedValue({ tasks: [{ ...task, usage: { ...task.usage, costMinorUsd: -1 } }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  ipc.mockResolvedValue({ tasks: [{ ...task, budget: { ...task.budget, maxCloudImages: 101 } }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
 });
 it('图像版本证据原样过界：记过的、没记过的、读不出的、原图换过的都不合并，陌生状态整条被拒', async () => {
  const repository = new TauriAgentRepository();
  const inspections = [
   { photo_id: 'a', visible_facts: ['左侧有深色主体'], interpretation: '形成停顿', uncertainty: '细节难辨', imageVersion: { previewSha256: 'a'.repeat(64), pixelVersion: 'verified', sourceMatches: true } },
   { photo_id: 'b', visible_facts: ['地平线偏上'], interpretation: '留出呼吸', uncertainty: '裁切未定', imageVersion: { previewSha256: null, pixelVersion: 'unrecorded', sourceMatches: true } },
   { photo_id: 'c', visible_facts: ['逆光'], interpretation: '氛围优先', uncertainty: '曝光不可调', imageVersion: { previewSha256: 'b'.repeat(64), pixelVersion: 'unreadable', sourceMatches: false } },
  ];
  ipc.mockResolvedValue({ tasks: [{ ...task, inspectedPhotos: inspections }], drafts: [] });
  const data = await repository.query('p');
  expect(data.tasks[0].inspectedPhotos).toMatchObject([
   { imageVersion: { previewSha256: 'a'.repeat(64), pixelVersion: 'verified', sourceMatches: true } },
   { imageVersion: { previewSha256: null, pixelVersion: 'unrecorded' } },
   { imageVersion: { pixelVersion: 'unreadable', sourceMatches: false } },
  ]);
  ipc.mockResolvedValue({ tasks: [{ ...task, inspectedPhotos: [{ ...inspections[0], imageVersion: { pixelVersion: 'probably-fine' } }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
 });
 it('知识出处、审校状态与创作记忆来路原样过界，字段变形时整条投影被拒', async () => {
  const repository = new TauriAgentRepository();
  const references = [
   { kind: 'knowledge', label: 'Stephen Shore', citationId: 'p-1', version: 3, passage: '这条断言已核实。', source: 'https://fixture.example/collection', institution: 'Fixture Museum', reviewStatus: 'verified', disputed: false },
   { kind: 'knowledge', label: 'Diane Arbus', citationId: 'p-2', reviewStatus: 'imported', disputed: true, detailError: { code: 'KNOWLEDGE_UNAVAILABLE', message: '知识服务暂时连不上，不能视为没有出处' } },
   { kind: 'context', label: 'chatContext_history', section: 'history', version: 9, visualEvidence: false },
   { kind: 'memory', label: 'creative_memory', passage: '偏向留白的构图', memoryId: 7, origin: 'user_confirmed', scope: 'cross_project', sourceProjectId: 'p-source', rounds: 2, retrievals: 1, memoryStatus: 'confirmed', memoryDisabled: false, visualEvidence: false },
   { kind: 'memory', label: 'creative_memory', passage: null, memoryId: 8, origin: 'rule_inferred', scope: 'project_local', sourceProjectId: 'p', rounds: 1, memoryStatus: null, memoryDisabled: null },
  ];
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: references }], drafts: [] });
  const data = await repository.query('p');
  expect(data.tasks[0].creativeReferences).toMatchObject([
   { reviewStatus: 'verified', institution: 'Fixture Museum', source: 'https://fixture.example/collection' },
   { disputed: true, detailError: { code: 'KNOWLEDGE_UNAVAILABLE' } },
   { kind: 'context', section: 'history', version: 9 },
   { kind: 'memory', memoryId: 7, origin: 'user_confirmed', scope: 'cross_project', sourceProjectId: 'p-source', rounds: 2, retrievals: 1, memoryStatus: 'confirmed', memoryDisabled: false, passage: '偏向留白的构图' },
   { kind: 'memory', memoryId: 8, origin: 'rule_inferred', passage: null, rounds: 1, memoryStatus: null, memoryDisabled: null },
  ]);
  // 没被主动查询过的记忆，Rust 就不带这个字段，前端也不会凭空补一个次数。
  expect(data.tasks[0].creativeReferences?.[4].retrievals).toBeUndefined();
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...references[1], detailError: { code: 1, message: 'x' } }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  // 记忆条的轮数、查询次数与正文必须原样过界：结构错误的读数不能被当成真的用过。
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...references[3], rounds: '2' }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...references[3], retrievals: '1' }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
  // "现在这条还作不作数"同理：把停用写成字符串不能被解释成"已经停用"。
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...references[3], memoryDisabled: 'yes' }] }], drafts: [] });
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

it('聊天两条发送路都带上发送前留存的编号，断流后凭它只读核对', async () => {
 const repo = new TauriAgentRepository();
 const turns = [{ role: 'user' as const, content: '逆光怎么拍' }];
 const id = '3f2a1b0c-9d8e-4f70-a1b2-c3d4e5f60718';
 ipc.mockResolvedValueOnce('一次性回复');
 expect(await repo.chat(turns, id)).toBe('一次性回复');
 expect(ipc).toHaveBeenCalledWith('agent_chat', { turns, requestId: id });
 // 流式命令把编号和 channel 一起交给 Rust，退回路径也沿用同一个编号。
 ipc.mockResolvedValueOnce('流式回复');
 const delta = vi.fn();
 expect(await repo.chat(turns, id, delta)).toBe('流式回复');
 const streamArgs = ipc.mock.calls[1][1] as { turns: unknown; requestId: string; onDelta: { onmessage?: (fragment: string) => void } };
 expect(streamArgs.requestId).toBe(id);
 streamArgs.onDelta.onmessage?.('逆光');
 expect(delta).toHaveBeenCalledWith('逆光');
 expect(ipc).toHaveBeenCalledWith('agent_chat_stream', expect.objectContaining({ turns, requestId: id }));
});

it('聊天回执只在已完成时接受正文，其余状态必须空正文', async () => {
 const repo = new TauriAgentRepository();
 ipc.mockResolvedValueOnce({ state: 'completed', text: '核对到的回复' });
 expect(await repo.chatJob('3f2a1b0c-9d8e-4f70-a1b2-c3d4e5f60718')).toEqual({ state: 'completed', text: '核对到的回复' });
 expect(ipc).toHaveBeenCalledWith('agent_chat_job', { requestId: '3f2a1b0c-9d8e-4f70-a1b2-c3d4e5f60718' });
 for (const value of [{ state: 'pending', text: null }, { state: 'rejected', text: null }, { state: 'expired', text: null }, { state: 'unknown', text: null }]) {
  ipc.mockResolvedValueOnce(value);
  expect(await repo.chatJob('3f2a1b0c-9d8e-4f70-a1b2-c3d4e5f60718')).toEqual(value);
 }
 // 说"已完成"却给不出正文，或还在处理却塞来一段文字，都属于对不上账的回执。
 for (const value of [{ state: 'completed', text: '' }, { state: 'completed' }, { state: 'pending', text: '半句' }, { state: 'settled', text: null }]) {
  ipc.mockResolvedValueOnce(value);
  await expect(repo.chatJob('3f2a1b0c-9d8e-4f70-a1b2-c3d4e5f60718')).rejects.toThrow();
 }
});
it('记忆正文版本状态必须可识别，未核对的正文不能冒充历史',async()=>{
 const repository=new TauriAgentRepository();
 for(const memoryTextVersion of ['verified','changed','unrecorded','source_unavailable']){
  const reference={kind:'memory',label:'creative_memory',memoryTextVersion,passage:memoryTextVersion==='verified'?'原文':null};
  ipc.mockResolvedValue({tasks:[{...task,creativeReferences:[reference]}],drafts:[]});
  expect((await repository.query('p')).tasks[0].creativeReferences?.[0].memoryTextVersion).toBe(memoryTextVersion);
 }
 for(const invalid of [{memoryTextVersion:'invented',passage:null},{memoryTextVersion:'changed',passage:'新文字'}]){
  ipc.mockResolvedValue({tasks:[{...task,creativeReferences:[{kind:'memory',label:'creative_memory',...invalid}]}],drafts:[]});
  await expect(repository.query('p')).rejects.toThrow();
 }
});

describe('知识引用"为什么算这条"的回执边界', () => {
 const base = { kind: 'knowledge', label: '色彩与版本比较', citationId: 'FR-K004' };
 const local = { kind: 'local_terms', termCount: 2, matchedTerms: ['色彩', '调色'], fields: ['title', 'tags'], matched: 4, returned: 1 };
 it('三种命中理由原样过界，自相矛盾或陌生的理由整份投影被拒', async () => {
  const repository = new TauriAgentRepository();
  for (const matchReason of [local, { kind: 'builtin_method' }, { kind: 'cloud_search', relevance: 0.5 }, { kind: 'cloud_search', relevance: null }]) {
   ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...base, matchReason }] }], drafts: [] });
   expect((await repository.query('p')).tasks[0].creativeReferences?.[0].matchReason).toEqual(matchReason);
  }
  for (const matchReason of [
   { ...local, termCount: 0 },
   { ...local, matchedTerms: ['一', '二', '三'] },
   { ...local, returned: 9 },
   { ...local, fields: [] },
   { ...local, fields: ['author'] },
   { kind: 'cloud_search', relevance: '高' },
   { kind: 'cloud_search' },
   { kind: 'because_i_said_so' },
  ]) {
   ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...base, matchReason }] }], drafts: [] });
   await expect(repository.query('p')).rejects.toThrow();
  }
 });
 it('只有摘要的标记必须是布尔值，回执没带理由时不补一个', async () => {
  const repository = new TauriAgentRepository();
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...base, summaryOnly: true }] }], drafts: [] });
  const reference = (await repository.query('p')).tasks[0].creativeReferences?.[0];
  expect(reference?.summaryOnly).toBe(true);
  expect(reference?.matchReason).toBeUndefined();
  ipc.mockResolvedValue({ tasks: [{ ...task, creativeReferences: [{ ...base, summaryOnly: 'yes' }] }], drafts: [] });
  await expect(repository.query('p')).rejects.toThrow();
 });
});

describe('检索台账的回执边界', () => {
 const base = { callId: 'c1', tool: 'search_photography_knowledge', query: '色彩', returned: 0 };
 const reason = { code: 'KNOWLEDGE_PROTOCOL', message: '响应不是检索列表' };
 it('四种结局原样过界，结论与数字对不上就整份投影被拒', async () => {
  const repository = new TauriAgentRepository();
  for (const search of [
   { ...base, returned: 2, localMatched: 1, cloudAvailable: true, outcome: 'found' },
   { ...base, cloudAvailable: true, outcome: 'no_match' },
   { ...base, cloudAvailable: null, outcome: 'no_match' },
   { ...base, cloudAvailable: false, cloudError: { code: 'AUTH_REQUIRED', message: '需要授权' }, outcome: 'unavailable' },
   { ...base, query: '', error: reason, outcome: 'failed' },
  ]) {
   ipc.mockResolvedValue({ tasks: [{ ...task, knowledgeSearches: [search] }], drafts: [] });
   expect((await repository.query('p')).tasks[0].knowledgeSearches?.[0]).toEqual(search);
  }
  for (const search of [
   { ...base, outcome: 'found' },
   { ...base, returned: 3, outcome: 'no_match' },
   { ...base, returned: 3, cloudAvailable: false, outcome: 'unavailable' },
   { ...base, cloudAvailable: true, outcome: 'unavailable' },
   { ...base, cloudAvailable: false, outcome: 'no_match' },
   { ...base, outcome: 'failed' },
   { ...base, error: reason, outcome: 'no_match' },
   { ...base, outcome: 'maybe' },
   { ...base, tool: 'search_everything', outcome: 'no_match' },
   { ...base, returned: -1, outcome: 'no_match' },
   { ...base, localMatched: '0', outcome: 'no_match' },
   { ...base, query: 'x'.repeat(201), outcome: 'no_match' },
   { ...base, cloudAvailable: false, cloudError: { code: '', message: '空编号' }, outcome: 'unavailable' },
  ]) {
   ipc.mockResolvedValue({ tasks: [{ ...task, knowledgeSearches: [search] }], drafts: [] });
   await expect(repository.query('p')).rejects.toThrow();
  }
 });
 it('回执没带台账时补空数组，旧任务照样过界', async () => {
  const repository = new TauriAgentRepository();
  ipc.mockResolvedValue({ tasks: [task], drafts: [] });
  expect((await repository.query('p')).tasks[0].knowledgeSearches).toEqual([]);
 });
});
