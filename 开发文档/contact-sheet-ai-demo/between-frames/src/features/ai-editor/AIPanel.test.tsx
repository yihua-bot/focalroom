import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi, type Mock } from 'vitest';
import { AIPanel } from './AIPanel';
import type { AiConfig, AgentProjection, TaskCommand } from '../../domain/agent';
import type { Project } from '../../domain/models';

const project = { id: 'p1', title: 't', brief: '', revision: 3, photos: [{ id: 'a', status: 'undecided' }], sequence: [], events: [] } as unknown as Project;
const readyConfig: AiConfig = { providerId: 'openai', modelId: 'gpt-test', ready: true, apiKeyTail: 'abcd', keySource: 'keychain', baseUrl: 'https://x', promptVersion: 'm3-openai-6', capabilities: ['vision', 'tool_calling', 'structured_output', 'instruction_following'], capabilitySource: 'operator_declaration', configurationError: null, consent: null };
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
        config.mockResolvedValue({ ...readyConfig, consent: { id: 9, projectId: 'p1', mode, provider: 'openai', quota: 4, used: 0, grantedAt: 'now', revokedAt: null } });
        return Promise.resolve({ id: 9, projectId: 'p1', mode, provider: 'openai', quota: 4, used: 0, grantedAt: 'now', revokedAt: null });
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
    await vi.waitFor(() => expect(consent).toHaveBeenCalledWith('p1', 'deny', 0));
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
    expect(screen.getByRole('button', { name: '开始本地 Mock 任务' })).toBeDisabled();
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
    const granted = { id: 9, projectId: 'p1', mode: 'task' as const, provider: 'openai', quota: 4, used: 0, grantedAt: 'now', revokedAt: null };
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
    await vi.waitFor(() => expect(setConsentMock).toHaveBeenCalledWith('p1', 'task', 4));
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
    expect((repository as { agent: { setConsent: Mock } }).agent.setConsent).toHaveBeenCalledWith('p1', 'deny', 0);
  });
  it('旧的单次任务授权不能跳过下一次任务的确认', async () => {
    const { repository, calls } = makeRepository({ consent: { id: 9, projectId: 'p1', mode: 'task', provider: 'openai', quota: 4, used: 1, grantedAt: 'now', revokedAt: null } });
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
