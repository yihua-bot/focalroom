import { userFacingError } from '../../services/userFacingError';
import { useEffect, useRef, useState } from 'react';
import type { BoardInfo, CanvasComparison, CanvasRepository, EditorialChange, EditorialSelection, EditorialResolution, MergeSimulation, StructuralResolution } from '../../domain/canvas';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { CanvasComparisonViewport } from './CanvasComparisonViewport';
import { EditorialDiscussion } from './EditorialDiscussion';
import { EditorialRevisionContext } from './EditorialRevisionContext';
import { EditorialStructuralConflicts } from './EditorialStructuralConflicts';
import './editorial-review.css';

const title = (change: EditorialChange) => (change.kinds ?? [change.kind]).map(kind => zh[`editorialKind_${kind}`]).join(' · ');
const conflictValue = (field: EditorialResolution['field'], value: unknown): string => {
  if (value == null) return '—';
  if (Array.isArray(value)) return value.join(', ');
  if (field === 'rendition') return zh.editorialField_rendition;
  if (typeof value === 'string' && ['content','style','children'].includes(field)) {
    try { const parsed = JSON.parse(value); return String(parsed.text ?? parsed.label ?? parsed.color ?? (Array.isArray(parsed) ? parsed.length : '—')); } catch { return value; }
  }
  return String(value);
};

export function EditorialReviewWorkspace({ repository, projectId, board, comparison: initialComparison, readOnly, onClose, onApplied, onSaveIndependent, onRevise }: {
  repository: CanvasRepository; projectId: string; board: BoardInfo; comparison: CanvasComparison; readOnly: boolean;
  onClose: () => void; onApplied: (target: string) => Promise<void>; onSaveIndependent: () => void;
  onRevise?: () => void;
}) {
  const [refreshedComparison, setRefreshedComparison] = useState<CanvasComparison>();
  const comparison = refreshedComparison ?? initialComparison;
  const [resolutions, setResolutions] = useState<EditorialResolution[]>([]);
  const [structuralResolutions, setStructuralResolutions] = useState<StructuralResolution[]>([]);
  const [fieldsByChange, setFieldsByChange] = useState<Record<string, EditorialResolution['field'][]>>({});
  const [appliedTarget, setAppliedTarget] = useState<string>();
  const storageKey = `focalroom.editorial.selection.${projectId}.${board.id}.${comparison.draftRevision}`;
  const [selected, setSelected] = useState<string[]>(() => {
    try { const saved: unknown = JSON.parse(localStorage.getItem(storageKey) ?? '[]'); return Array.isArray(saved) ? saved.filter((id): id is string => typeof id === 'string' && comparison.changes.some(c => c.id === id && c.decision !== 'accepted')) : []; } catch { return []; }
  });
  const [focused, setFocused] = useState(comparison.changes[0]?.id ?? '');
  const [filter, setFilter] = useState('all');
  const [selectionVersion, setSelectionVersion] = useState(0);
  const [mode, setMode] = useState<'merged' | 'split' | 'overlay'>('merged');
  const [result, setResult] = useState<{ key: string; value: MergeSimulation }>();
  const [failure, setFailure] = useState<{ key: string; message: string }>();
  const [applying, setApplying] = useState(false);
  const applyLock = useRef(false);
  const key = JSON.stringify([selectionVersion, selected, comparison.currentSourceRevision, comparison.draftRevision, resolutions, fieldsByChange, structuralResolutions]);
  const canSimulate = Boolean(comparison.sourceBoardId && comparison.currentSourceRevision && repository.simulateMerge);
  const selection: EditorialSelection = {
    draftId: board.id, draftRevision: comparison.draftRevision, targetBoardId: comparison.sourceBoardId ?? '',
    expectedTargetRevision: comparison.currentSourceRevision ?? 0, selectedChangeIds: selected,
    ...(resolutions.length ? { resolutions } : {}),
    ...(Object.keys(fieldsByChange).length ? { fieldsByChange } : {}),
    ...(structuralResolutions.length ? { structuralResolutions } : {}),
  };
  const simulation = result?.key === key ? result.value : undefined;
  const error = failure?.key === key ? failure.message : undefined;
  const change = comparison.changes.find(item => item.id === focused);
  const list = comparison.changes.filter(c => filter === 'all' || (c.kinds ?? [c.kind]).includes(filter));
  const pending = repository.pendingCommands?.(projectId).some(c => c.boardId === selection.targetBoardId);

  useEffect(() => {
    if (!canSimulate) return;
    let active = true;
    const captured = JSON.parse(key);
    const request: EditorialSelection = { draftId: board.id, draftRevision: comparison.draftRevision, targetBoardId: comparison.sourceBoardId!, expectedTargetRevision: comparison.currentSourceRevision!, selectedChangeIds: captured[1], ...(captured[4].length ? { resolutions: captured[4] } : {}), ...(Object.keys(captured[5]).length ? { fieldsByChange: captured[5] } : {}), ...(captured[6].length ? {structuralResolutions:captured[6]} : {}) };
    repository.simulateMerge!(projectId, request).then(value => { if (active) setResult({ key, value }); })
      .catch(error => { if (active) setFailure({ key, message: userFacingError(error, zh.editorialPreviewFailed) }); });
    return () => { active = false; };
  }, [canSimulate, repository, projectId, board.id, comparison.draftRevision, comparison.sourceBoardId, comparison.currentSourceRevision, key]);

  const select = (ids: string[]) => {
    if (applyLock.current) return;
    setSelected(ids); setResolutions(values => values.filter(r => ids.includes(r.changeId))); setSelectionVersion(version => version + 1);
    setStructuralResolutions(values => values.filter(r => ids.includes(r.changeId)));
    setFieldsByChange(values => Object.fromEntries(ids.flatMap(id => {
      const change = comparison.changes.find(c => c.id === id);
      const fields = values[id] ?? (change?.acceptedFields?.length ? change.fields?.filter(f => !change.acceptedFields?.includes(f)) : undefined);
      return fields?.length ? [[id, fields]] : [];
    })));
    try { localStorage.setItem(storageKey, JSON.stringify(ids)); } catch { /* 存储不可用仍可审稿，命令提交另有持久回执保护。 */ }
  };
  const toggle = (id: string) => select(selected.includes(id) ? selected.filter(value => value !== id) : [...selected, id]);
  const apply = async () => {
    if (applyLock.current || !simulation?.canApply || readOnly || !repository.applyMerge || pending) return;
    applyLock.current = true; setApplying(true);
    try {
      await repository.applyMerge(projectId, selection, simulation.simulationId);
      try { localStorage.removeItem(storageKey); } catch { /* 已持久化的正式回执是权威。 */ }
      if (repository.compareDraft) {
        setAppliedTarget(selection.targetBoardId);
        setRefreshedComparison(await repository.compareDraft(projectId, board.id));
        setSelected([]); setResolutions([]); setStructuralResolutions([]); setFieldsByChange({}); setSelectionVersion(v => v + 1);
      } else await onApplied(selection.targetBoardId);
    } catch (error) { setFailure({ key, message: userFacingError(error, zh.editorialApplyFailed) }); }
    finally { applyLock.current = false; setApplying(false); }
  };
  const decide = async (decision: 'pending'|'rejected'|'deferred', ids = selected) => {
    if (!repository.recordDecision || !repository.compareDraft || applyLock.current || readOnly || pending || !ids.length) return;
    applyLock.current = true; setApplying(true);
    try {
      await repository.recordDecision(projectId, {...selection,selectedChangeIds:ids,fieldsByChange:Object.fromEntries(Object.entries(fieldsByChange).filter(([id]) => ids.includes(id)))}, decision);
      setRefreshedComparison(await repository.compareDraft(projectId, board.id));
      selectAfterDecision();
    } catch (error) { setFailure({ key, message: userFacingError(error, zh.editorialApplyFailed) }); }
    finally { applyLock.current = false; setApplying(false); }
  };
  const selectAfterDecision = () => {
    setSelected([]); setResolutions([]); setStructuralResolutions([]); setFieldsByChange({}); setSelectionVersion(v => v + 1);
    try { localStorage.removeItem(storageKey); } catch { /* 决定已保存至命令回执。 */ }
  };
  const close = () => { if (!applyLock.current) { if (appliedTarget) void onApplied(appliedTarget); else onClose(); } };
  const rationale = (() => { try { const value = JSON.parse(change?.after?.contentJson ?? '{}').rationale; return typeof value === 'string' ? value : ''; } catch { return ''; } })();
  const status = applying ? zh.editorialApplying : error ?? (pending ? zh.editorialPending : !canSimulate ? zh.editorialNoSource : simulation?.conflicts.length ? simulation.conflicts.join('；') : simulation?.attributeConflicts?.length || simulation?.structuralConflicts?.some(c => !c.resolved) ? zh.editorialResolveConflicts : !selected.length ? zh.editorialNone : !simulation ? zh.editorialSimulating : zh.editorialPreviewReady);

  return <Modal title={zh.editorialTitle} className="editorial-workspace" onClose={close}>
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
          <div className="editorial-selection-actions"><Button size="small" disabled={applying} onClick={() => select([...new Set([...selected, ...list.filter(c => c.decision !== 'accepted').map(c => c.id)])])}>{zh.editorialSelectAll}</Button><Button size="small" disabled={applying} onClick={() => select(selected.filter(id => !list.some(c => c.id === id)))}>{zh.editorialClear}</Button></div>
          <ul>{list.map((item, index) => <li key={item.id} className={focused === item.id ? 'is-focused' : ''}>
            <input type="checkbox" aria-label={`${zh.editorialSelect} ${index + 1}`} checked={selected.includes(item.id) || Boolean(simulation?.autoIncludedChangeIds.includes(item.id))} disabled={applying || item.decision === 'accepted' || Boolean(simulation?.autoIncludedChangeIds.includes(item.id))} onChange={() => toggle(item.id)} />
            <button type="button" className="editorial-change" aria-current={focused === item.id ? 'true' : undefined} onClick={() => setFocused(item.id)} onKeyDown={event => {
              if (event.key === ' ') { event.preventDefault(); if (item.decision !== 'accepted') toggle(item.id); }
              if (['ArrowUp', 'ArrowDown'].includes(event.key)) { event.preventDefault(); const next = (index + (event.key === 'ArrowUp' ? -1 : 1) + list.length) % list.length; setFocused(list[next].id); (event.currentTarget.closest('ul')?.querySelectorAll<HTMLButtonElement>('.editorial-change')[next])?.focus(); }
            }}><span>{String(index + 1).padStart(2, '0')}</span><strong>{title(item)}</strong><small>{zh[`editorialNode_${item.nodeType}`]}{item.decision && item.decision !== 'pending' ? ` · ${zh[`editorialDecision_${item.decision}`]}` : ''}</small></button>
          </li>)}</ul>
        </aside>
        <CanvasComparisonViewport mode={mode} baseline={comparison.currentSourceNodes ?? comparison.baselineNodes} draft={comparison.draftNodes ?? []} merged={simulation?.mergedNodes} nodeIds={simulation?.nodeIdMap} change={change} changes={comparison.changes} onFocus={id => setFocused(id)} />
        <aside className="editorial-inspector" aria-label={zh.editorialDetails}>
          <h3>{change ? title(change) : zh.editorialDetails}</h3>
          {change?.before && <p>{zh.editorialBefore}: {Math.round(change.before.x)}, {Math.round(change.before.y)} · {Math.round(change.before.width)} × {Math.round(change.before.height)}</p>}
          {change?.after && <p>{zh.editorialAfter}: {Math.round(change.after.x)}, {Math.round(change.after.y)} · {Math.round(change.after.width)} × {Math.round(change.after.height)}</p>}
          {change?.after?.contentJson && <blockquote>{(() => { try { const c = JSON.parse(change.after.contentJson); return String(c.text ?? c.label ?? ''); } catch { return ''; } })()}</blockquote>}
          {change?.kind === 'change' && !!change.fields?.length && <fieldset className="editorial-property-selection"><legend>{zh.editorialProperties}</legend>{change.fields.map(field => <label key={field}><input type="checkbox" aria-label={zh[`editorialField_${field}`]} disabled={applying || readOnly || change.decision === 'accepted' || change.acceptedFields?.includes(field)} checked={Boolean(change.acceptedFields?.includes(field)) || (selected.includes(change.id) && (fieldsByChange[change.id] ?? change.fields!).includes(field))} onChange={event => {
            const available = (change.fields ?? []).filter(f => !change.acceptedFields?.includes(f));
            const previous = selected.includes(change.id) ? fieldsByChange[change.id] ?? available : [];
            const next = event.target.checked ? [...new Set([...previous, field])] : previous.filter(f => f !== field);
            if (!next.length) select(selected.filter(id => id !== change.id));
            else { if (!selected.includes(change.id)) setSelected(ids => [...ids, change.id]); setFieldsByChange(values => ({ ...values, [change.id]: next })); setResolutions(values => values.filter(r => r.changeId !== change.id || next.includes(r.field))); setSelectionVersion(v => v + 1); }
          }} />{zh[`editorialField_${field}`]}{change.fieldDecisions?.[field] && change.fieldDecisions[field] !== 'pending' ? ` · ${zh[`editorialDecision_${change.fieldDecisions[field]}`]}` : change.acceptedFields?.includes(field) ? ` · ${zh.editorialDecision_accepted}` : ''}</label>)}</fieldset>}
          <h4>{zh.editorialReason}</h4><p>{rationale || zh.editorialNoReason}</p><p className="muted">{zh.editorialEvidenceNote}</p>
          {!!change?.requiredDependencies?.length && <p>{zh.editorialDependency}: {change.requiredDependencies.length}</p>}
          {simulation?.autoIncludedChangeIds.length ? <p>{zh.editorialAutoIncluded} {simulation.autoIncludedChangeIds.length}</p> : null}
          {!!simulation?.attributeConflicts?.length && <section className="editorial-conflicts" aria-label={zh.editorialResolveConflicts}><h4>{zh.editorialResolveConflicts}</h4>{simulation.attributeConflicts.map(conflict => <fieldset key={`${conflict.changeId}:${conflict.field}`}><legend>{zh[`editorialField_${conflict.field}`]}</legend>
            <p>{zh.editorialCurrent}: {conflictValue(conflict.field, conflict.current)}</p><p>{zh.editorialProposed}: {conflictValue(conflict.field, conflict.proposed)}</p>
            {(['keep_current','use_draft'] as const).map(choice => <Button key={choice} size="small" disabled={applying} onClick={() => { setResolutions(values => [...values.filter(r => r.changeId !== conflict.changeId || r.field !== conflict.field), { changeId: conflict.changeId, field: conflict.field, choice }]); setSelectionVersion(v => v + 1); }}>{zh[`editorialChoice_${choice}`]}</Button>)}
          </fieldset>)}</section>}
          <EditorialStructuralConflicts conflicts={simulation?.structuralConflicts ?? []} disabled={applying || readOnly || Boolean(pending)} canKeep={Boolean(repository.recordDecision && repository.compareDraft)} onChoose={resolution => {
            if (applyLock.current || readOnly) return;
            setStructuralResolutions(values => [...values.filter(r => r.changeId !== resolution.changeId),resolution]); setSelectionVersion(v => v + 1);
          }} onKeep={id => void decide('rejected',[id])} />
          <EditorialDiscussion key={`${projectId}:${board.id}:${comparison.draftRevision}`} repository={repository} projectId={projectId} draftId={board.id} revision={comparison.draftRevision} changeId={change?.id} changeTitles={Object.fromEntries(comparison.changes.map((item,index) => [item.id,`${index+1} · ${title(item)}`]))} readOnly={readOnly} onRevise={onRevise} />
          <EditorialRevisionContext key={`context:${projectId}:${board.id}`} repository={repository} projectId={projectId} draftId={board.id} />
        </aside>
      </div>
      <footer className="editorial-apply-bar"><div><strong>{zh.editorialSelected} {simulation?.effectiveChangeIds.length ?? selected.length} / {comparison.changes.length}</strong><p role="status" aria-live="polite">{status}</p></div>
        {repository.recordDecision && <><Button disabled={applying || readOnly || pending || !selected.length} onClick={() => void decide('deferred')}>{zh.editorialDefer}</Button><Button disabled={applying || readOnly || pending || !selected.length} onClick={() => void decide('rejected')}>{zh.editorialReject}</Button></>}
        {appliedTarget && <Button disabled={applying} onClick={close}>{zh.editorialDone}</Button>}
        <Button disabled={applying || readOnly} onClick={onSaveIndependent}>{zh.editorialSaveIndependent}</Button>
        <Button variant="primary" disabled={applying || readOnly || pending || Boolean(error) || !simulation?.canApply} onClick={() => void apply()}>{zh.editorialApply}</Button>
      </footer>
    </div>
  </Modal>;
}
