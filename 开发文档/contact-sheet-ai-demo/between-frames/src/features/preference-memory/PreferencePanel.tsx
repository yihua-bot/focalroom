import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { PreviewEvent, Project, DesktopProjectRepository } from '../../domain/models';
import { zh } from '../../app/copy';
import { EmptyState } from '../../components/shared/EmptyState';
import { Button } from '../../components/shared/Button';

const kindLabels: Record<string, string> = { keep: '人工保留', reject: '人工淘汰', comparison: '两图比较', draft_apply: '采纳草案', draft_reject: '丢弃草案', note: '备注' };
const statusLabels: Record<string, string> = { unconfirmed: '未确认', confirmed: '已确认', corrected: '已纠正', retired: '已停用' };

export function PreferencePanel({ events, repository, project }: { events: PreviewEvent[]; repository?: DesktopProjectRepository; project: Project }) {
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['memory', project.id], queryFn: () => repository!.memory.query(project.id), enabled: Boolean(repository), staleTime: 5_000 });
  const [correcting, setCorrecting] = useState<number | null>(null);
  const [correction, setCorrection] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const evidence = query.data?.evidence ?? [];
  const hypotheses = query.data?.hypotheses ?? [];
  const act = async (action: () => Promise<unknown>) => {
    if (busy) return; setBusy(true); setError('');
    try { await action(); await query.refetch(); void client.invalidateQueries({ queryKey: ['agent', project.id] }); }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  };
  return <section><h3>{zh.memoryTitle}</h3><p className="muted">{repository ? (query.data?.notice ?? zh.memoryHint) : `${zh.memoryHint} 桌面版可查看证据支持的偏好假设。`}</p>
   <div className="stat-row">
    <div className="info-card"><span className="eyebrow">YOUR CHOICES</span><strong className="stat">{evidence.filter(item => item.kind === 'keep' || item.kind === 'reject').length || events.filter(event => event.type === 'decision').length}</strong><small>人工选择</small></div>
    <div className="info-card"><span className="eyebrow">COMPARISONS</span><strong className="stat">{evidence.filter(item => item.kind === 'comparison').length || events.filter(event => event.type === 'compare').length}</strong><small>比较记录</small></div>
    <div className="info-card"><span className="eyebrow">HYPOTHESES</span><strong className="stat">{hypotheses.length}</strong><small>偏好假设</small></div></div>
   {error && <p role="alert">{error}</p>}
   {repository && <>
    <div className="divider" /><div className="eyebrow">PREFERENCE HYPOTHESES / 偏好假设</div>
    {hypotheses.length ? hypotheses.map(hypothesis => <article key={hypothesis.id} className="info-card memory-hypothesis" data-status={hypothesis.status}>
     <strong>{hypothesis.statement}</strong>
     <p className="micro">{statusLabels[hypothesis.status]} · {hypothesis.confidence === 'moderate' ? '中等' : '初步'}置信 · 证据 {hypothesis.evidenceIds.length} 条{hypothesis.counterexampleIds.length > 0 ? ` · 反例 ${hypothesis.counterexampleIds.length} 条` : ''}{hypothesis.disabled ? ' · 已停用检索' : ''}</p>
     {hypothesis.userCorrection && <p className="micro" role="note">你的纠正：{hypothesis.userCorrection}</p>}
     <details><summary>查看证据</summary><ol className="timeline">{hypothesis.evidenceIds.map(id => evidence.find(item => item.id === id)).filter(Boolean).map(item => <li key={item!.id}><time>{new Date(item!.createdAt).toLocaleString('zh-CN')}</time><p>{kindLabels[item!.kind]}{item!.photoId ? ` · ${item!.photoId.slice(0, 8)}` : ''}</p></li>)}</ol></details>
     {hypothesis.status !== 'corrected' && <div className="agent-task-actions">
      {correcting === hypothesis.id
       ? <form onSubmit={event => { event.preventDefault(); void act(async () => { await repository.memory.correct(project.id, hypothesis.id, correction.trim()); setCorrecting(null); setCorrection(''); }); }}>
          <label>纠正说明<textarea aria-label="纠正说明" required maxLength={2000} value={correction} onChange={event => setCorrection(event.target.value)} /></label>
          <Button type="submit" size="small" disabled={busy || !correction.trim()}>提交纠正</Button>
          <Button size="small" disabled={busy} onClick={() => { setCorrecting(null); setCorrection(''); }}>取消</Button>
         </form>
       : <><Button size="small" disabled={busy} onClick={() => { setCorrecting(hypothesis.id); setCorrection(''); }}>纠正此假设</Button>
          <Button size="small" disabled={busy} onClick={() => { void act(() => repository.memory.setEnabled(project.id, hypothesis.id, hypothesis.disabled)); }}>{hypothesis.disabled ? '恢复检索' : '停用检索'}</Button></>}
     </div>}
    </article>) : <EmptyState title="还没有假设" hint="进行几次带理由的人工选择后，这里会出现证据支持的偏好假设。" />}
    <div className="divider" /><div className="eyebrow">EVIDENCE / 人工证据</div>
    {evidence.length ? <ol className="timeline">{evidence.slice(0, 20).map(item => <li key={item.id}><time>{new Date(item.createdAt).toLocaleTimeString('zh-CN')}</time><p>{kindLabels[item.kind]}{item.photoId ? ` · ${item!.photoId.slice(0, 8)}` : ''}</p></li>)}</ol> : <EmptyState title={zh.noEvents} hint="从一次人工选择开始。" />}
    <p className="micro">证据仅记录你的明确操作；Agent 的草案与建议不会出现在这里。撤销或重做不重复计数。</p>
   </>}
   {!repository && events.length ? <><div className="divider" /><div className="eyebrow">RECENT DECISIONS / 人工记录</div><ol className="timeline">{events.slice(0, 20).map(event => <li key={event.id}><time>{new Date(event.at).toLocaleTimeString('zh-CN')}</time><p>{event.description}</p></li>)}</ol></> : null}
  </section>;
}
