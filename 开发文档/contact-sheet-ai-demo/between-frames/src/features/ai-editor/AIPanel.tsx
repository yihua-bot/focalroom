import { DiagnosticsDialog } from '../diagnostics/DiagnosticsDialog';
import { useEffect, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { DesktopProjectRepository, Project } from '../../domain/models';
import { defaultAgentBudget, type AgentBudget, type TaskCommand, type TaskState } from '../../domain/agent';
import { Button } from '../../components/shared/Button';
import { DraftReview } from './DraftReview';

export const taskLabels: Record<TaskState, string> = { queued: '排队中', planning: '准备中', running: '执行中', needs_input: '等待答复', review_ready: '待审核', completed: '已完成', paused: '已暂停', stale: '项目已更新', failed: '失败', cancelled: '已取消', interrupted: '已中断' };
const budgetFields: { key: keyof AgentBudget; label: string; min: number; max: number }[] = [
 { key: 'maxModelTurns', label: '模型轮次上限', min: 1, max: 200 }, { key: 'maxToolCalls', label: '工具调用上限', min: 1, max: 1000 },
 { key: 'maxDurationSeconds', label: '执行秒数上限', min: 1, max: 7200 }, { key: 'maxContextTokens', label: '单轮文本估算上限', min: 128, max: 256000 },
 { key: 'maxTotalTokens', label: '累计文本估算上限', min: 128, max: 2000000 }, { key: 'maxOutputTokens', label: '输出估算上限', min: 128, max: 16000 },
 { key: 'maxCloudImages', label: '云端图像上限（张）', min: 0, max: 100 }, { key: 'maxCostMinorUsd', label: '费用上限（美分）', min: 0, max: 100000 },
];
export interface QuickAgentAction {
  /** 每次触发生成新 id，驱动 effect 一次性发起。 */
  id: string;
  goal: string;
  count: number;
  photoIds: string[];
}

export function AIPanel({ repository, project, selectedIds, onUpdated, initialGoal, initialCount, quickAction, onQuickActionFired }: {
  repository?: DesktopProjectRepository; project: Project; selectedIds: string[]; onUpdated: (project: Project) => void;
  initialGoal?: string; initialCount?: number; quickAction?: QuickAgentAction | null; onQuickActionFired?: () => void;
}) {
 const client = useQueryClient();
 const key = ['agent', project.id];
 const query = useQuery({ queryKey: key, queryFn: () => repository!.agent.query(project.id), enabled: Boolean(repository), refetchInterval: repository ? 800 : false, refetchOnWindowFocus: 'always' });
 const [composerOpen, setComposerOpen] = useState(!query.data?.tasks.length);
 const [goal, setGoal] = useState(initialGoal ?? '给出两版更克制的 25 张作品序列');
 const [count, setCount] = useState(initialCount ?? 25);
 const [scopeSelection, setScopeSelection] = useState(false);
 const [budget, setBudget] = useState<AgentBudget>({ ...defaultAgentBudget });
 const [budgetEdited, setBudgetEdited] = useState(false);
 const [busy, setBusy] = useState(false);
 const [error, setError] = useState('');
 const [answer, setAnswer] = useState('');
 const [taskId, setTaskId] = useState<string | null>(null);
 const [draftId, setDraftId] = useState<string | null>(null);
 const [consentPrompt, setConsentPrompt] = useState(false);
 const [consentStartsTask, setConsentStartsTask] = useState(false);
 const [pendingStart, setPendingStart] = useState<{ goal: string; count: number; ids: string[] } | null>(null);
 const configQuery = useQuery({ queryKey: ['ai-config', project.id], queryFn: () => repository!.agent.aiConfig(project.id), enabled: Boolean(repository), staleTime: 10_000, refetchInterval: 15_000 });
 const config = configQuery.data;
 const hosted = config?.keySource === 'focalroom';
 const account = config?.hostedAccount;
 const hostedBlocked = hosted && (Boolean(config?.hostedError) || !account?.enabled || !account.verified || account.available <= 0);
 const observation = config?.observation;
 const observationModel = observation?.modelId ?? config?.modelId;
 const providerLabel = config ? (config.ready ? `${hosted ? 'FocalRoom AI · DeepSeek' : config.modelId}${config.apiKeyTail ? ` ···${config.apiKeyTail}` : ''}` : '演示模式 · 不分析照片') : '';
 const [apiKey, setApiKey] = useState('');
 const [configBusy, setConfigBusy] = useState(false);
 const [configNote, setConfigNote] = useState('');
 const saveKey = async () => {
  if (!repository || configBusy || !apiKey.trim()) return; setConfigBusy(true); setConfigNote('');
  try {
   const { invoke } = await import('@tauri-apps/api/core');
   await invoke('store_api_key', { key: apiKey.trim() });
   setApiKey(''); setConfigNote('已存入系统钥匙串；重启应用后生效。');
   void configQuery.refetch();
  } catch (error) { setConfigNote(error instanceof Error ? error.message : String(error)); }
  finally { setConfigBusy(false); }
 };
 const removeKey = async () => {
  if (!repository || configBusy) return; setConfigBusy(true); setConfigNote('');
  try {
   const { invoke } = await import('@tauri-apps/api/core');
   await invoke('delete_api_key');
   setConfigNote('已从钥匙串移除；重启应用后生效。');
   void configQuery.refetch();
  } catch (error) { setConfigNote(error instanceof Error ? error.message : String(error)); }
  finally { setConfigBusy(false); }
 };
 const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
 const visionWanted = budget.maxCloudImages > 0 && Boolean(config?.ready);
 const visionCapabilities = observation?.capabilities ?? config?.capabilities;
 const visionSupported = ['vision', 'structured_output', 'instruction_following'].every(capability => visionCapabilities?.includes(capability));
 const configurationError = config?.configurationError || (visionWanted && observation?.configurationError) || (visionWanted && !config?.consentProvider ? '服务授权标识缺失，请刷新模型配置。' : '') || (visionWanted && !visionSupported ? '当前模型未配置图像观察能力。请将云端图像上限设为 0，或联系部署方调整配置。' : '');
 const configurationBlocked = hostedBlocked || Boolean(configurationError) || !config || configQuery.isError;
 const consentGrantBlocked = configurationBlocked || !config?.ready || !config.consentProvider || !visionSupported || Boolean(observation?.configurationError);
 const consentActive = config?.consent?.mode === 'project' && config.consent.provider === config.consentProvider && config.consent.projectId === project.id && !config.consent.revokedAt && config.consent.quota - config.consent.used > 0 ? config.consent : null;
 useEffect(() => {
  if (!repository) return;
  let stopped = false; let cleanup: (() => void) | undefined;
  void repository.agent.subscribe(project.id, () => { void client.invalidateQueries({ queryKey: ['agent', project.id] }); }).then(fn => { if (stopped) fn(); else cleanup = fn; }).catch(() => { /* 轮询保留权威投影刷新。 */ });
  return () => { stopped = true; cleanup?.(); };
 }, [repository, project.id, client]);
 useEffect(() => {
  if (!hosted) return;
  let stopped = false; let cleanup: (() => void) | undefined;
  void import('@tauri-apps/api/event').then(({listen}) => listen('cloud-login-updated', () => { void client.resetQueries({queryKey:['ai-config',project.id]}); })).then(fn => { if (stopped) fn(); else cleanup = fn; }).catch(() => { /* 定时刷新保留。 */ });
  return () => { stopped = true; cleanup?.(); };
 }, [hosted,client,project.id]);
 const tasks = query.data?.tasks ?? [];
 const task = tasks.find(item => item.id === taskId) ?? tasks[0];
 const running = tasks.some(t => ['queued','planning','running'].includes(t.state));
 const request = async (command: TaskCommand) => {
  if (!repository || busy) return; setBusy(true); setError('');
  try { const id = await repository.agent.request(project.id, command); setTaskId(id); const current = await query.refetch(); const saved = current.data?.tasks.find(task => task.id === id); if (saved) setBudget(saved.budget); setComposerOpen(false); setBudgetEdited(false); }
  catch (error) { setError(error instanceof Error ? error.message : String(error)); }
  finally { setBusy(false); }
 };
 const grantConsent = async (mode: 'task' | 'project' | 'deny') => {
  if (!repository || busy || (mode !== 'deny' && consentGrantBlocked)) return; setBusy(true); setError('');
  try {
   if (mode === 'deny') { setBudget(previous => ({ ...previous, maxCloudImages: 0 })); setBudgetEdited(true); }
   const granted = await repository.agent.setConsent(project.id, mode, mode === 'task' ? budget.maxCloudImages : mode === 'project' ? 100 : 0, mode === 'deny' ? null : config?.consentProvider);
   setConsentPrompt(false);
   setConsentStartsTask(false);
   void configQuery.refetch();
   if (mode !== 'deny' && consentStartsTask && pendingStart) { const payload = pendingStart; setPendingStart(null); await request({ type: 'StartTask', goal: payload.goal, targetCount: payload.count, selectedPhotoIds: payload.ids, budget, consentId: granted.id }); }
   else setPendingStart(null);
  }
  catch (error) {
   setError(error instanceof Error ? error.message : String(error));
   if (typeof error === 'object' && error && 'code' in error && error.code === 'CONSENT_TARGET_CHANGED') void configQuery.refetch();
  }
  finally { setBusy(false); }
 };
 const fireStart = (payload: { goal: string; count: number; ids: string[] }, consentId: number | string | null) =>
   request({ type: 'StartTask', goal: payload.goal, targetCount: payload.count, selectedPhotoIds: payload.ids, budget, consentId: consentId === null ? null : Number(consentId) });
 const startFlow = () => {
  if (configurationBlocked) return;
  const payload = { goal: goal.trim(), count, ids: scopeSelection ? selectedIds : [] };
  if (visionWanted && !consentActive) { setPendingStart(payload); setConsentStartsTask(true); setConsentPrompt(true); return; }
  void fireStart(payload, consentActive?.id ?? null);
 };
 // 免 Prompt 上下文动作：选区按钮直接发起并自动携带所选 photo_id；云端图像授权仍需显式确认。
 // 外部命令式启动（与滚动定位同类）：一次性消费 quickAction 打开授权卡，无级联渲染链。
 useEffect(() => {
  if (!repository || !quickAction || busy || running || configurationBlocked) return;
  const payload = { goal: quickAction.goal, count: quickAction.count, ids: quickAction.photoIds };
  // eslint-disable-next-line react-hooks/set-state-in-effect -- 一次性消费外部命令输入。
  if (visionWanted && !consentActive) { setPendingStart(payload); setConsentStartsTask(true); setConsentPrompt(true); }
  else void fireStart(payload, consentActive?.id ?? null);
  onQuickActionFired?.();
  // 依赖含 configurationBlocked：模型配置异步加载完成后补触发一次；触发后父层清空 quickAction 防重复。
  // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [quickAction?.id, configurationBlocked]);
 const drafts = query.data?.drafts.filter(draft => !task || draft.taskId === task.id) ?? [];
 const draft = query.data?.drafts.find(item => item.id === draftId);
 return <section className="ai-panel agent-panel"><h3>编辑建议</h3>{providerLabel && <p className="micro">{providerLabel}</p>}
  
  {!repository && <p className="info-card">请在桌面版使用编辑建议。</p>}
  {configurationError && <p role="alert">{configurationError}</p>}
  {configQuery.isError && <p role="alert">模型配置读取失败，请刷新后重试。</p>}
  {hosted && <div className="ai-account" aria-label="FocalRoom AI 额度">
   {config?.hostedError ? <p role="alert">{config.hostedError}</p> : !account ? <p>正在读取 AI 额度…</p> : <>
    <p>可用 {account.available.toLocaleString()} tokens{account.reserved > 0 ? ` · 预留 ${account.reserved.toLocaleString()}` : ''}</p>
    {!account.enabled && <p>FocalRoom AI 暂未开放。</p>}
    {!account.verified && <p>请先验证账户邮箱。</p>}
    {account.enabled && account.verified && account.available === 0 && <p>AI 额度已用完，请联系 FocalRoom。</p>}
   </>}
   <Button size="small" disabled={configQuery.isFetching} onClick={() => { void configQuery.refetch(); }}>刷新额度</Button>
  </div>}
  {repository && <details className="ai-config"><summary>{hosted ? 'AI 设置' : '模型设置'}</summary>
   {!hosted && <p className="micro">密钥保存在系统钥匙串。</p>}
   {config?.ready && observation && observation.modelId !== config.modelId && <p className="micro">选片模型：{config.modelId} · 图像模型：{observation.modelId}</p>}
   <div className="agent-task-actions">
    {!hosted && <><input type="password" aria-label="API Key（存入钥匙串）" placeholder="sk-..." disabled={configBusy} value={apiKey} maxLength={4096} onChange={event => setApiKey(event.target.value)} />
    <Button size="small" disabled={configBusy || !apiKey.trim()} onClick={() => { void saveKey(); }}>存入钥匙串</Button>
    {config?.keySource === 'keychain' && <Button size="small" disabled={configBusy} onClick={() => { void removeKey(); }}>移除钥匙串密钥</Button>}
    </>}<Button size="small" disabled={busy} onClick={() => { setComposerOpen(true); setConsentStartsTask(false); setConsentPrompt(true); }}>管理图像授权</Button>
    <Button size="small" onClick={() => setDiagnosticsOpen(true)}>查看本地存储与诊断</Button>
   </div>
   {configNote && <p className="micro" role="status">{configNote}</p>}
  </details>}
  <Button size="small" disabled={busy} onClick={() => { if (!composerOpen && task) { setBudget(task.budget); setBudgetEdited(false); } setComposerOpen(open => !open); }}>{composerOpen ? '收起任务设置' : '任务设置'}</Button>
  {composerOpen && <form className="agent-form" onSubmit={event => { event.preventDefault(); startFlow(); }}>
   <label>编辑目标<textarea aria-label="Agent 编辑目标" disabled={busy} required maxLength={4000} value={goal} onChange={event => setGoal(event.target.value)} /></label>
   <label>目标照片数<input aria-label="Agent 目标照片数" disabled={busy} type="number" min={1} max={100} required value={count} onChange={event => setCount(Number(event.target.value))} /></label>
   <label className="agent-scope"><input type="checkbox" disabled={busy} checked={scopeSelection} onChange={event => setScopeSelection(event.target.checked)} />限制到当前选区（{selectedIds.length} 张；无选区则为项目）</label>
   <details><summary>任务预算</summary><div className="agent-budget">{budgetFields.map(field => <label key={field.key}>{field.label}<input type="number" aria-label={field.label} disabled={busy} min={field.min} max={field.max} required value={budget[field.key]} onChange={event => { setBudgetEdited(true); setBudget(previous => ({ ...previous, [field.key]: Number(event.target.value) })); }} /></label>)}</div><p className="micro">费用为估算值。</p></details>
   <Button type="submit" variant="primary" disabled={!repository || configurationBlocked || busy || running || !project.photos.length}>{visionWanted ? '开始视觉任务' : config?.ready ? '开始编辑任务（不上传照片）' : '运行示例任务'}</Button>
   {consentPrompt && <div className="info-card consent-card" role="dialog" aria-label="云端图像授权">
    <strong>云端图像授权</strong>
    <p className="micro" style={{ overflowWrap: 'anywhere' }}>服务地址：{config?.ready ? config.baseUrl : '未配置'} · 型号：{config?.ready ? observationModel : '无'}</p>
    <p className="micro">仅授权此服务与型号；更换后需重新授权，不授权自动转发给其他服务。「禁止云端图像」会撤回本项目所有服务的图像授权。</p>
    {config?.ready
     ? <p className="micro">将向 {hosted ? 'FocalRoom 与 DeepSeek' : observationModel} 上传至多 {budget.maxCloudImages} 张最长边 1400px 的 JPEG 预览（重编码、不含 EXIF/GPS）；观察事实会作为上下文交给同一服务的策展模型 {config.modelId}。应用将缓存与草案保存在本机，服务端的数据处理依其条款。拒绝后仍可用全部本地功能，视觉工具将降级为人工元数据对照。</p>
     : <p className="micro">当前未配置云端图像服务。你仍可禁止本项目向所有服务上传图像；本地选片功能照常可用。</p>}
    <p className="micro">费用按当前部署的预算费率估算计入任务上限，不代表服务商账单；授权可随时撤回。</p>
    <div className="agent-task-actions">
     <Button size="small" disabled={busy || consentGrantBlocked} onClick={() => { void grantConsent('task'); }}>仅本次任务（{budget.maxCloudImages} 张）</Button>
     <Button size="small" disabled={busy || consentGrantBlocked} onClick={() => { void grantConsent('project'); }}>本项目（可撤回，100 张）</Button>
     <Button size="small" disabled={busy} onClick={() => { void grantConsent('deny'); }}>禁止云端图像</Button>
     <Button size="small" disabled={busy} onClick={() => { setConsentPrompt(false); setConsentStartsTask(false); }}>取消授权操作</Button>
    </div>
   </div>}
  </form>}
  {(error || query.error) && <p role="alert">{error || query.error?.message}</p>}
  {tasks.length > 1 && <label>任务记录<select aria-label="选择 Agent 任务" value={task?.id ?? ''} onChange={event => { setTaskId(event.target.value); setBudget(tasks.find(t => t.id === event.target.value)?.budget ?? defaultAgentBudget); setBudgetEdited(false); }}>{tasks.map(t => <option key={t.id} value={t.id}>{taskLabels[t.state]} · {t.goal.slice(0, 30)}</option>)}</select></label>}
  {task && <section className="agent-task" aria-label="Agent 任务详情"><strong role="status">{taskLabels[task.state]}</strong><p className="agent-task-goal">目标：{task.goal}</p>{['failed', 'paused', 'interrupted', 'stale'].includes(task.state) && <p>{task.reason}</p>}<details><summary>用量详情</summary><p>{task.reason}</p><p className="micro">{task.usage.modelTurns} / {task.budget.maxModelTurns} 轮 · {task.usage.toolCalls} / {task.budget.maxToolCalls} 工具 · {task.usage.inputEstimate + task.usage.outputEstimate} 文本估算 · 图像 {task.usage.cloudImages} / {task.budget.maxCloudImages} · 估算费用 {(task.usage.costMinorUsd / 100).toFixed(2)} 美元 · 预算计时 {(task.elapsedMs / 1000).toFixed(1)} / {task.budget.maxDurationSeconds} 秒</p></details>
   <div className="agent-task-actions">{['queued','planning','running'].includes(task.state) && <Button disabled={busy} size="small" onClick={() => { void request({ type: 'PauseTask', taskId: task.id }); }}>暂停任务</Button>}
    {['paused','interrupted'].includes(task.state) && <Button disabled={busy || running} size="small" onClick={() => { void request({ type: 'ResumeTask', taskId: task.id, budget: budgetEdited ? budget : task.budget }); }}>{budgetEdited ? '按当前预算恢复' : '按已保存预算恢复'}（{(budgetEdited ? budget : task.budget).maxModelTurns} 轮 / {(budgetEdited ? budget : task.budget).maxToolCalls} 工具）</Button>}
    {!['completed','failed','cancelled'].includes(task.state) && <Button disabled={busy} size="small" onClick={() => { void request({ type: 'CancelTask', taskId: task.id }); }}>取消任务</Button>}
    {['review_ready','stale'].includes(task.state) && <Button disabled={busy} size="small" onClick={() => { void request({ type: 'CloseTask', taskId: task.id }); }}>结束审核</Button>}
   </div>
   {task.state === 'needs_input' && <form onSubmit={event => { event.preventDefault(); void request({ type: 'SubmitFeedback', taskId: task.id, answer: answer.trim() }); }}><label>{task.question}<input aria-label="人工答复" required maxLength={2000} value={answer} onChange={event => setAnswer(event.target.value)} /></label><Button type="submit" disabled={busy}>提交答复</Button></form>}
   <details><summary>已执行步骤（{task.steps.length}）</summary><ol className="agent-steps">{task.steps.slice(-12).map(step => <li key={step.id}>{step.summary}</li>)}</ol></details>
  </section>}
  <div className="agent-drafts">{drafts.map(draft => <article key={draft.id}><strong>{draft.title}</strong><p>{draft.items.length} 项 · {draft.state === 'applied' ? '已采纳' : draft.state === 'discarded' ? '已丢弃' : draft.stale ? '需要重新比较' : '待审核'}</p><Button size="small" disabled={task && ['queued','planning','running'].includes(task.state)} onClick={() => setDraftId(draft.id)}>查看草案差异</Button></article>)}</div>
  {diagnosticsOpen && repository && <DiagnosticsDialog repository={repository.diagnostics} onClose={() => setDiagnosticsOpen(false)} />}
  {draft && repository && <DraftReview key={draft.id + ':' + draft.revision} draft={draft} project={project} repository={repository} onClose={() => setDraftId(null)} onUpdated={updated => { onUpdated(updated); void query.refetch(); }} />}
 </section>;
}
