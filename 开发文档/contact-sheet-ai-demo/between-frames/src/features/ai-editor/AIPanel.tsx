import { userFacingError } from '../../services/userFacingError';
import { useEffect, useRef, useState, type ReactNode, type ComponentProps } from 'react';
import { ArrowUp, Check, Image, Sparkles } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useUIStore } from '../../stores/uiStore';
import { zh } from '../../app/copy';
import type { DesktopProjectRepository, Project } from '../../domain/models';
import { defaultAgentBudget, type AiConfig, type AgentTask, type TaskCommand, type TaskState, type DraftReference } from '../../domain/agent';
import { budgetMessage, resumeBudget } from './budgetRecovery';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { DraftReview } from './DraftReview';
import { TaskProgress } from './TaskProgress';
import { useAccount } from '../cloud-login/AccountProvider';
import { PreferenceProposalCard } from './PreferenceProposalCard';

export const taskLabels: Record<TaskState, string> = { queued: '排队中', planning: '准备中', running: '执行中', needs_input: '等待答复', review_ready: '待审核', completed: '已完成', paused: '已暂停', stale: '项目已更新', failed: '失败', cancelled: '已取消', interrupted: '已中断' };
export interface QuickAgentAction { id: string; goal: string; count: number; photoIds: string[] }
const supportsVision = (config?: AiConfig) => ['vision', 'structured_output', 'instruction_following'].every(capability => (config?.observation?.capabilities ?? config?.capabilities)?.includes(capability));
function unavailable(config?: AiConfig): string {
 if (!config?.ready) return zh.aiNotReady;
 if (config.configurationError || config.observation?.configurationError) return zh.aiNotReady;
 if (config.keySource !== 'focalroom') return '';
 if (config.hostedError) return zh.aiConnectionFailed;
 if (!config.hostedAccount?.enabled) return zh.aiClosed;
 if (!config.hostedAccount.verified) return zh.aiVerify;
 return config.hostedAccount.available > 0 ? '' : zh.aiQuota;
}

function AIPanelContent({ repository, project, selectedIds, onUpdated, initialGoal, initialCount, quickAction, onQuickActionFired, settingsOpen = false, onSettingsClose = () => {}, children }: {
 repository?: DesktopProjectRepository; project: Project; selectedIds: string[]; onUpdated: (project: Project) => void;
 initialGoal?: string; initialCount?: number; quickAction?: QuickAgentAction | null; onQuickActionFired?: () => void;
 settingsOpen?: boolean; onSettingsClose?: () => void; children?: ReactNode;
}) {
 const client = useQueryClient();
 const account = useAccount();
 const canvasSource = useUIStore(state => state.canvasTaskSource);
 const openCanvasBoard = useUIStore(state => state.openCanvasBoard);
 const selectPhoto = useUIStore(state => state.selectRange);
 const accountId = account?.status?.user?.id ?? 'local';
 const query = useQuery({ queryKey: ['agent', project.id, accountId], queryFn: () => repository!.agent.query(project.id), enabled: Boolean(repository), refetchInterval: repository ? 1000 : false, refetchIntervalInBackground: true, refetchOnWindowFocus: 'always' });
 const configQuery = useQuery({ queryKey: ['ai-config', project.id], queryFn: () => repository!.agent.aiConfig(project.id), enabled: Boolean(repository), staleTime: 10_000, refetchInterval: 15_000 });
 const config = configQuery.data;
 const saved = useUIStore(state => state.aiDrafts[project.id]);
 const saveDraft = useUIStore(state => state.saveAIDraft);
 const [message, setMessage] = useState(saved?.goal ?? initialGoal ?? '');
 const [count, setCount] = useState(saved?.count ?? initialCount ?? 20);
 const [scopeSelection, setScopeSelection] = useState(saved?.scopeSelection ?? false);
 const [imageLimit, setImageLimit] = useState(100);
 const [duration, setDuration] = useState(defaultAgentBudget.maxDurationSeconds);
 const [busy, setBusy] = useState(false);
 const sending = useRef(false);
 const [error, setError] = useState('');
 const [draftId, setDraftId] = useState<string | null>(null);
 const [continuation, setContinuation] = useState<{ reference: DraftReference; title: string } | null>(null);
 const last = useRef<HTMLDivElement>(null);
 const composer = useRef<HTMLTextAreaElement>(null);
 const consumedAction = useRef<string | null>(null);
 const contextKey = `${project.id}:${accountId}`;
 const currentContext = useRef(contextKey);
 useEffect(() => {
  currentContext.current = contextKey;
  return () => { currentContext.current = ''; };
 }, [contextKey]);
 const tasks = query.data?.tasks ?? [];
 const task = tasks[0];
 const running = tasks.some(item => ['queued', 'planning', 'running'].includes(item.state));
 const blocked = !repository || configQuery.isError || !!unavailable(config);
 const vision = supportsVision(config);
 const selectionEmpty = scopeSelection && !selectedIds.length;
 const updateMessage = (goal: string) => {
  const explicit = goal.match(/(\d+)\s*(?:张|枚|장|photos?\b|images?\b|frames?\b)/i) ?? goal.match(/(?:select|keep|choose)\s+(\d+)/i);
  const nextCount = explicit ? Math.min(500, Math.max(1, Number(explicit[1]))) : count;
  setMessage(goal); setCount(nextCount);
  saveDraft(project.id, { goal, count: nextCount, scopeSelection });
 };

 useEffect(() => {
  if (!repository) return;
  let stopped = false; let cleanup: (() => void) | undefined;
  void repository.agent.subscribe(project.id, () => { void client.invalidateQueries({ queryKey: ['agent', project.id] }); }).then(fn => { if (stopped) fn(); else cleanup = fn; }).catch(() => {});
  return () => { stopped = true; cleanup?.(); };
 }, [repository, project.id, client]);
 useEffect(() => {
  if (config?.keySource !== 'focalroom') return;
  let stopped = false; let cleanup: (() => void) | undefined;
  void import('@tauri-apps/api/event').then(({ listen }) => listen('cloud-login-updated', () => { void client.removeQueries({ queryKey: ['agent', project.id] }); void client.resetQueries({ queryKey: ['ai-config', project.id] }); })).then(fn => { if (stopped) fn(); else cleanup = fn; }).catch(() => {});
  return () => { stopped = true; cleanup?.(); };
 }, [config?.keySource, client, project.id]);
 useEffect(() => { last.current?.scrollIntoView?.({ block: 'end' }); }, [tasks.length, task?.state]);
 useEffect(() => {
  if (!quickAction || consumedAction.current === quickAction.id) return;
  consumedAction.current = quickAction.id;
  // 外部选区动作只填写消息，不在抽屉打开时自动发送。
  setMessage(quickAction.goal); setCount(quickAction.count); setScopeSelection(true);
  saveDraft(project.id, { goal: quickAction.goal, count: quickAction.count, scopeSelection: true });
  onQuickActionFired?.();
 }, [quickAction, onQuickActionFired, project.id, saveDraft]);

 const request = async (command: TaskCommand) => {
  if (!repository || sending.current) return;
  sending.current = true; setBusy(true); setError('');
  try { await repository.agent.request(project.id, command); await query.refetch(); }
  catch (cause) { setError(userFacingError(cause)); }
  finally { sending.current = false; setBusy(false); }
 };
 const send = async () => {
  if (!repository || sending.current || blocked || running || !message.trim() || selectionEmpty || count < 1 || count > 500) return;
  if (!/[\p{L}\p{N}]/u.test(message)) { setError(zh.chatClarifyMessage); return; }
  sending.current = true; setBusy(true); setError('');
  try {
   const fresh = await configQuery.refetch();
   if (fresh.isError || !fresh.data) throw new Error(zh.aiConnectionFailed);
   if (currentContext.current !== contextKey) return;
   const reason = unavailable(fresh.data);
   if (reason) throw new Error(reason);
   if (!continuation && task?.state === 'needs_input') {
    await repository.agent.request(project.id, { type: 'SubmitFeedback', taskId: task.id, answer: message.trim() });
   } else {
    const ids = scopeSelection ? selectedIds : [];
    const images = supportsVision(fresh.data) && project.photos.length > 0 ? imageLimit : 0;
    const budget = { ...defaultAgentBudget, maxDurationSeconds: duration, maxCloudImages: images };
    let consentId: number | null = null;
    if (images > 0) {
     if (!fresh.data.consentProvider || fresh.data.consentProvider !== config?.consentProvider) throw new Error(zh.aiServiceChanged);
     // 用户发送选片请求即授权本次预览分析；后台仍绑定服务、项目、配额与明确回执。
     consentId = (await repository.agent.setConsent(project.id, 'task', images, fresh.data.consentProvider)).id;
    }
    if (currentContext.current !== contextKey) return;
    await repository.agent.request(project.id, {
     type: 'StartConversation', expectedAccountId: account?.status?.user?.id ?? null, goal: message.trim(), targetCount: count, selectedPhotoIds: ids,
     focusPhotoIds: selectedIds.filter(id => !scopeSelection || ids.includes(id)), budget, consentId, sourceBoard: continuation ? null : canvasSource, continuation: continuation?.reference ?? null,
    });
   }
   if (currentContext.current === contextKey) { updateMessage(''); setContinuation(null); }
   await query.refetch();
  } catch (cause) { if (currentContext.current === contextKey) setError(userFacingError(cause)); }
  finally { sending.current = false; setBusy(false); }
 };
 useEffect(() => { composer.current?.focus(); }, [project.id]);
 const draft = query.data?.drafts.find(item => item.id === draftId);
 const continueDraft = (reference: DraftReference, title: string) => {
  setContinuation({ reference, title });
  if (!message.trim()) updateMessage(zh.chatRevisePrompt);
  composer.current?.focus();
 };
 const memoryDecision = async (taskId: string, proposalId: string, statement?: string) => {
  if (!repository || currentContext.current !== contextKey) return;
  setError('');
  try {
   const expectedAccountId = account?.status?.user?.id ?? null;
   await repository.memory.request(project.id, statement === undefined ? { type: 'DismissPreferenceProposal', taskId, proposalId, expectedAccountId } : { type: 'RememberPreference', taskId, proposalId, statement, expectedAccountId });
   await query.refetch();
   await client.invalidateQueries({ queryKey: ['memory', project.id] });
  } catch (cause) { if (currentContext.current === contextKey) setError(userFacingError(cause)); }
 };
 const prepareRestart = (item: AgentTask) => {
  setMessage(item.goal); setScopeSelection(true);
  saveDraft(project.id, { goal: item.goal, count, scopeSelection: true });
  composer.current?.focus();
 };
 const suggestions = [
  { label: zh.chatSuggestSelect, goal: project.brief || zh.chatGoalSelect },
  { label: zh.chatSuggestCompare, goal: zh.chatGoalCompare },
  { label: zh.chatSuggestSequence, goal: zh.chatGoalSequence },
 ];
 return <section className="ai-chat">
  <div className="ai-chat-feed" role="log" aria-label={zh.chatHistory} aria-live="polite">
   {!tasks.length && <div className="ai-chat-welcome"><span className="chat-welcome-mark"><Sparkles size={27} /></span><h3>{zh.chatWelcome}</h3><p>{zh.chatWelcomeHint}</p>
    <div className="chat-suggestions">{suggestions.map(item => <button key={item.label} onClick={() => updateMessage(item.goal)}>{item.label}<span>↗</span></button>)}</div>
   </div>}
   {[...tasks].slice(0, 10).reverse().map(item => <article key={item.id} className="chat-turn" aria-label={zh.chatTurn}>
    {item.messages?.length ? item.messages.map(turn => <div key={turn.id} className={turn.role === 'user' ? 'chat-user-message' : 'chat-assistant-message'}><p className="focal-message-text">{turn.content}</p></div>) : <div className="chat-user-message"><p>{item.goal}</p></div>}
    {(!item.conversation || item.state !== 'completed' || !!item.inspectedPhotos?.length || !!item.preferenceProposals?.length || !!item.creativeReferences?.length || !!item.boardDrafts?.length || query.data?.drafts.some(draft => draft.taskId === item.id)) && <div className="chat-assistant-message"><span className="chat-assistant-icon"><Sparkles size={15} /></span><div>
     <strong className="chat-task-status" data-state={item.state}>{taskLabels[item.state]}</strong>
     <TaskProgress task={item} />
     {!!item.inspectedPhotos?.length && <details className="chat-inspection"><summary>{zh.chatInspected.replace('{count}', String(item.inspectedPhotos.length))}</summary>
      {item.inspectedPhotos.map(photo => <div key={photo.photo_id}><button type="button" onClick={() => selectPhoto(photo.photo_id, project.photos.map(p => p.id))}>{project.photos.find(p => p.id === photo.photo_id)?.name ?? photo.photo_id}</button>
       <ul>{photo.visible_facts.map((fact, index) => <li key={index}>{fact}</li>)}</ul><p>{photo.interpretation}</p><small>{photo.uncertainty}</small></div>)}
     </details>}
     {!item.messages?.some(turn => turn.role === 'assistant' && turn.content === item.question) && item.question && <p>{item.question}</p>}
     {!item.conversation && ['completed', 'review_ready'].includes(item.state) && <p className="focal-message-text">{item.reason}</p>}
     {['paused', 'interrupted', 'failed', 'stale'].includes(item.state) && <p>{budgetMessage(item) ?? userFacingError(new Error(item.reason))}</p>}
     {item.budgetIssue?.kind === 'images' && <p className="muted">{zh.chatLimitImagesCount.replace('{used}', String(item.budgetIssue.used)).replace('{requested}', String(item.budgetIssue.requested)).replace('{limit}', String(item.budgetIssue.limit))}</p>}
     {item.budgetIssue?.stage === 'vision' && item.budgetIssue.requestNotSent && <p className="muted">{zh.chatPhotosNotSent}</p>}
     {(query.data?.drafts ?? []).filter(candidate => candidate.taskId === item.id).map(candidate => <div className="chat-draft-card" key={candidate.id}>
      <div><Check size={15} /><strong>{candidate.title}</strong></div><p>{candidate.summary}</p>
      <Button size="small" disabled={running} onClick={() => setDraftId(candidate.id)}>{zh.chatReviewDraft}</Button>
      <Button size="small" disabled={busy || running} onClick={() => continueDraft({ kind: 'selection', id: candidate.id, revision: candidate.revision }, candidate.title)}>{zh.chatReviseDraft}</Button>
     </div>)}
     {(item.boardDrafts ?? []).map(board => <div className="chat-draft-card" key={board.id}>
      <div><Check size={15} /><strong>{board.title}</strong></div>
      <Button size="small" disabled={!['review_ready', 'completed'].includes(item.state)} onClick={() => openCanvasBoard(board.id)}>{zh.chatReviewBoard}</Button>
      <Button size="small" disabled={busy || running || ['queued','planning','running','needs_input'].includes(item.state)} onClick={() => continueDraft({ kind: 'board', id: board.id, revision: board.revision ?? 1 }, board.title)}>{zh.chatReviseDraft}</Button>
     </div>)}
     {!!item.creativeReferences?.length && <details className="chat-references"><summary>{zh.chatReferences}</summary>{item.creativeReferences.map((reference, index) => <div key={index}><strong>{reference.kind === 'knowledge' ? reference.label : zh[`chatContext_${reference.section}`]}</strong>{reference.version != null && <small> · {reference.version}</small>}{reference.passage && <blockquote>{reference.passage}</blockquote>}{reference.citationId && <small>{reference.citationId} · {reference.license}</small>}</div>)}</details>}
     {!!item.draftLineage?.length && <details className="chat-lineage"><summary>{zh.chatRevisionHistory}</summary>{item.draftLineage.map(link => <p key={link.childId}>{zh.chatRevisionParent} {link.parent.revision} · {link.reason}</p>)}</details>}
     {(item.preferenceProposals ?? []).map(proposal => <PreferenceProposalCard key={proposal.id} proposal={proposal} disabled={busy} onSave={statement => memoryDecision(item.id, proposal.id, statement)} onDismiss={() => memoryDecision(item.id, proposal.id)} />)}
     {item.id === task?.id && <div className="chat-task-controls">
      {['queued', 'planning', 'running'].includes(item.state) && <Button size="small" disabled={busy} onClick={() => { void request({ type: 'PauseTask', taskId: item.id }); }}>{zh.chatPause}</Button>}
      {['paused', 'interrupted'].includes(item.state) && (resumeBudget(item, duration) ? <Button size="small" disabled={busy || running || blocked} onClick={() => {
       const budget = resumeBudget(item, duration);
       if (budget) void request({ type: 'ResumeTask', taskId: item.id, budget });
      }}>{item.budgetIssue?.kind === 'time' ? zh.chatExtendWait : zh.chatContinue}</Button> : <Button size="small" disabled={busy || running} onClick={() => prepareRestart(item)}>{zh.chatAdjustRange}</Button>)}
      {!['completed', 'failed', 'cancelled'].includes(item.state) && <Button size="small" disabled={busy} onClick={() => { void request({ type: 'CancelTask', taskId: item.id }); }}>{zh.chatCancel}</Button>}
      {['review_ready', 'stale'].includes(item.state) && <Button size="small" disabled={busy} onClick={() => { void request({ type: 'CloseTask', taskId: item.id }); }}>{zh.chatFinishReview}</Button>}
     </div>}
    </div></div>}
   </article>)}
   {(error || query.error) && <p role="alert" className="chat-error">{error || userFacingError(query.error)}</p>}
   {children}
   <div ref={last} />
  </div>
  <footer className="ai-chat-composer">
   {account && !account.status?.authenticated ? <p className="chat-notice"><button onClick={() => account.signIn('ai')}>{zh.signIn}</button></p> : !repository ? <p className="chat-notice">{zh.aiDesktop}</p> : configQuery.isError ? <p role="alert" className="chat-notice">{zh.aiConnectionFailed}<button onClick={() => { void configQuery.refetch(); }}>{zh.retryAI}</button></p> : unavailable(config) && <p role="status" className="chat-notice">{unavailable(config)}{config?.hostedAccount && !config.hostedAccount.available && account && <button onClick={() => account.openSubscription('plans')}>查看订阅</button>}</p>}
   {repository && config?.ready && !vision && <p className="chat-notice">{zh.chatTextOnly}</p>}
   <form onSubmit={event => { event.preventDefault(); if (account) void account.requireSignIn('ai', send); else void send(); }}>
    {continuation ? <p className="chat-footnote">{zh.chatRevisionOf} {continuation.title} · {continuation.reference.revision} <button type="button" aria-label={zh.chatRevisionClear} onClick={() => setContinuation(null)}>×</button></p> : canvasSource && <p className="chat-footnote">{zh.chatCanvasSource}</p>}
    <div className="chat-compose-box">
     <textarea ref={composer} aria-label={zh.chatMessage} placeholder={task?.state === 'needs_input' ? zh.chatReplyPlaceholder : zh.chatPlaceholder} maxLength={2000} rows={3} value={message} disabled={busy}
      onChange={event => updateMessage(event.target.value)} onKeyDown={event => { if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} />
     <div className="chat-compose-actions">
      <details className="chat-count-options"><summary>{zh.chatOptions}</summary><label className="chat-count">{zh.chatCount}<input aria-label={zh.chatCount} type="number" min={1} max={500} required disabled={busy} value={count} onChange={event => { const count = Number(event.target.value); setCount(count); saveDraft(project.id, { goal: message, count, scopeSelection }); }} /></label></details>
      <button type="submit" className="chat-send" aria-label={zh.chatSend} disabled={blocked || busy || running || selectionEmpty || !message.trim()}><ArrowUp size={19} /></button>
     </div>
    </div>
    <div className="chat-scope-row"><Image size={13} /><select aria-label={zh.chatScope} value={scopeSelection ? 'selected' : 'project'} disabled={busy} onChange={event => { const enabled = event.target.value === 'selected'; setScopeSelection(enabled); saveDraft(project.id, { goal: message, count, scopeSelection: enabled }); }}>
     <option value="project">{zh.chatProjectPhotos} · {project.photos.length}</option><option value="selected">{zh.chatSelectedPhotos} · {selectedIds.length}</option>
    </select></div>
   </form>
   {selectionEmpty ? <p className="chat-footnote">{zh.chatSelectPhotos}</p> : project.photos.length > 0 && vision ? <p className="chat-footnote">{zh.chatSendConsent.replace('{count}', String(imageLimit)).replace('{provider}', config?.hostedAccount?.provider ?? config?.observation?.modelId ?? config?.modelId ?? 'FocalRoom AI')}</p> : null}
  </footer>
  {settingsOpen && <Modal title={zh.settings} onClose={onSettingsClose}>
   <div className="ai-settings-fields"><div className="settings-service"><Sparkles size={17} /><span>FocalRoom AI</span></div>
    <label>{zh.chatImageLimit}<select aria-label={zh.chatImageLimit} value={imageLimit} onChange={event => setImageLimit(Number(event.target.value))}>{[20, 50, 100].map(n => <option key={n} value={n}>{n}</option>)}</select></label>
    <label>{zh.chatTimeLimit}<select aria-label={zh.chatTimeLimit} value={duration} onChange={event => setDuration(Number(event.target.value))}>{[120, 300, 480, 600].map(n => <option key={n} value={n}>{n / 60} {zh.minutes}</option>)}</select></label>
    <p className="muted">{zh.chatImagePrivacy}</p>
   </div>
  </Modal>}
  {draft && repository && <DraftReview key={draft.id + ':' + draft.revision} draft={draft} project={project} repository={repository} onClose={() => setDraftId(null)} onUpdated={updated => { onUpdated(updated); void query.refetch(); }} />}
</section>;
}

export function AIPanel(props: ComponentProps<typeof AIPanelContent>) {
 const account = useAccount();
 return <AIPanelContent key={`${props.project.id}:${account?.status?.user?.id ?? 'local'}`} {...props} />;
}
