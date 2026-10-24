import { useRef, useState } from 'react';
import type { DesktopProjectRepository, Project } from '../../domain/models';
import type { SelectionDraft } from '../../domain/agent';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { SelectionRevisionContext } from './SelectionRevisionContext';
import { zh } from '../../app/copy';
import { userFacingError } from '../../services/userFacingError';

export function DraftReview({ draft, project, repository, onClose, onUpdated }: { draft: SelectionDraft; project: Project; repository: DesktopProjectRepository; onClose: () => void; onUpdated: (project: Project) => void }) {
 const [selected, setSelected] = useState<string[]>(draft.items.map(item => item.photo_id));
 const [busy, setBusy] = useState(false);
 const [error, setError] = useState('');
 const [confirmFull, setConfirmFull] = useState(false);
 const lock = useRef(false);
 const proposed = draft.items.filter(item => item.proposed_action === 'keep').map(item => item.photo_id);
 const added = proposed.filter(id => !project.sequence.includes(id)).length;
 const removed = project.sequence.filter(id => !proposed.includes(id)).length;
 const moved = proposed.filter((id, pos) => project.sequence.includes(id) && project.sequence.indexOf(id) !== pos).length;
 const stale = draft.stale || draft.baseRevision !== project.revision;
 const run = async (task: () => Promise<Project>, close = false) => {
  if (lock.current) return; lock.current=true; setBusy(true); setError('');
  try { const updated = await task(); onUpdated(updated); if (close) onClose(); }
  catch (error) { setError(userFacingError(error)); }
  finally { lock.current=false; setBusy(false); }
 };
 return <Modal title={draft.title} onClose={onClose} wide className="draft-review-dialog">
  <p>{draft.summary}</p>
  <div className="draft-diff" role="status">{stale && <strong>{zh.selectionStale}</strong>}<p>{zh.selectionDifference.replace('{added}',String(added)).replace('{removed}',String(removed)).replace('{moved}',String(moved))}</p></div>
  {stale && draft.state === 'pending' && <Button disabled={busy} onClick={() => { void run(() => repository.rebaseDraft(project.id, draft)); }}>{zh.selectionRecompare}</Button>}
  {draft.state === 'pending' && <div className="draft-select-tools"><Button size="small" disabled={busy} onClick={() => setSelected(draft.items.map(item => item.photo_id))}>{zh.selectionSelectAll}</Button><Button size="small" disabled={busy} onClick={() => setSelected([])}>{zh.selectionClear}</Button><span>{zh.selectionSelected} {selected.length} / {draft.items.length}</span></div>}
  <ol className="draft-items">{draft.items.map((item, index) => {
   const photo = project.photos.find(photo => photo.id === item.photo_id);
   return <li key={item.photo_id}><label>{draft.state==='pending' && <input type="checkbox" aria-label={zh.selectionAcceptPhoto+' '+(photo?.name ?? zh.selectionPhotoUnavailable)} checked={selected.includes(item.photo_id)} disabled={busy || draft.state !== 'pending'} onChange={event => setSelected(previous => event.target.checked ? [...previous, item.photo_id] : previous.filter(id => id !== item.photo_id))} />}
    {photo && <img src={photo.src} alt={photo.name} />}<span><strong>{index + 1}. {photo?.name ?? zh.selectionPhotoUnavailable}</strong><small>{zh[`selectionProposed_${item.proposed_action}`]} · {zh.selectionCurrent} {zh[`selectionStatus_${photo?.status ?? 'undecided'}`]}</small><small>{item.rationale}</small></span></label>
    <div><Button size="small" disabled={busy || draft.state !== 'pending' || index === 0} aria-label={zh.selectionMoveUp+' '+photo?.name} onClick={() => { const ids = draft.items.map(i => i.photo_id); [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]]; void run(() => repository.updateDraft(project.id, draft, ids)); }}>↑</Button>
    <Button size="small" disabled={busy || draft.state !== 'pending' || index === draft.items.length - 1} aria-label={zh.selectionMoveDown+' '+photo?.name} onClick={() => { const ids = draft.items.map(i => i.photo_id); [ids[index], ids[index + 1]] = [ids[index + 1], ids[index]]; void run(() => repository.updateDraft(project.id, draft, ids)); }}>↓</Button></div>
   </li>;
  })}</ol>
  <SelectionRevisionContext repository={repository.agent} projectId={project.id} draftId={draft.id} />
  {error && <p role="alert">{error}</p>}
  {draft.state !== 'pending' ? <p>{zh[draft.state==='applied'?'selectionApplied':'selectionDiscarded']}</p> : <div className="dialog-actions">
   <Button disabled={busy} onClick={() => { void run(() => repository.discardDraft(project.id, draft), true); }}>{zh.selectionDiscard}</Button>
   <Button disabled={busy || stale || selected.length === 0} variant="primary" onClick={() => { void run(() => repository.applyDraft(project.id, draft, selected, 'partial'), true); }}>{zh.selectionAcceptSelected.replace('{count}',String(selected.length))}</Button>
   <Button disabled={busy || stale} onClick={() => setConfirmFull(true)}>{zh.selectionReplace}</Button>
  </div>}
  {confirmFull && <div className="draft-full-confirm"><p>{zh.selectionReplaceHint.replace('{count}',String(draft.items.length))}</p><Button disabled={busy} onClick={() => setConfirmFull(false)}>{zh.selectionBack}</Button><Button disabled={busy || stale} variant="primary" onClick={() => { void run(() => repository.applyDraft(project.id, draft, draft.items.map(item => item.photo_id), 'full'), true); }}>{zh.selectionConfirmReplace}</Button></div>}
 </Modal>;
}
