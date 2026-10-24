import { userFacingError } from '../../services/userFacingError';
import { useEffect, useRef, useState } from 'react';
import type { CanvasRepository, EditorialHistoryEntry, RevertSimulation } from '../../domain/canvas';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { CanvasComparisonViewport } from './CanvasComparisonViewport';
import './editorial-review.css';

export function EditorialHistoryDialog({ repository, projectId, boardId, readOnly, onClose, onChanged }: {
  repository: CanvasRepository; projectId: string; boardId: string; readOnly: boolean; onClose: () => void; onChanged: () => Promise<void>;
}) {
  const [entries, setEntries] = useState<EditorialHistoryEntry[]>([]);
  const [selected, setSelected] = useState('');
  const [previewRequest, setPreviewRequest] = useState(0);
  const [preview, setPreview] = useState<RevertSimulation>();
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const lock = useRef(false);
  useEffect(() => {
    let active = true;
    repository.editorialHistory?.(projectId, boardId).then(values => { if (active) { setEntries(values); setHasMore(values.length === 100); } }).catch(error => { if (active) setError(userFacingError(error, zh.editorialHistoryFailed)); });
    return () => { active = false; };
  }, [repository, projectId, boardId]);
  useEffect(() => {
    if (!selected || !repository.simulateRevert) return;
    let active = true;
    repository.simulateRevert(projectId, boardId, selected).then(value => { if (active) setPreview(value); }).catch(error => { if (active) setError(userFacingError(error, zh.editorialHistoryFailed)); });
    return () => { active = false; };
  }, [repository, projectId, boardId, selected, previewRequest]);
  const confirm = async () => {
    if (!preview?.canApply || preview.mergeCommandId !== selected || !repository.revertMerge || readOnly || lock.current) return;
    lock.current = true; setBusy(true); setError('');
    try {
      await repository.revertMerge(projectId, boardId, preview);
      await onChanged();
      const latest = await repository.editorialHistory!(projectId, boardId);
      setEntries(latest); setHasMore(latest.length === 100);
      setSelected(''); setPreview(undefined);
    } catch (error) { setError(userFacingError(error, zh.editorialHistoryFailed)); }
    finally { lock.current = false; setBusy(false); }
  };
  return <Modal title={zh.editorialHistory} className="editorial-history-dialog" onClose={() => { if (!lock.current) onClose(); }}>
    <p>{zh.editorialHistoryHint}</p>
    <div className="editorial-history-layout"><div className="editorial-history-list">
      {!entries.length && <p>{zh.editorialHistoryEmpty}</p>}
      {entries.map(entry => <article key={entry.commandId}><strong>{new Date(entry.createdAt).toLocaleString()}</strong><p>{entry.nodeTypes.map(type => zh[`editorialNode_${type}`]).join(' · ')} · {entry.changeCount} {zh.editorialHistoryChanges} · {entry.undone ? zh.editorialHistoryUndone : entry.compensated ? zh.editorialHistoryReverted : entry.isCompensation ? zh.editorialHistoryCompensation : zh.editorialDecision_accepted}</p>
        {!entry.undone && !entry.compensated && !entry.isCompensation && <Button size="small" disabled={busy} onClick={() => { setError(''); setPreview(undefined); setSelected(entry.commandId); setPreviewRequest(v => v + 1); }}>{zh.editorialPreviewRevert}</Button>}
      </article>)}
      {hasMore && <Button size="small" disabled={busy} onClick={() => {
        if (!repository.editorialHistory || !entries.length || lock.current) return;
        lock.current = true; setBusy(true);
        void repository.editorialHistory(projectId, boardId, entries[entries.length - 1].cursor).then(values => { setEntries(old => [...old, ...values]); setHasMore(values.length === 100); }).catch(error => setError(userFacingError(error, zh.editorialHistoryFailed))).finally(() => { lock.current = false; setBusy(false); });
      }}>{zh.editorialHistoryMore}</Button>}
    </div><div className="editorial-history-preview">
      {preview?.mergeCommandId === selected && <><CanvasComparisonViewport autoFit labels={{ baseline: zh.editorialCurrent, draft: zh.editorialRevertResult }} mode="split" baseline={preview.currentNodes} draft={preview.mergedNodes} merged={preview.mergedNodes} changes={[]} onFocus={() => {}} />
        {preview.conflicts.map((message, i) => <p role="alert" key={i}>{message}</p>)}
        {!!preview.restoredCommentCount && <p>{zh.editorialRestoredComments.replace('{count}',String(preview.restoredCommentCount))}</p>}
        <Button variant="primary" disabled={busy || readOnly || !preview.canApply} onClick={() => void confirm()}>{zh.editorialConfirmRevert}</Button>
      </>}
    </div></div>
    {error && <p role="alert">{error}</p>}
  </Modal>;
}
