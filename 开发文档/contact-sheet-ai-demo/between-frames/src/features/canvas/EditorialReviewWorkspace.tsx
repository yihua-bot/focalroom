import { useEffect, useRef, useState } from 'react';
import type { BoardInfo, CanvasComparison, CanvasRepository, EditorialChange, EditorialSelection, MergeSimulation } from '../../domain/canvas';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { CanvasComparisonViewport } from './CanvasComparisonViewport';
import './editorial-review.css';

const title = (change: EditorialChange) => (change.kinds ?? [change.kind]).map(kind => zh[`editorialKind_${kind}`]).join(' · ');

export function EditorialReviewWorkspace({ repository, projectId, board, comparison, readOnly, onClose, onApplied, onSaveIndependent }: {
  repository: CanvasRepository; projectId: string; board: BoardInfo; comparison: CanvasComparison; readOnly: boolean;
  onClose: () => void; onApplied: (target: string) => Promise<void>; onSaveIndependent: () => void;
}) {
  const storageKey = `focalroom.editorial.selection.${projectId}.${board.id}.${comparison.draftRevision}`;
  const [selected, setSelected] = useState<string[]>(() => {
    try { const saved: unknown = JSON.parse(localStorage.getItem(storageKey) ?? '[]'); return Array.isArray(saved) ? saved.filter((id): id is string => typeof id === 'string' && comparison.changes.some(c => c.id === id)) : []; } catch { return []; }
  });
  const [focused, setFocused] = useState(comparison.changes[0]?.id ?? '');
  const [filter, setFilter] = useState('all');
  const [selectionVersion, setSelectionVersion] = useState(0);
  const [mode, setMode] = useState<'merged' | 'split' | 'overlay'>('merged');
  const [result, setResult] = useState<{ key: string; value: MergeSimulation }>();
  const [failure, setFailure] = useState<{ key: string; message: string }>();
  const [applying, setApplying] = useState(false);
  const applyLock = useRef(false);
  const key = JSON.stringify([selectionVersion, selected, comparison.currentSourceRevision, comparison.draftRevision]);
  const canSimulate = Boolean(comparison.sourceBoardId && comparison.currentSourceRevision && repository.simulateMerge);
  const selection: EditorialSelection = {
    draftId: board.id, draftRevision: comparison.draftRevision, targetBoardId: comparison.sourceBoardId ?? '',
    expectedTargetRevision: comparison.currentSourceRevision ?? 0, selectedChangeIds: selected,
  };
  const simulation = result?.key === key ? result.value : undefined;
  const error = failure?.key === key ? failure.message : undefined;
  const change = comparison.changes.find(item => item.id === focused);
  const list = comparison.changes.filter(c => filter === 'all' || (c.kinds ?? [c.kind]).includes(filter));
  const pending = repository.pendingCommands?.(projectId).some(c => c.boardId === selection.targetBoardId);

  useEffect(() => {
    if (!canSimulate) return;
    let active = true;
    const request: EditorialSelection = { draftId: board.id, draftRevision: comparison.draftRevision, targetBoardId: comparison.sourceBoardId!, expectedTargetRevision: comparison.currentSourceRevision!, selectedChangeIds: JSON.parse(key)[1] };
    repository.simulateMerge!(projectId, request).then(value => { if (active) setResult({ key, value }); })
      .catch(error => { if (active) setFailure({ key, message: error instanceof Error ? error.message : zh.editorialPreviewFailed }); });
    return () => { active = false; };
  }, [canSimulate, repository, projectId, board.id, comparison.draftRevision, comparison.sourceBoardId, comparison.currentSourceRevision, key]);

  const select = (ids: string[]) => {
    if (applyLock.current) return;
    setSelected(ids); setSelectionVersion(version => version + 1);
    try { localStorage.setItem(storageKey, JSON.stringify(ids)); } catch { /* 存储不可用仍可审稿，命令提交另有持久回执保护。 */ }
  };
  const toggle = (id: string) => select(selected.includes(id) ? selected.filter(value => value !== id) : [...selected, id]);
  const apply = async () => {
    if (applyLock.current || !simulation?.canApply || readOnly || !repository.applyMerge || pending) return;
    applyLock.current = true; setApplying(true);
    try {
      await repository.applyMerge(projectId, selection, simulation.simulationId);
      try { localStorage.removeItem(storageKey); } catch { /* 已持久化的正式回执是权威。 */ }
      await onApplied(selection.targetBoardId);
    } catch (error) { setFailure({ key, message: error instanceof Error ? error.message : zh.editorialApplyFailed }); }
    finally { applyLock.current = false; setApplying(false); }
  };
  const rationale = (() => { try { const value = JSON.parse(change?.after?.contentJson ?? '{}').rationale; return typeof value === 'string' ? value : ''; } catch { return ''; } })();
  const status = applying ? zh.editorialApplying : error ?? (pending ? zh.editorialPending : !canSimulate ? zh.editorialNoSource : simulation?.conflicts.length ? simulation.conflicts.join('；') : !selected.length ? zh.editorialNone : !simulation ? zh.editorialSimulating : zh.editorialPreviewReady);

  return <Modal title={zh.editorialTitle} className="editorial-workspace" onClose={() => { if (!applyLock.current) onClose(); }}>
    <div className="editorial-layout" onKeyDown={event => {
      event.stopPropagation();
      if ((event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
      if ((event.metaKey || event.ctrlKey) && ['1', '2', '3'].includes(event.key)) {
        event.preventDefault(); setMode(event.key === '1' ? 'split' : event.key === '2' ? 'overlay' : 'merged');
      }
    }}>
      <header className="editorial-header"><div><strong>{board.title}</strong><span>{zh.editorialBasedOn} {comparison.sourceRevision ?? '—'}</span></div>
        <div role="group" aria-label={zh.editorialPreviewMode}>{(['merged', 'split', 'overlay'] as const).map(value => <Button key={value} size="small" aria-pressed={mode === value} onClick={() => setMode(value)}>{zh[`editorialMode_${value}`]}</Button>)}</div>
      </header>
      <div className="editorial-columns">
        <aside className="editorial-change-list" aria-label={zh.editorialChanges}>
          <h3>{zh.editorialChanges} · {comparison.changes.length}</h3>
          <select aria-label={zh.editorialFilter} value={filter} onChange={event => setFilter(event.target.value)}><option value="all">{zh.editorialAll}</option>{[...new Set(comparison.changes.flatMap(c => c.kinds ?? [c.kind]))].map(kind => <option value={kind} key={kind}>{zh[`editorialKind_${kind}`]}</option>)}</select>
          <div className="editorial-selection-actions"><Button size="small" disabled={applying} onClick={() => select([...new Set([...selected, ...list.map(c => c.id)])])}>{zh.editorialSelectAll}</Button><Button size="small" disabled={applying} onClick={() => select(selected.filter(id => !list.some(c => c.id === id)))}>{zh.editorialClear}</Button></div>
          <ul>{list.map((item, index) => <li key={item.id} className={focused === item.id ? 'is-focused' : ''}>
            <input type="checkbox" aria-label={`${zh.editorialSelect} ${index + 1}`} checked={selected.includes(item.id) || Boolean(simulation?.autoIncludedChangeIds.includes(item.id))} disabled={applying || Boolean(simulation?.autoIncludedChangeIds.includes(item.id))} onChange={() => toggle(item.id)} />
            <button type="button" className="editorial-change" aria-current={focused === item.id ? 'true' : undefined} onClick={() => setFocused(item.id)} onKeyDown={event => {
              if (event.key === ' ') { event.preventDefault(); toggle(item.id); }
              if (['ArrowUp', 'ArrowDown'].includes(event.key)) { event.preventDefault(); const next = (index + (event.key === 'ArrowUp' ? -1 : 1) + list.length) % list.length; setFocused(list[next].id); (event.currentTarget.closest('ul')?.querySelectorAll<HTMLButtonElement>('.editorial-change')[next])?.focus(); }
            }}><span>{String(index + 1).padStart(2, '0')}</span><strong>{title(item)}</strong><small>{zh[`editorialNode_${item.nodeType}`]}</small></button>
          </li>)}</ul>
        </aside>
        <CanvasComparisonViewport mode={mode} baseline={comparison.baselineNodes} draft={comparison.draftNodes ?? []} merged={simulation?.mergedNodes} nodeIds={simulation?.nodeIdMap} change={change} changes={comparison.changes} onFocus={id => setFocused(id)} />
        <aside className="editorial-inspector" aria-label={zh.editorialDetails}>
          <h3>{change ? title(change) : zh.editorialDetails}</h3>
          {change?.before && <p>{zh.editorialBefore}: {Math.round(change.before.x)}, {Math.round(change.before.y)} · {Math.round(change.before.width)} × {Math.round(change.before.height)}</p>}
          {change?.after && <p>{zh.editorialAfter}: {Math.round(change.after.x)}, {Math.round(change.after.y)} · {Math.round(change.after.width)} × {Math.round(change.after.height)}</p>}
          {change?.after?.contentJson && <blockquote>{(() => { try { const c = JSON.parse(change.after.contentJson); return String(c.text ?? c.label ?? ''); } catch { return ''; } })()}</blockquote>}
          <h4>{zh.editorialReason}</h4><p>{rationale || zh.editorialNoReason}</p><p className="muted">{zh.editorialEvidenceNote}</p>
          {!!change?.requiredDependencies?.length && <p>{zh.editorialDependency}: {change.requiredDependencies.length}</p>}
          {simulation?.autoIncludedChangeIds.length ? <p>{zh.editorialAutoIncluded} {simulation.autoIncludedChangeIds.length}</p> : null}
        </aside>
      </div>
      <footer className="editorial-apply-bar"><div><strong>{zh.editorialSelected} {simulation?.effectiveChangeIds.length ?? selected.length} / {comparison.changes.length}</strong><p role="status" aria-live="polite">{status}</p></div>
        <Button disabled={applying || readOnly} onClick={onSaveIndependent}>{zh.editorialSaveIndependent}</Button>
        <Button variant="primary" disabled={applying || readOnly || pending || Boolean(error) || !simulation?.canApply} onClick={() => void apply()}>{zh.editorialApply}</Button>
      </footer>
    </div>
  </Modal>;
}
