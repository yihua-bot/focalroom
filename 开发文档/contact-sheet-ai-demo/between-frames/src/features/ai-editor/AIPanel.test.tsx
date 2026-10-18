import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi, type Mock } from 'vitest';
import { AIPanel } from './AIPanel';
import type { AiConfig, AgentProjection, TaskCommand } from '../../domain/agent';
import type { Project } from '../../domain/models';

const project = { id: 'p1', title: 't', brief: '', revision: 3, photos: [{ id: 'a', status: 'undecided' }], sequence: [], events: [] } as unknown as Project;
const readyConfig: AiConfig = { observation: null, consentProvider: 'openai@' + 'a'.repeat(64), providerId: 'openai', modelId: 'gpt-test', ready: true, apiKeyTail: 'abcd', keySource: 'keychain', baseUrl: 'https://x', promptVersion: 'm3-openai-6', capabilities: ['vision', 'tool_calling', 'structured_output', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null, consent: null };
const emptyProjection: AgentProjection = { tasks: [], drafts: [] };

function makeRepository(overrides: Partial<AiConfig> = {}) {
  const calls: { command: TaskCommand; consentMode: string | null }[] = [];
  const repository: Record<string, unknown> = {
    agent: {
      query: vi.fn().mockResolvedValue(emptyProjection),
      request: vi.fn().mockImplementation((_projectId: string, command: TaskCommand) => { calls.push({ command, consentMode: null }); return Promise.resolve('task-1'); }),
      subscribe: vi.fn().mockResolvedValue(() => {}),
      aiConfig: vi.fn().mockResolvedValue({ ...readyConfig, ...overrides }),
      setConsent: vi.fn().mockImplementation((_projectId: string, mode: 'task' | 'project' | 'deny') => {
        calls.push({ command: null as never, consentMode: mode });
        const config = (repository as { agent: { aiConfig: Mock } }).agent.aiConfig;
        config.mockResolvedValue({ ...readyConfig, consent: { id: 9, projectId: 'p1', mode, provider: 'openai@' + 'a'.repeat(64), quota: 4, used: 0, grantedAt: 'now', revokedAt: null } });
        return Promise.resolve({ id: 9, projectId: 'p1', mode, provider: 'openai@' + 'a'.repeat(64), quota: 4, used: 0, grantedAt: 'now', revokedAt: null });
      }),
    },
  };
  return { repository, calls };
}

function mount(repository: unknown) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return { ...render(<QueryClientProvider client={client}><AIPanel repository={repository as never} project={project} selectedIds={['a']} onUpdated={() => {}} /></QueryClientProvider>), client };
}

describe('云端图像授权对话框', () => {
  it('策展无需视觉能力，图像授权明确显示独立观察型号', async () => {
    const { repository } = makeRepository({
      modelId: 'curator-only', capabilities: ['tool_calling', 'instruction_following'],
      observation: { modelId: 'observer-only', capabilities: ['vision', 'structured_output', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null },
    });
    mount(repository);
    const user = userEvent.setup();
    await screen.findByText('curator-only ···abcd');
    await user.click(screen.getByText('模型设置'));
    expect(screen.getByText('选片模型：curator-only · 图像模型：observer-only')).toBeVisible();
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '2');
    await user.click(screen.getByRole('button', { name: '开始视觉任务' }));
    const dialog = await screen.findByRole('dialog', { name: '云端图像授权' });
    expect(dialog).toHaveTextContent('型号：observer-only');
    expect(dialog).toHaveTextContent('将向 observer-only 上传至多 2 张');
    expect(dialog).toHaveTextContent('观察事实会作为上下文交给同一服务的策展模型 curator-only');
    expect(dialog).not.toHaveTextContent('将向 curator-only');
    expect(screen.getByRole('button', { name: '仅本次任务（2 张）' })).toBeEnabled();
  });
  it('观察模型能力不足时禁止视觉授权，但保留零图像编辑与全局禁止', async () => {
    const { repository, calls } = makeRepository({
      observation: { modelId: 'observer-text', capabilities: ['instruction_following'], capabilitySource: 'operator_declaration', configurationError: '观察模型不支持视觉请求' },
    });
    mount(repository);
    const user = userEvent.setup();
    expect(await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' })).toBeEnabled();
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '2');
    expect(await screen.findByRole('alert')).toHaveTextContent('观察模型不支持视觉请求');
    expect(screen.getByRole('button', { name: '开始视觉任务' })).toBeDisabled();
    await user.click(screen.getByText('模型设置'));
    await user.click(screen.getByRole('button', { name: '管理图像授权' }));
    expect(await screen.findByRole('button', { name: '仅本次任务（2 张）' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '禁止云端图像' })).toBeEnabled();
    expect(calls).toEqual([]);
  });
  it('已打开授权卡后能力配置失效仍可禁止图像，禁止继续授予和委托', async () => {
    const { repository, calls } = makeRepository();
    const { client } = mount(repository);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '2');
    await user.click(screen.getByRole('button', { name: '开始视觉任务' }));
    expect(await screen.findByRole('dialog', { name: '云端图像授权' })).toBeInTheDocument();
    await act(async () => { client.setQueryData(['ai-config', project.id], { ...readyConfig, capabilities: ['tool_calling', 'instruction_following'] }); });
    expect(await screen.findByRole('alert')).toHaveTextContent('当前模型未配置图像观察能力');
    expect(screen.getByRole('button', { name: '仅本次任务（2 张）' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '禁止云端图像' })).toBeEnabled();
    await user.click(screen.getByRole('button', { name: '仅本次任务（2 张）' }));
    const consent = (repository as { agent: { setConsent: Mock } }).agent.setConsent;
    expect(consent).not.toHaveBeenCalled();
    expect(calls).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: '禁止云端图像' }));
    await vi.waitFor(() => expect(consent).toHaveBeenCalledWith('p1', 'deny', 0, null));
    expect(screen.getByLabelText('云端图像上限（张）')).toHaveValue(0);
    expect(calls.filter(call => call.command)).toHaveLength(0);
  });
  it('未知能力配置显示错误，点击或直接提交表单都不发起任务和授权', async () => {
    const { repository, calls } = makeRepository({ capabilities: [], capabilitySource: null, configurationError: '当前模型缺少可信能力配置' });
    mount(repository);
    expect(await screen.findByRole('alert')).toHaveTextContent('当前模型缺少可信能力配置');
    const start = screen.getByRole('button', { name: '开始编辑任务（不上传照片）' });
    expect(start).toBeDisabled();
    fireEvent.submit(start.closest('form')!);
    expect(calls).toHaveLength(0);
    expect(screen.queryByRole('dialog', { name: '云端图像授权' })).not.toBeInTheDocument();
  });
  it('文本模型保留无图任务，视觉预算开启时阻止授权，归零后恢复', async () => {
    const { repository, calls } = makeRepository({ capabilities: ['tool_calling', 'instruction_following'] });
    mount(repository);
    const user = userEvent.setup();
    const start = await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
    expect(start).toBeEnabled();
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '2');
    expect(await screen.findByRole('alert')).toHaveTextContent('当前模型未配置图像观察能力');
    expect(screen.getByRole('button', { name: '开始视觉任务' })).toBeDisabled();
    fireEvent.submit(start.closest('form')!);
    expect(calls).toHaveLength(0);
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '0');
    await user.click(screen.getByRole('button', { name: '开始编辑任务（不上传照片）' }));
    await vi.waitFor(() => expect(calls.filter(call => call.command)).toHaveLength(1));
    expect(calls[0].command).toMatchObject({ type: 'StartTask', budget: { maxCloudImages: 0 } });
    expect((repository as { agent: { setConsent: Mock } }).agent.setConsent).not.toHaveBeenCalled();
  });
  it('配置查询失败时显示失败并阻止委托', async () => {
    const { repository, calls } = makeRepository();
    (repository as { agent: { aiConfig: Mock } }).agent.aiConfig.mockRejectedValue(new Error('invalid response'));
    mount(repository);
    expect(await screen.findByRole('alert')).toHaveTextContent('模型配置读取失败');
    expect(screen.getByRole('button', { name: '运行示例任务' })).toBeDisabled();
    expect(calls).toHaveLength(0);
  });
  it('视觉预算开启且无授权时，先弹授权卡，不直接发任务', async () => {
    const { repository, calls } = makeRepository();
    mount(repository as never);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
    await user.click(await screen.findByRole('button', { name: '开始视觉任务' }));
    expect(await screen.findByRole('dialog', { name: '云端图像授权' })).toBeInTheDocument();
    expect(calls.filter(call => call.command)).toHaveLength(0);
  });
  it('选择仅本次任务后按配额授权并携带 consentId 发起任务', async () => {
    const granted = { id: 9, projectId: 'p1', mode: 'task' as const, provider: 'openai@' + 'a'.repeat(64), quota: 4, used: 0, grantedAt: 'now', revokedAt: null };
    const { repository, calls } = makeRepository();
    const agent = (repository as { agent: { aiConfig: Mock } }).agent;
    // 首次（面板加载）无授权；授权动作后再次查询返回已授权。
    agent.aiConfig.mockResolvedValueOnce({ ...readyConfig, consent: null });
    void granted;
    mount(repository as never);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
    await user.click(await screen.findByRole('button', { name: '开始视觉任务' }));
    await user.click(await screen.findByRole('button', { name: '仅本次任务（4 张）' }));
    const setConsentMock = (repository as { agent: { setConsent: Mock } }).agent.setConsent;
    await vi.waitFor(() => expect(setConsentMock).toHaveBeenCalledWith('p1', 'task', 4, 'openai@' + 'a'.repeat(64)));
    await vi.waitFor(() => expect(calls.filter(call => call.command)).toHaveLength(1));
    expect(calls.find(call => call.command)?.command).toMatchObject({ type: 'StartTask', consentId: 9 });
  });
  it('禁止云端图像会把预算归零并回到本地任务', async () => {
    const { repository } = makeRepository();
    mount(repository as never);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
    await user.click(await screen.findByRole('button', { name: '开始视觉任务' }));
    await user.click(await screen.findByRole('button', { name: '禁止云端图像' }));
    expect((screen.getByLabelText('云端图像上限（张）') as HTMLInputElement).value).toBe('0');
    expect(screen.queryByRole('dialog', { name: '云端图像授权' })).not.toBeInTheDocument();
    expect((repository as { agent: { setConsent: Mock } }).agent.setConsent).toHaveBeenCalledWith('p1', 'deny', 0, null);
  });
  it('旧的单次任务授权不能跳过下一次任务的确认', async () => {
    const { repository, calls } = makeRepository({ consent: { id: 9, projectId: 'p1', mode: 'task', provider: 'openai@' + 'a'.repeat(64), quota: 4, used: 1, grantedAt: 'now', revokedAt: null } });
    mount(repository);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
    await user.clear(screen.getByLabelText('云端图像上限（张）'));
    await user.type(screen.getByLabelText('云端图像上限（张）'), '2');
    await user.click(screen.getByRole('button', { name: '开始视觉任务' }));
    expect(await screen.findByRole('dialog', { name: '云端图像授权' })).toBeInTheDocument();
    expect(calls.filter(call => call.command)).toHaveLength(0);
  });
});

it('授权界面显示真实服务与型号，项目授权也携带明确回执 ID', async () => {
 const { repository, calls } = makeRepository();
 const user = userEvent.setup();
 mount(repository);
 await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
 await user.clear(screen.getByLabelText('云端图像上限（张）'));
 await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
 await user.click(screen.getByRole('button', { name: '开始视觉任务' }));
 expect(screen.getByText(/服务地址：https:\/\/x · 型号：gpt-test/)).toBeInTheDocument();
 expect(screen.getByText(/不授权自动转发给其他服务/)).toBeInTheDocument();
 await user.click(screen.getByRole('button', { name: '本项目（可撤回，100 张）' }));
 await vi.waitFor(() => expect(calls.find(c => c.command)?.command).toMatchObject({ consentId: 9 }));
});
it('其他收件人或已撤回的项目授权不能跳过明确授权', async () => {
 for (const consent of [
  { provider: 'openai@' + 'b'.repeat(64), revokedAt: null },
  { provider: readyConfig.consentProvider!, revokedAt: 'revoked' },
 ]) {
  const { repository } = makeRepository({ consent: { id: 9, projectId: 'p1', mode: 'project', quota: 100, used: 0, grantedAt: 'now', ...consent } });
  const mounted = mount(repository);
  const user = userEvent.setup();
  await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
  await user.clear(screen.getByLabelText('云端图像上限（张）'));
 await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
  await user.click(screen.getByRole('button', { name: '开始视觉任务' }));
  expect(screen.getByRole('dialog', { name: '云端图像授权' })).toBeInTheDocument();
  expect((repository as { agent: { request: Mock } }).agent.request).not.toHaveBeenCalled();
  mounted.unmount();
 }
});
it('服务目标变化时显示原因并刷新配置，授权失败不开始任务', async () => {
 const { repository } = makeRepository();
 const agent = (repository as { agent: { setConsent: Mock; aiConfig: Mock; request: Mock } }).agent;
 agent.setConsent.mockRejectedValue(Object.assign(new Error('服务目标已变化，请重新授权'), { code: 'CONSENT_TARGET_CHANGED' }));
 mount(repository);
 const user = userEvent.setup();
 await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
 await user.clear(screen.getByLabelText('云端图像上限（张）'));
 await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
 await user.click(screen.getByRole('button', { name: '开始视觉任务' }));
 const reads = agent.aiConfig.mock.calls.length;
 await user.click(screen.getByRole('button', { name: '仅本次任务（4 张）' }));
 expect(await screen.findByRole('alert')).toHaveTextContent('服务目标已变化');
 await vi.waitFor(() => expect(agent.aiConfig.mock.calls.length).toBeGreaterThan(reads));
 expect(agent.request).not.toHaveBeenCalled();
});

it('已有项目授权可主动打开管理界面撤回，离线配置也保留禁止入口', async () => {
 const { repository } = makeRepository({ ready: false, consentProvider: null, configurationError: null });
 const user = userEvent.setup();
 mount(repository);
 await screen.findByRole('button', { name: '运行示例任务' });
 await user.click(screen.getByText('模型设置'));
 await user.click(screen.getByRole('button', { name: '管理图像授权' }));
 expect(screen.getByRole('button', { name: '仅本次任务（0 张）' })).toBeDisabled();
 await user.click(screen.getByRole('button', { name: '禁止云端图像' }));
 const agent = (repository as { agent: { setConsent: Mock; request: Mock } }).agent;
 await vi.waitFor(() => expect(agent.setConsent).toHaveBeenCalledWith('p1', 'deny', 0, null));
 expect(agent.request).not.toHaveBeenCalled();
});

it('从管理入口更新项目授权不会自动创建编辑任务', async () => {
 const { repository } = makeRepository();
 const user = userEvent.setup();
 mount(repository);
 await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
 await user.click(screen.getByText('模型设置'));
 await user.click(screen.getByRole('button', { name: '管理图像授权' }));
 await user.click(screen.getByRole('button', { name: '取消授权操作' }));
 expect(screen.queryByRole('dialog', { name: '云端图像授权' })).not.toBeInTheDocument();
 expect((repository as { agent: { setConsent: Mock } }).agent.setConsent).not.toHaveBeenCalled();
 await user.click(screen.getByRole('button', { name: '管理图像授权' }));
 await user.click(screen.getByRole('button', { name: '本项目（可撤回，100 张）' }));
 const agent = (repository as { agent: { setConsent: Mock; request: Mock } }).agent;
 await vi.waitFor(() => expect(agent.setConsent).toHaveBeenCalledWith('p1', 'project', 100, readyConfig.consentProvider));
 expect(agent.request).not.toHaveBeenCalled();
});

describe('免 Prompt 上下文动作', () => {
 const mountWithQuick = (repository: unknown, quickAction: unknown, onFired?: () => void) => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={client}><AIPanel repository={repository as never} project={project} selectedIds={['a']} onUpdated={() => {}} quickAction={quickAction as never} onQuickActionFired={onFired} /></QueryClientProvider>);
 };
 it('无图配置下快捷动作直接携带所选 photo_id 发起任务', async () => {
  const { repository, calls } = makeRepository({ capabilities: ['tool_calling', 'instruction_following'] });
  const onFired = vi.fn();
  mountWithQuick(repository, { id: 'qa-1', goal: '比较照片 a 与 b 的叙事价值', count: 1, photoIds: ['a', 'b'] }, onFired);
  await vi.waitFor(() => expect(calls.filter(call => call.command).length).toBe(1));
  const command = calls.find(call => call.command)!.command as Extract<TaskCommand, { type: 'StartTask' }>;
  expect(command.type).toBe('StartTask');
  expect(command.goal).toContain('叙事价值');
  expect(command.selectedPhotoIds).toEqual(['a', 'b']);
  expect(command.targetCount).toBe(1);
  expect(onFired).toHaveBeenCalled();
 });
 it('视觉预算开启且未授权时，快捷动作先弹授权，确认后按快捷参数发任务', async () => {
  const { repository, calls } = makeRepository();
  const user = userEvent.setup();
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const initial = <QueryClientProvider client={client}><AIPanel repository={repository as never} project={project} selectedIds={['a']} onUpdated={() => {}} /></QueryClientProvider>;
  const view = render(initial);
  await screen.findByRole('button', { name: '开始编辑任务（不上传照片）' });
  await user.clear(screen.getByLabelText('云端图像上限（张）'));
  await user.type(screen.getByLabelText('云端图像上限（张）'), '4');
  view.rerender(<QueryClientProvider client={client}><AIPanel repository={repository as never} project={project} selectedIds={['a']} onUpdated={() => {}} quickAction={{ id: 'qa-2', goal: '生成两版顺序', count: 4, photoIds: ['a', 'b', 'c', 'd'] }} /></QueryClientProvider>);
  const dialog = await screen.findByRole('dialog', { name: '云端图像授权' });
  expect(dialog).toBeVisible();
  expect(calls.filter(call => call.command)).toHaveLength(0);
  await user.click(screen.getByRole('button', { name: '仅本次任务（4 张）' }));
  await vi.waitFor(() => expect(calls.filter(call => call.command).length).toBe(1));
  const command = calls.find(call => call.command)!.command as Extract<TaskCommand, { type: 'StartTask' }>;
  expect(command.goal).toBe('生成两版顺序');
  expect(command.selectedPhotoIds).toEqual(['a', 'b', 'c', 'd']);
  expect(command.consentId).toBe(9);
 });
});
