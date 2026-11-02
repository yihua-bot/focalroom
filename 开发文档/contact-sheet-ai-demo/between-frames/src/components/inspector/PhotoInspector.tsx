import { useState } from 'react';
import { Maximize2 } from 'lucide-react';
import type { Photo, PhotoStatus } from '../../domain/models';
import { zh } from '../../app/copy';
import { Button } from '../shared/Button';

export function PhotoInspector({ photo, busy, onDecide, onRate, onNote, onBookMark, onEnlarge, onRelink, draft, onDraft }: {
  photo: Photo; busy: boolean; onDecide: (id: string, status: PhotoStatus) => void;
  onRate: (id: string, rating: number) => void;
  onBookMark?: (id: string, mark: 'spread' | 'full' | null) => void;
  onNote: (id: string, note: string) => void; onEnlarge: (photo: Photo) => void;
  onRelink?: () => void; draft?: string; onDraft?: (note: string) => void;
}) {
  const [localNote, setLocalNote] = useState(photo.note);
  const note = draft ?? localNote;
  return <section aria-label={zh.inspector}><button className="inspector-preview" aria-label={zh.enlarge} onClick={() => onEnlarge(photo)}>
    <img src={photo.previewSrc ?? photo.src} alt={zh.currentPhotoAlt.replace('{name}', photo.name)} /><Maximize2 size={16} /></button>
    <div className="micro">FRAME {String(photo.index).padStart(2, '0')}</div>
    <h3>{photo.name}</h3><div className={`status-label ${photo.status}`}>{String(zh['status.' + photo.status] ?? photo.status)}</div>
    {photo.isMissing && <div className="missing-notice"><p>{zh.missingPhotoNotice}</p><Button size="small" disabled={busy} onClick={onRelink}>{zh.relinkOriginal}</Button></div>}
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
