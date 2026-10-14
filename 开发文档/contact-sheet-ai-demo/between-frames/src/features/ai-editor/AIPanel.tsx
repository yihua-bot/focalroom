import { useEffect, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Sparkles } from 'lucide-react';
import type { DesktopProjectRepository, Project } from '../../domain/models';
import { defaultAgentBudget, type AgentBudget, type TaskCommand, type TaskState } from '../../domain/agent';
import { Button } from '../../components/shared/Button';
import { DraftReview } from './DraftReview';

export const taskLabels: Record<TaskState, string> = { queued: '排队中', planning: '准备上下文', running: '执行中', needs_input: '等待答复', review_ready: '待审核', completed: '已完成', paused: '已暂停', stale: '原任务基线过期', failed: '失败', cancelled: '已取消', interrupted: '已中断 · 可恢复' };
const budgetFields: { key: keyof AgentBudget; label: string; min: number; max: number }[] = [
 { key: 'maxModelTurns', label: '模型轮次上限', min: 1, max: 200 }, { key: 'maxToolCalls', label: '工具调用上限', min: 1, max: 1000 },
 { key: 'maxDurationSeconds', label: '执行秒数上限', min: 1, max: 7200 }, { key: 'maxContextTokens', label: '单轮文本估算上限', min: 128, max: 256000 },
 { key: 'maxTotalTokens', label: '累计文本估算上限', min: 128, max: 2000000 }, { key: 'maxOutputTokens', label: '输出估算上限', min: 128, max: 16000 },
 { key: 'maxCloudImages', label: '云端图像上限（张）', min: 0, max: 100 }, { key: 'maxCostMinorUsd', label: '费用上限（美分）', min: 0, max: 100000 },
];
export function AIPanel({ repository, project, selectedIds, onUpdated }: { repository?: DesktopProjectRepository; project: Project; selectedIds: string[]; onUpdated: (project: Project) => void }) {
 const client = useQueryClient();
 const key = ['agent', project.id];
 const query = useQuery({ queryKey: key, queryFn: () => repository!.agent.query(project.id), enabled: Boolean(repository), refetchInterval: repository ? 800 : false, refetchOnWindowFocus: 'always' });
 const [composerOpen, setComposerOpen] = useState(!query.data?.tasks.length);
 const [goal, setGoal] = useState('给出两版更克制的 25 张作品序列');
 const [count, setCount] = useState(25);
 const [scopeSelection, setScopeSelection] = useState(false);
 const [budget, setBudget] = useState<AgentBudget>({ ...defaultAgentBudget });
 const [budgetEdited, setBudgetEdited] = useState(false);
 const [busy, setBusy] = useState(false);
 const [error, setError] = useState('');
 const [answer, setAnswer] = useState('');
 const [taskId, setTaskId] = useState<string | null>(null);
 const [draftId, setDraftId] = useState<string | null>(null);
 const [consentPrompt, setConsentPrompt] = useState(false);
 const configQuery = useQuery({ queryKey: ['ai-config', project.id], queryFn: () => repository!.agent.aiConfig(project.id), enabled: Boolean(repository), staleTime: 30_000 });
 const config = configQuery.data;
 const providerLabel = config ? (config.ready ? `真实模型 ${config.modelId}${config.apiKeyTail ? ` ···${config.apiKeyTail}` : ''}` : '本地 Mock（未配置 API Key）') : '';
 const visionWanted = budget.maxCloudImages > 0 && Boolean(config?.ready);
 const consentActive = config?.consent && config.consent.mode !== 'deny' && config.consent.quota - config.consent.used > 0 ? config.consent : null;
 useEffect(() => {
  if (!repository) return;
  let stopped = false; let cleanup: (() => void) | undefined;
  void repository.agent.subscribe(project.id, () => { void client.invalidateQueries({ queryKey: ['agent', project.id] }); }).then(fn => { if (stopped) fn(); else cleanup = fn; }).catch(() => { /* 轮询保留权威投影刷新。 */ });
  return () => { stopped = true; cleanup?.(); };
 }, [repository, project.id, client]);
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
  if (!repository || busy) return; setBusy(true); setError('');
  try {
   if (mode === 'deny') { setBudget(previous => ({ ...previous, maxCloudImages: 0 })); setBudgetEdited(true); }
   else await repository.agent.setConsent(project.id, mode, mode === 'task' ? budget.maxCloudImages : 100);
   setConsentPrompt(false);
   if (mode !== 'deny') { void configQuery.refetch(); await request({ type: 'StartTask', goal: goal.trim(), targetCount: count, selectedPhotoIds: scopeSelection ? selectedIds : [], budget, consentId: mode === 'task' ? (await repository.agent.aiConfig(project.id)).consent?.id ?? null : null }); }
  }
  catch (error) { setError(error instanceof Error ? error.message : String(error)); }
  finally { setBusy(false); }
 };
 const startFlow = () => {
  if (visionWanted && !consentActive) { setConsentPrompt(true); return; }
  void request({ type: 'StartTask', goal: goal.trim(), targetCount: count, selectedPhotoIds: scopeSelection ? selectedIds : [], budget, consentId: consentActive?.id ?? null });
 };
 const drafts = query.data?.drafts.filter(draft => !task || draft.taskId === task.id) ?? [];
 const draft = query.data?.drafts.find(item => item.id === draftId);
 return <section className="ai-panel agent-panel"><Sparkles size={27} strokeWidth={1} /><h3>摄影编辑{config ? (config.ready ? ' · 真实模型' : ' · 本地 Mock') : ''}</h3><p className="micro">{providerLabel && `提供方：${providerLabel}。`}验证受控工具闭环。AI 不直接修改正式选片；云端图像仅在明确授权后上传。</p>
  {!repository && <p className="info-card">浏览器为内存 UI 预览。本地 Mock 任务和草案审批请在 Between Frames M2 Review 桌面版运行。</p>}
  <Button size="small" disabled={busy} onClick={() => { if (!composerOpen && task) { setBudget(task.budget); setBudgetEdited(false); } setComposerOpen(open => !open); }}>{composerOpen ? '收起委托与预算' : '编辑委托与预算'}</Button>
  {composerOpen && <form className="agent-form" onSubmit={event => { event.preventDefault(); startFlow(); }}>
   <label>编辑目标<textarea aria-label="Agent 编辑目标" disabled={busy} required maxLength={4000} value={goal} onChange={event => setGoal(event.target.value)} /></label>
   <label>目标照片数<input aria-label="Agent 目标照片数" disabled={busy} type="number" min={1} max={100} required value={count} onChange={event => setCount(Number(event.target.value))} /></label>
   <label className="agent-scope"><input type="checkbox" disabled={busy} checked={scopeSelection} onChange={event => setScopeSelection(event.target.checked)} />限制到当前选区（{selectedIds.length} 张；无选区则为项目）</label>
   <details><summary>任务预算 · 后端执行</summary><div className="agent-budget">{budgetFields.map(field => <label key={field.key}>{field.label}<input type="number" aria-label={field.label} disabled={busy} min={field.min} max={field.max} required value={budget[field.key]} onChange={event => { setBudgetEdited(true); setBudget(previous => ({ ...previous, [field.key]: Number(event.target.value) })); }} /></label>)}</div><p>Mock 费用 $0、云端图像 0；文本为保守估算。恢复时使用上面明确设置的预算。</p></details>
   <Button type="submit" variant="primary" disabled={!repository || busy || running || !project.photos.length}>{visionWanted ? '开始视觉任务' : '开始本地 Mock 任务'}</Button>
   {consentPrompt && <div className="consent-card" role="dialog" aria-label="云端图像授权">
    <strong>云端图像授权</strong>
    <p className="micro">将向 {config?.modelId} 上传至多 {budget.maxCloudImages} 张最长边 1400px 的 JPEG 预览（重编码、不含 EXIF/GPS）；用于项目内视觉观察，服务商数据政策见其条款；观察结果仅保存在本机。拒绝后仍可用全部本地功能，视觉工具将降级为人工元数据对照。</p>
    <p className="micro">费用按保守估算计入预算上限（每张 1 美分 + 文本）；授权可随时撤回。</p>
    <div className="agent-task-actions">
     <Button size="small" disabled={busy} onClick={() => { void grantConsent('task'); }}>仅本次任务（{budget.maxCloudImages} 张）</Button>
     <Button size="small" disabled={busy} onClick={() => { void grantConsent('project'); }}>本项目（可撤回，100 张）</Button>
     <Button size="small" disabled={busy} onClick={() => { void grantConsent('deny'); }}>禁止云端图像</Button>
    </div>
   </div>}
  </form>}
  {(error || query.error) && <p role="alert">{error || query.error?.message}</p>}
  {tasks.length > 1 && <label>任务记录<select aria-label="选择 Agent 任务" value={task?.id ?? ''} onChange={event => { setTaskId(event.target.value); setBudget(tasks.find(t => t.id === event.target.value)?.budget ?? defaultAgentBudget); setBudgetEdited(false); }}>{tasks.map(t => <option key={t.id} value={t.id}>{taskLabels[t.state]} · {t.goal.slice(0, 30)}</option>)}</select></label>}
  {task && <section className="agent-task" aria-label="Agent 任务详情"><strong role="status">{taskLabels[task.state]}</strong><p className="agent-task-goal">目标：{task.goal}</p><p>{task.reason}</p><p className="micro">{task.usage.modelTurns} / {task.budget.maxModelTurns} 轮 · {task.usage.toolCalls} / {task.budget.maxToolCalls} 工具 · {task.usage.inputEstimate + task.usage.outputEstimate} 文本估算 · 图像 {task.usage.cloudImages} / {task.budget.maxCloudImages} · 估算费用 {(task.usage.costMinorUsd / 100).toFixed(2)} 美元 · 预算计时 {(task.elapsedMs / 1000).toFixed(1)} / {task.budget.maxDurationSeconds} 秒</p>
   <div className="agent-task-actions">{['queued','planning','running'].includes(task.state) && <Button disabled={busy} size="small" onClick={() => { void request({ type: 'PauseTask', taskId: task.id }); }}>暂停任务</Button>}
    {['paused','interrupted'].includes(task.state) && <Button disabled={busy || running} size="small" onClick={() => { void request({ type: 'ResumeTask', taskId: task.id, budget: budgetEdited ? budget : task.budget }); }}>{budgetEdited ? '按当前预算恢复' : '按已保存预算恢复'}（{(budgetEdited ? budget : task.budget).maxModelTurns} 轮 / {(budgetEdited ? budget : task.budget).maxToolCalls} 工具）</Button>}
    {!['completed','failed','cancelled'].includes(task.state) && <Button disabled={busy} size="small" onClick={() => { void request({ type: 'CancelTask', taskId: task.id }); }}>取消任务</Button>}
    {['review_ready','stale'].includes(task.state) && <Button disabled={busy} size="small" onClick={() => { void request({ type: 'CloseTask', taskId: task.id }); }}>结束审核</Button>}
   </div>
   {task.state === 'needs_input' && <form onSubmit={event => { event.preventDefault(); void request({ type: 'SubmitFeedback', taskId: task.id, answer: answer.trim() }); }}><label>{task.question}<input aria-label="人工答复" required maxLength={2000} value={answer} onChange={event => setAnswer(event.target.value)} /></label><Button type="submit" disabled={busy}>提交答复</Button></form>}
   <details open><summary>已执行步骤（{task.steps.length}）</summary><ol className="agent-steps">{task.steps.slice(-12).map(step => <li key={step.id}>{step.summary}</li>)}</ol></details>
  </section>}
  <div className="agent-drafts">{drafts.map(draft => <article key={draft.id}><strong>{draft.title}</strong><p>{draft.items.length} 项 · {draft.state === 'applied' ? '已采纳' : draft.state === 'discarded' ? '已丢弃' : draft.stale ? '基线过期' : '待审核'}</p><Button size="small" disabled={task && ['queued','planning','running'].includes(task.state)} onClick={() => setDraftId(draft.id)}>查看草案差异</Button></article>)}</div>
  {draft && repository && <DraftReview key={draft.id + ':' + draft.revision} draft={draft} project={project} repository={repository} onClose={() => setDraftId(null)} onUpdated={updated => { onUpdated(updated); void query.refetch(); }} />}
 </section>;
}
