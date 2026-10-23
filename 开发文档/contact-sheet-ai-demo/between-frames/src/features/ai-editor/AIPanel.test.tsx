import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useUIStore } from '../../stores/uiStore';
import { AIPanel } from './AIPanel';
import type { AiConfig, AgentProjection, AgentTask } from '../../domain/agent';
import { defaultAgentBudget } from '../../domain/agent';
import type { Project } from '../../domain/models';
const project = { id: 'p1', title: 't', brief: '', revision: 3, photos: [{ id: 'a', status: 'undecided' }, { id: 'b', status: 'undecided' }], sequence: [], events: [] } as unknown as Project;
const readyConfig: AiConfig = { observation: null, consentProvider: 'openai@' + 'a'.repeat(64), providerId: 'openai', modelId: 'test-model', ready: true, apiKeyTail: 'abcd', keySource: 'keychain', baseUrl: 'https://x', promptVersion: 'test', capabilities: ['vision', 'tool_calling', 'structured_output', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null, consent: null };
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
function fixture(config: Partial<AiConfig> = {}, projection: AgentProjection = { tasks: [], drafts: [] }) {
 const agent = { query: vi.fn().mockResolvedValue(projection), request: vi.fn().mockResolvedValue('task-1'), subscribe: vi.fn().mockResolvedValue(() => {}), aiConfig: vi.fn().mockResolvedValue({ ...readyConfig, ...config }), setConsent: vi.fn().mockResolvedValue({ id: 9 }) };
 return { agent };
}
function mount(repository = fixture(), props: Partial<React.ComponentProps<typeof AIPanel>> = {}) {
 const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
 const panel = (extra = {}) => <QueryClientProvider client={client}><AIPanel repository={repository as never} project={project} selectedIds={['a']} initialGoal="选出有叙事力的照片" initialCount={1} onUpdated={() => {}} {...props} {...extra} /></QueryClientProvider>;
 const result = render(panel()); return { ...result, repository, panel, client };
}
const send = () => screen.getByRole('button', { name: '发送' });
const ready = () => waitFor(() => expect(send()).toBeEnabled());
function task(state: AgentTask['state']): AgentTask { return { id: 't1', state, goal: '旅行故事', baseRevision: 3, reason: '预算已用完', question: state === 'needs_input' ? '希望以人物为主吗？' : null, budget: defaultAgentBudget, usage: { modelTurns: 1, toolCalls: 1, inputEstimate: 1, outputEstimate: 1, cloudImages: 0, costMinorUsd: 0 }, elapsedMs: 1, finalDraftIds: [], providerId: 'test', steps: [], createdAt: 'now' }; }
beforeEach(() => { useUIStore.getState().reset(); useUIStore.setState({ aiDrafts: {} }); });
describe('选片对话与本次预览使用', () => {
 it.each([true, false])('达到等待时间后，明确延长并继续同一任务（请求未发送=%s）', async requestNotSent => {
  const paused = { ...task('paused'), elapsedMs: 480000, budgetIssue: { kind: 'time' as const, used: 480000, requested: 1, limit: 480000, stage: requestNotSent ? 'vision' : 'model_response', requestNotSent } };
  const repo = fixture({}, { tasks: [paused], drafts: [] }); mount(repo); await ready();
  if (requestNotSent) expect(screen.getByText(/这一次照片请求尚未发送/)).toBeVisible();
  else expect(screen.queryByText(/这一次照片请求尚未发送/)).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole('button', { name: '延长时间并继续' }));
  expect(repo.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ type: 'ResumeTask', taskId: 't1', budget: expect.objectContaining({ maxDurationSeconds: 960, maxCloudImages: 0, maxCostMinorUsd: 100 }) }));
 });
 it('步骤耗尽后继续会实际增加额度，不重复提交原来的上限', async () => {
  const paused = { ...task('paused'), usage: { ...task('paused').usage, toolCalls: 40 }, budgetIssue: { kind: 'steps' as const, used: 40, requested: 1, limit: 40, stage: 'vision', requestNotSent: true } };
  const repo = fixture({}, { tasks: [paused], drafts: [] }); mount(repo); await ready();
  await userEvent.click(screen.getByRole('button', { name: /^继续$/ }));
  expect(repo.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ type: 'ResumeTask', budget: expect.objectContaining({ maxToolCalls: 80, maxCloudImages: 0, maxCostMinorUsd: 100 }) }));
 });
 it.each(['images', 'cost', 'context', 'tokens'] as const)('%s 不自动提额，调整范围只填写要求，由用户再次发送', async kind => {
  const paused = { ...task('paused'), budgetIssue: { kind, used: 20, requested: 1, limit: 20, stage: 'vision', requestNotSent: true } };
  const repo = fixture({}, { tasks: [paused], drafts: [] }); mount(repo); await ready();
  expect(screen.queryByRole('button', { name: /^继续$/ })).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole('button', { name: '调整照片范围' }));
  expect(screen.getByLabelText('消息')).toHaveValue('旅行故事');
  expect(screen.getByLabelText('照片范围')).toHaveValue('selected');
  expect(repo.agent.request).not.toHaveBeenCalled();
  expect(repo.agent.setConsent).not.toHaveBeenCalled();
 });
 it('旧版混合报错显示可读提示，不暴露内部预算术语', async () => {
  mount(fixture({}, { tasks: [{ ...task('paused'), reason: '视觉请求预留超过图像、费用、工具或时间预算；未上传' }], drafts: [] }));
  await ready();
  expect(screen.getByText('这次分析已暂停。已保留进度，请调整照片范围后重新开始。')).toBeVisible();
  expect(screen.queryByText(/视觉请求预留/)).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /^继续$/ })).not.toBeInTheDocument();
 });
 it.each(['视觉请求的传输结果尚未确认；预算预留保留，请重新委托，恢复不会重复上传', '模型等待达到时间上限；未执行未确认工具'])('结果不明时说明可能的重复消耗，不显示未发送或继续按钮：%s', async reason => {
  const repo = fixture({}, { tasks: [{ ...task('paused'), reason }], drafts: [] });
  mount(repo); await ready();
  expect(screen.getByText(/上次照片分析的结果尚未确认/)).toBeVisible();
  expect(screen.queryByText(/这一次照片请求尚未发送/)).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /^继续$/ })).not.toBeInTheDocument();
  expect(repo.agent.request).not.toHaveBeenCalled();
 });
 it('同一对话入口发送自然语言，无需切换任务模式', async () => {
  const { repository } = mount(fixture(), { initialGoal: '给我一些画布布局思路' }); await ready();
  expect(screen.queryByRole('tablist')).not.toBeInTheDocument();
  await userEvent.click(send());
  expect(repository.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ type: 'StartConversation' }));
 });
 it('从当前画布发起时绑定原版与选区，不自动创建任务', async () => {
  useUIStore.getState().openCanvasAgent({ boardId: 'board-1', revision: 8, selectedNodeIds: ['node-1'] });
  const { repository } = mount(); await ready();
  expect(repository.agent.request).not.toHaveBeenCalled();
  expect(screen.getByText('基于当前画布')).toBeVisible();
  await userEvent.click(send());
  expect(repository.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ type: 'StartConversation', sourceBoard: { boardId: 'board-1', revision: 8, selectedNodeIds: ['node-1'] } }));
 });
 it('可从已完成任务打开画布方案，而不直接采纳', async () => {
  const candidate = { ...task('review_ready'), canvasTask: true, boardDrafts: [{ id: 'draft-board', projectId: 'p1', title: '叙事分支', state: 'draft', owningTaskId: 't1', revision: 3 }] };
  const { repository } = mount(fixture({}, { tasks: [candidate], drafts: [] })); await ready();
  await userEvent.click(await screen.findByRole('button', { name: '查看画布方案' }));
  expect(useUIStore.getState()).toMatchObject({ view: 'canvas', canvasBoardId: 'draft-board', aiOpen: false });
  expect(repository.agent.request).not.toHaveBeenCalled();
 });
 it('仅打开对话不会授权、发送任务或伪造回复', async () => { const { repository } = mount(); await ready(); expect(repository.agent.setConsent).not.toHaveBeenCalled(); expect(repository.agent.request).not.toHaveBeenCalled(); expect(screen.queryByText('管理图像授权')).not.toBeInTheDocument(); });
 it('发送时先绑定本次预览范围，再委托真实任务', async () => { const { repository } = mount(); await ready(); await userEvent.click(send()); await waitFor(() => expect(repository.agent.request).toHaveBeenCalled()); expect(repository.agent.setConsent).toHaveBeenCalledWith('p1', 'task', 100, readyConfig.consentProvider); expect(repository.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ type: 'StartConversation', consentId: 9, targetCount: 1, selectedPhotoIds: [], budget: expect.objectContaining({ maxCloudImages: 100 }) })); expect(repository.agent.setConsent.mock.invocationCallOrder[0]).toBeLessThan(repository.agent.request.mock.invocationCallOrder[0]); expect(screen.getByLabelText('消息')).toHaveValue(''); });
 it('授权写入失败时保留输入，不开始任务、不自动重试', async () => { const repo = fixture(); repo.agent.setConsent.mockRejectedValue(new Error('保存失败')); mount(repo); await ready(); await userEvent.click(send()); expect(await screen.findByRole('alert')).toHaveTextContent('保存失败'); expect(repo.agent.request).not.toHaveBeenCalled(); expect(repo.agent.setConsent).toHaveBeenCalledTimes(1); expect(screen.getByLabelText('消息')).not.toHaveValue(''); });
 it('服务身份在发送前改变时拒绝委托，不误发到新供应商', async () => { const repo = fixture(); mount(repo); await ready(); repo.agent.aiConfig.mockResolvedValue({ ...readyConfig, consentProvider: 'other' }); await userEvent.click(send()); expect(await screen.findByRole('alert')).toHaveTextContent('AI 已更新'); expect(repo.agent.setConsent).not.toHaveBeenCalled(); expect(repo.agent.request).not.toHaveBeenCalled(); });
 it('未知视觉接收方不能授权或发送', async () => { const repo = fixture({ consentProvider: null }); mount(repo); await ready(); await userEvent.click(send()); await screen.findByRole('alert'); expect(repo.agent.setConsent).not.toHaveBeenCalled(); expect(repo.agent.request).not.toHaveBeenCalled(); });
 it('发送前配置查询失败时拒绝委托', async () => { const repo = fixture(); mount(repo); await ready(); repo.agent.aiConfig.mockRejectedValue(new Error('network')); await userEvent.click(send()); await screen.findAllByRole('alert'); expect(repo.agent.request).not.toHaveBeenCalled(); });
 it('文本服务不授权图片，明确提示且以零图像预算运行', async () => { const repo = fixture({ capabilities: ['tool_calling', 'instruction_following'] }); mount(repo); await ready(); expect(screen.getByText(/当前服务不支持看图/)).toBeVisible(); await userEvent.click(send()); await waitFor(() => expect(repo.agent.request).toHaveBeenCalled()); expect(repo.agent.setConsent).not.toHaveBeenCalled(); expect(repo.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ consentId: null, budget: expect.objectContaining({ maxCloudImages: 0 }) })); });
 it('独立视觉模型的能力用于实际预览预算', async () => { const repo = fixture({ capabilities: ['tool_calling'], observation: { modelId: 'observer', capabilities: ['vision', 'structured_output', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null } }); mount(repo); await ready(); await userEvent.click(send()); await waitFor(() => expect(repo.agent.setConsent).toHaveBeenCalled()); });
 it.each([{ ready: false }, { configurationError: '配置无效' }, { keySource: 'focalroom', hostedError: '请先登录' }, { keySource: 'focalroom', hostedAccount: { enabled: true, verified: true, available: 0 } }, { keySource: 'focalroom', hostedAccount: { enabled: false, verified: true, available: 100 } }])('未就绪/未登录/零额度阻止发送：%j', async config => { const repo = fixture(config as Partial<AiConfig>); mount(repo); await screen.findByRole('status'); expect(send()).toBeDisabled(); fireEvent.submit(send().closest('form')!); expect(repo.agent.request).not.toHaveBeenCalled(); });
 it('选区为空不会隐式扩大为全项目', async () => { const { repository, rerender, panel } = mount(); await ready(); await userEvent.selectOptions(screen.getByLabelText('照片范围'), 'selected'); rerender(panel({ selectedIds: [] })); expect(send()).toBeDisabled(); fireEvent.submit(send().closest('form')!); expect(repository.agent.request).not.toHaveBeenCalled(); });
 it('选区发送只携带选中的 ID 和对应预算', async () => { const { repository } = mount(); await ready(); await userEvent.selectOptions(screen.getByLabelText('照片范围'), 'selected'); await userEvent.click(send()); await waitFor(() => expect(repository.agent.request).toHaveBeenCalled()); expect(repository.agent.setConsent).toHaveBeenCalledWith('p1', 'task', 100, readyConfig.consentProvider); expect(repository.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ selectedPhotoIds: ['a'] })); });
 it('连续提交只创建一次授权与任务', async () => { const { repository } = mount(); await ready(); fireEvent.submit(send().closest('form')!); fireEvent.submit(send().closest('form')!); await waitFor(() => expect(repository.agent.request).toHaveBeenCalledTimes(1)); expect(repository.agent.setConsent).toHaveBeenCalledTimes(1); });
 it('自然语言中明确的张数同步到保留选项', async () => { mount(); await ready(); fireEvent.change(screen.getByLabelText('消息'), { target: { value: '给旅行摄影书选出 7 张照片' } }); expect(screen.getByLabelText('保留张数')).toHaveValue(7); });
 it('快捷比较只填写消息，等待用户发送', async () => { const { repository } = mount(fixture(), { quickAction: { id: 'q1', goal: '比较这两张', count: 1, photoIds: ['a'] } }); await ready(); expect(screen.getByLabelText('消息')).toHaveValue('比较这两张'); expect(screen.getByLabelText('照片范围')).toHaveValue('selected'); expect(repository.agent.request).not.toHaveBeenCalled(); });
 it('追问回答发送反馈，不创建新任务或图片授权', async () => { const repo = fixture({}, { tasks: [task('needs_input')], drafts: [] }); mount(repo); await ready(); expect(screen.getByText('希望以人物为主吗？')).toBeVisible(); await userEvent.click(send()); await waitFor(() => expect(repo.agent.request).toHaveBeenCalledWith('p1', { type: 'SubmitFeedback', taskId: 't1', answer: '选出有叙事力的照片' })); expect(repo.agent.setConsent).not.toHaveBeenCalled(); });
 it('新消息不拼接或伪造历史，历史由执行引擎加载', async () => { const repo = fixture({}, { tasks: [task('review_ready')], drafts: [] }); mount(repo); await ready(); await userEvent.click(send()); await waitFor(() => expect(repo.agent.request).toHaveBeenCalled()); expect(repo.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ goal: '选出有叙事力的照片' })); });
 it('运行中的任务禁用再次发送，暂停按钮传递真实任务 ID', async () => { const repo = fixture({}, { tasks: [task('running')], drafts: [] }); mount(repo); await screen.findByText('执行中'); expect(send()).toBeDisabled(); await userEvent.click(screen.getByRole('button', { name: '暂停' })); await waitFor(() => expect(repo.agent.request).toHaveBeenCalledWith('p1', { type: 'PauseTask', taskId: 't1' })); });
 it('设置只提供照片数与等待时间，不暴露引擎参数', async () => { mount(fixture(), { settingsOpen: true }); expect(await screen.findByRole('dialog', { name: '设置' })).toBeVisible(); expect(screen.getByLabelText('本次分析次数上限')).toBeVisible(); expect(screen.getByLabelText('最长等待时间')).toBeVisible(); expect(screen.queryByLabelText('模型轮次上限')).not.toBeInTheDocument(); });
});

describe('统一摄影助手', () => {
 it('没有照片也能讨论摄影，且不授权或附带图像', async () => {
  const repo = fixture(); mount(repo, { project: { ...project, photos: [] }, selectedIds: [], initialGoal: '如何理解摄影叙事的节奏？' });
  await ready(); await userEvent.click(send());
  expect(repo.agent.setConsent).not.toHaveBeenCalled();
  expect(repo.agent.request).toHaveBeenCalledWith('p1', expect.objectContaining({ type: 'StartConversation', goal: '如何理解摄影叙事的节奏？', focusPhotoIds: [], consentId: null, budget: expect.objectContaining({ maxCloudImages: 0 }) }));
 });
 it('显示后端保存的回答和追问历史，不要求跳转任务页', async () => {
  const reply = { ...task('completed'), conversation: true, reason: '内部完成状态', messages: [
   { id: 1, role: 'user', content: '什么是观看距离？', createdAt: 'now' },
   { id: 2, role: 'assistant', content: '可以从主体在画面中的尺度谈起。', createdAt: 'now' },
  ] };
  mount(fixture({}, { tasks: [reply], drafts: [] })); await ready();
  expect(screen.getByText('什么是观看距离？')).toBeVisible();
  expect(screen.getByText('可以从主体在画面中的尺度谈起。')).toBeVisible();
  expect(screen.queryByText('内部完成状态')).not.toBeInTheDocument();
  expect(screen.queryByRole('tablist')).not.toBeInTheDocument();
 });
 it('视觉证据可展开并定位对应照片，元数据没有证据卡', async () => {
  const reply = { ...task('completed'), conversation: true, inspectedPhotos: [{ photo_id: 'a', visible_facts: ['画面左侧有深色主体'], interpretation: '可能形成停顿', uncertainty: '预览无法辨认远处细节' }] };
  mount(fixture({}, { tasks: [reply], drafts: [] })); await ready();
  await userEvent.click(screen.getByText('已查看 1 张照片'));
  expect(screen.getByText('画面左侧有深色主体')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: 'a' }));
  expect(useUIStore.getState().activeId).toBe('a');
 });
 it('输入法确认不会意外发送', async () => {
  const repo = fixture(); mount(repo); await ready();
  fireEvent.keyDown(screen.getByRole('textbox', { name: '消息' }), { key: 'Enter', isComposing: true });
  expect(repo.agent.request).not.toHaveBeenCalled();
 });
});
