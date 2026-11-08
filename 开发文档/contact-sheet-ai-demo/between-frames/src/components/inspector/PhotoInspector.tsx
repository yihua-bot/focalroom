import { useRef, useState } from 'react';
import { Maximize2 } from 'lucide-react';
import type { Photo, PhotoStatus, PreviewRepairOutcome } from '../../domain/models';
import { previewRepairLabel } from '../review/previewRepair';
import { zh } from '../../app/copy';
import { Button } from '../shared/Button';

export function PhotoInspector({ photo, busy, onDecide, onRate, onNote, onBookMark, onEnlarge, onRelink, onRepairPreview, onInspectPreview, draft, onDraft }: {
  photo: Photo; busy: boolean; onDecide: (id: string, status: PhotoStatus) => void;
  onRate: (id: string, rating: number) => void;
  onBookMark?: (id: string, mark: 'spread' | 'full' | null) => void;
  onNote: (id: string, note: string) => void; onEnlarge: (photo: Photo) => void;
  onRelink?: () => void; onRepairPreview?: () => Promise<PreviewRepairOutcome[]>;
  onInspectPreview?: () => Promise<PreviewRepairOutcome>;
  draft?: string; onDraft?: (note: string) => void;
}) {
  const [localNote, setLocalNote] = useState(photo.note);
  const [repairOutcome, setRepairOutcome] = useState<string>();
  const [fileState, setFileState] = useState<string>();
  const [checking, setChecking] = useState(false);
  const [repairing, setRepairing] = useState(false);
  const lock = useRef(false);
  const note = draft ?? localNote;
  const inspect = async () => {
    if (!onInspectPreview || lock.current) return;
    lock.current = true; setChecking(true);
    try { setFileState((await onInspectPreview()).outcome); }
    catch { setFileState('failed'); }
    finally { lock.current = false; setChecking(false); }
  };
  const repair = async () => {
    if (!onRepairPreview || lock.current) return;
    lock.current = true; setRepairing(true);
    try { setRepairOutcome(previewRepairLabel((await onRepairPreview())[0]?.outcome)); }
    catch { setRepairOutcome(previewRepairLabel()); }
    // 修完再体检一次：状态行说的是现在读不读得出来，动手之前那一遍的结论不能留着冒充现状。
    try { if (onInspectPreview) setFileState((await onInspectPreview()).outcome); }
    catch { setFileState('failed'); }
    finally { lock.current = false; setRepairing(false); }
  };
  const canRepair = !!onRepairPreview && (!onInspectPreview || fileState === 'needs_repair');
  const fileStateLine = fileState === 'healthy' ? zh.inspectorPreviewHealthy
    : fileState === 'needs_repair' ? zh.inspectorPreviewBroken : previewRepairLabel(fileState);
  return <section aria-label={zh.inspector}><button className="inspector-preview" aria-label={zh.enlarge} onClick={() => onEnlarge(photo)}>
    <img src={photo.previewSrc ?? photo.src} alt={zh.currentPhotoAlt.replace('{name}', photo.name)} onError={() => { void inspect(); }} /><Maximize2 size={16} /></button>
    <div className="micro">FRAME {String(photo.index).padStart(2, '0')}</div>
    <h3>{photo.name}</h3><div className={`status-label ${photo.status}`}>{String(zh['status.' + photo.status] ?? photo.status)}</div>
    {photo.isMissing && <div className="missing-notice"><p>{zh.missingPhotoNotice}</p><Button size="small" disabled={busy} onClick={onRelink}>{zh.relinkOriginal}</Button></div>}
    {(onInspectPreview || canRepair) && <div className="chat-preview-repair"><p className="micro">{zh.chatRepairPreviewHint}</p>
      {onInspectPreview && <Button size="small" disabled={busy || checking || repairing} onClick={() => { void inspect(); }}>{zh.inspectorPreviewCheck}</Button>}
      {fileState && <small role="status">{fileStateLine}</small>}
      {canRepair && <Button size="small" disabled={busy || repairing || checking} onClick={() => { void repair(); }}>{zh.chatRepairPreview}</Button>}
      {repairOutcome && <small role="status">{repairOutcome}</small>}</div>}
    {photo.width && <p className="micro">{photo.width} × {photo.height}</p>}
    <div className="rating-control" role="group" aria-label={zh.manualStarsAria}><span className="micro">{zh.yourRating}</span>{[1, 2, 3, 4, 5].map(rating => <button key={rating} disabled={busy} aria-label={zh.manualRateStarsAria.replace('{count}', String(rating))} aria-pressed={photo.rating === rating} onClick={() => onRate(photo.id, rating)} className={photo.rating >= rating ? 'filled' : ''}>★</button>)}<button disabled={busy || !photo.rating} onClick={() => onRate(photo.id, 0)} aria-label={zh.clearManualRating}>{zh.clearShort}</button></div>
    {onBookMark && <div className="rating-control" role="group" aria-label={zh.bookMarkGroup}><span className="micro">{zh.bookMarkLabel}</span>
      <button disabled={busy} aria-pressed={photo.bookMark === 'spread'} onClick={() => onBookMark(photo.id, photo.bookMark === 'spread' ? null : 'spread')}>{zh.spreadMark}</button>
      <button disabled={busy} aria-pressed={photo.bookMark === 'full'} onClick={() => onBookMark(photo.id, photo.bookMark === 'full' ? null : 'full')}>{zh.fullPageMark}</button>
      <span className="micro">{photo.bookMark ? zh.spreadHint : zh.defaultLayoutHint}</span>
    </div>}
    <div className="inspector-buttons"><Button disabled={busy} variant={photo.status === 'keep' ? 'primary' : 'default'} onClick={() => onDecide(photo.id, 'keep')}>◇ {zh.keep}</Button>
      <Button disabled={busy} onClick={() => onDecide(photo.id, 'reject')}>× {zh.reject}</Button>
      {photo.status !== 'undecided' && <Button size="small" variant="quiet" className="span-two" disabled={busy} onClick={() => onDecide(photo.id, 'undecided')}>{zh.undecided}</Button>}</div>
    <label htmlFor="photo-note" className="micro">{zh.note}</label><textarea id="photo-note" className="text-input" rows={4} maxLength={2000} placeholder={zh.notePlaceholder} value={note} onChange={event => { setLocalNote(event.target.value); onDraft?.(event.target.value); }} />
    <div className="dialog-actions"><Button size="small" disabled={busy || note === photo.note} onClick={() => onNote(photo.id, note.trim())}>{zh.saveNote}</Button></div>
  </section>;
}
