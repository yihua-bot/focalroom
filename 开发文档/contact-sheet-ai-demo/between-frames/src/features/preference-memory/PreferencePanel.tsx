import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { PreviewEvent, Project, DesktopProjectRepository } from '../../domain/models';
import { zh } from '../../app/copy';
import { EmptyState } from '../../components/shared/EmptyState';
import { MemoryEvidenceList } from './MemoryEvidenceList';
import { Button } from '../../components/shared/Button';

const statusLabels: Record<string, string> = { unconfirmed: '未确认', confirmed: '已确认', corrected: '已纠正', retired: '已移除' };

export function PreferencePanel({ events, repository, project }: { events: PreviewEvent[]; repository?: DesktopProjectRepository; project: Project }) {
  const client = useQueryClient();
  const [search, setSearch] = useState('');
  const [keyword, setKeyword] = useState('');
  const [showRemoved, setShowRemoved] = useState(false);
  const [evidenceLimit, setEvidenceLimit] = useState(20);
  const query = useQuery({ queryKey: ['memory', project.id, keyword, project.revision], queryFn: () => repository!.memory.query(project.id, keyword), enabled: Boolean(repository), staleTime: 5_000 });
  const [correcting, setCorrecting] = useState<number | null>(null);
  const [correction, setCorrection] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const evidence = query.data?.evidence ?? [];
  const hypotheses = (query.data?.hypotheses ?? []).filter(item => showRemoved || item.status !== 'retired');
  const activeEvidence = evidence.filter(item => !item.excluded);
  const act = async (action: () => Promise<unknown>) => {
    if (busy) return; setBusy(true); setError('');
    try { await action(); await client.invalidateQueries({ queryKey: ['memory'] }); void client.invalidateQueries({ queryKey: ['agent'] }); }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  };
  return <section><h3>{zh.memoryTitle}</h3><p className="muted">{repository ? (query.data?.notice ?? zh.memoryHint) : `${zh.memoryHint} 桌面版可查看证据支持的偏好假设。`}</p>
   <div className="stat-row">
    <div className="info-card"><span className="eyebrow">YOUR CHOICES</span><strong className="stat">{repository ? activeEvidence.filter(item => item.kind === 'keep' || item.kind === 'reject').length : events.filter(event => event.type === 'decision').length}</strong><small>人工选择</small></div>
    <div className="info-card"><span className="eyebrow">COMPARISONS</span><strong className="stat">{repository ? activeEvidence.filter(item => item.kind === 'comparison').length : events.filter(event => event.type === 'compare').length}</strong><small>比较记录</small></div>
    <div className="info-card"><span className="eyebrow">HYPOTHESES</span><strong className="stat">{hypotheses.length}</strong><small>偏好假设</small></div></div>
   {(error || query.error) && <p role="alert">{error || String(query.error)}</p>}
   {repository && <>
    <p className="micro">调整记忆会暂停当前 Agent，请基于当前记忆重新委托。选片、作品序列与撤销历史保持原样。</p>
    <div className="info-card">
      <label><input type="checkbox" aria-label="允许离线 Agent 引用其他本地项目记忆" checked={query.data?.crossProjectEnabled ?? false} disabled={busy || !query.data || query.isError}
        onChange={event => {
          const enabled = event.target.checked;
          if (enabled && !window.confirm('允许离线 Agent 引用其他本地项目已确认的假设？这些线索会附来源项目与证据引用。共享项目和云模型不参与；更改授权会暂停执行中的任务，需要重新委托。')) return;
          void act(() => repository.memory.request(project.id, { type: 'SetCrossProjectMemory', enabled }));
        }} /> 允许离线 Agent 引用其他本地项目记忆</label>
      <p className="micro">{query.data?.authorMemoryNotice || '默认关闭。仅限本机单一本地作者；共享项目和云模型不参与。更改授权后需要重新委托。'}</p>
    </div>
    <form className="memory-search" onSubmit={event => { event.preventDefault(); setKeyword(search.trim()); }}>
      <label>搜索偏好关键词<input className="text-input" aria-label="搜索偏好关键词" maxLength={400} value={search} onChange={event => setSearch(event.target.value)} placeholder="例如：留白、逆光" /></label>
      <Button type="submit" size="small" disabled={busy}>搜索</Button>
      {keyword && <Button size="small" onClick={() => { setSearch(''); setKeyword(''); }}>清空搜索</Button>}
    </form>
    {query.data?.crossProjectEnabled && <section aria-label="其他本地项目的已确认线索">
      <div className="divider" /><div className="eyebrow">AUTHOR MEMORY / 其他项目线索（只读）</div>
      <p className="micro">这些是其他项目中的判断线索，引用不代表永久偏好。请进入来源项目查看证据、纠正或停用。</p>
      {(query.data.authorHypotheses ?? []).map(item => <article key={item.id} className="info-card" data-author-memory={item.id}>
        <strong>{item.statement}</strong>
        <p className="micro">已确认 · 证据 {item.evidenceIds.length} 条 · 反例 {item.counterexampleIds.length} 条 · 假设 #{item.id}</p>
        <p className="micro">来源项目：{item.projectId} · 证据：{item.evidenceIds.map(id => `#${id}`).join('、')}</p>
        {item.counterexampleIds.length > 0 && <p className="micro">反例：{item.counterexampleIds.map(id => `#${id}`).join('、')}</p>}
        <a href={`#/project/${encodeURIComponent(item.projectId)}`}>进入来源项目查看与管理</a>
      </article>)}
      {!query.data.authorHypotheses?.length && <p className="micro">没有符合条件的其他项目线索。</p>}
    </section>}
    <label className="micro"><input type="checkbox" checked={showRemoved} onChange={event => setShowRemoved(event.target.checked)} /> 显示已移除假设</label>
    <div className="divider" /><div className="eyebrow">PREFERENCE HYPOTHESES / 偏好假设</div>
    {hypotheses.length ? hypotheses.map(hypothesis => <article key={hypothesis.id} className="info-card memory-hypothesis" data-status={hypothesis.status}>
     <strong>{hypothesis.statement}</strong>
     <p className="micro">{statusLabels[hypothesis.status]} · {hypothesis.confidence === 'moderate' ? '中等' : '初步'}置信 · 证据 {hypothesis.evidenceIds.length} 条{hypothesis.counterexampleIds.length > 0 ? ` · 反例 ${hypothesis.counterexampleIds.length} 条` : ''}{hypothesis.disabled ? ' · 已停用检索' : ''}</p>
     {hypothesis.userCorrection && <p className="micro" role="note">你的纠正：{hypothesis.userCorrection}</p>}
     <details><summary>查看证据</summary><MemoryEvidenceList ids={hypothesis.evidenceIds} evidence={evidence} photos={project.photos} /></details>
     {hypothesis.counterexampleIds.length > 0 && <details><summary>查看反例</summary><MemoryEvidenceList ids={hypothesis.counterexampleIds} evidence={evidence} photos={project.photos} /></details>}
     <div className="agent-task-actions">
      {hypothesis.status === 'unconfirmed' && <Button size="small" disabled={busy || !hypothesis.evidenceIds.length} onClick={() => { void act(() => repository.memory.request(project.id, { type: 'ConfirmHypothesis', hypothesisId: hypothesis.id })); }}>确认此假设</Button>}
      <Button size="small" disabled={busy} onClick={() => { void act(() => repository.memory.request(project.id, { type: hypothesis.status === 'retired' ? 'RestoreHypothesis' : 'RetireHypothesis', hypothesisId: hypothesis.id })); }}>{hypothesis.status === 'retired' ? '恢复此假设' : '移除此假设'}</Button>
     </div>
     {hypothesis.status === 'corrected' && <div className="agent-task-actions">
       <Button size="small" disabled={busy} onClick={() => { if (window.confirm('物理删除这条已纠正的假设？不可撤销；删除后不会从你的操作中重新生成。')) { void act(() => repository.memory.deleteHypothesis(project.id, hypothesis.id)); } }}>删除假设</Button>
     </div>}
     {!['corrected', 'retired'].includes(hypothesis.status) && <div className="agent-task-actions">
      {correcting === hypothesis.id
       ? <form onSubmit={event => { event.preventDefault(); void act(async () => { await repository.memory.correct(project.id, hypothesis.id, correction.trim()); setCorrecting(null); setCorrection(''); }); }}>
          <label>纠正说明<textarea className="text-input" aria-label="纠正说明" required maxLength={2000} value={correction} onChange={event => setCorrection(event.target.value)} /></label>
          <Button type="submit" size="small" disabled={busy || !correction.trim()}>提交纠正</Button>
          <Button size="small" disabled={busy} onClick={() => { setCorrecting(null); setCorrection(''); }}>取消</Button>
         </form>
       : <><Button size="small" disabled={busy} onClick={() => { setCorrecting(hypothesis.id); setCorrection(''); }}>纠正此假设</Button>
          <Button size="small" disabled={busy} onClick={() => { void act(() => repository.memory.setEnabled(project.id, hypothesis.id, hypothesis.disabled)); }}>{hypothesis.disabled ? '恢复检索' : '停用检索'}</Button>
          <Button size="small" disabled={busy} onClick={() => { if (window.confirm('物理删除这条假设？不可撤销；删除后不会从你的操作中重新生成。')) { void act(() => repository.memory.deleteHypothesis(project.id, hypothesis.id)); } }}>删除假设</Button></>}
     </div>}
    </article>) : <EmptyState title={keyword ? "没有匹配的假设" : "还没有假设"} hint="进行几次带理由的人工选择后，这里会出现证据支持的偏好假设。" />}
    <div className="divider" /><div className="eyebrow">EVIDENCE / 人工证据</div>
    <div className="agent-task-actions"><Button size="small" disabled={busy} onClick={() => { if (window.confirm('清空本项目的视觉观察结果及相关检索缓存？已上传的云端数据不受影响；再次视觉观察会重新申请授权并上传。')) { void act(() => repository.memory.purgeObservationCache(project.id)); } }}>清空视觉观察缓存</Button><span className="micro">不含证据与假设；删除本机缓存的观察结果及相关检索缓存。</span></div>
    {evidence.length ? <MemoryEvidenceList ids={evidence.slice(0, evidenceLimit).map(item => item.id)} evidence={evidence} photos={project.photos} busy={busy} onExclude={item => { void act(() => repository.memory.request(project.id, { type: 'SetEvidenceExcluded', evidenceId: item.id, excluded: !item.excluded })); }} onDelete={item => { void act(() => repository.memory.deleteEvidence(project.id, item.id)); }} /> : <EmptyState title={zh.noEvents} hint="从一次人工选择开始。" />}
    {evidence.length > evidenceLimit && <Button size="small" onClick={() => setEvidenceLimit(value => value + 20)}>查看更多证据</Button>}
    <p className="micro">证据仅记录你的明确操作；Agent 的草案与建议不会出现在这里。撤销或重做不重复计数。</p>
   </>}
   {!repository && events.length ? <><div className="divider" /><div className="eyebrow">RECENT DECISIONS / 人工记录</div><ol className="timeline">{events.slice(0, 20).map(event => <li key={event.id}><time>{new Date(event.at).toLocaleTimeString('zh-CN')}</time><p>{event.description}</p></li>)}</ol></> : null}
  </section>;
}
