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
    <img src={photo.previewSrc ?? photo.src} alt={`当前选中：${photo.name}`} /><Maximize2 size={16} /></button>
    <div className="micro">FRAME {String(photo.index).padStart(2, '0')}</div>
    <h3>{photo.name}</h3><div className={`status-label ${photo.status}`}>{String(zh['status.' + photo.status] ?? photo.status)}</div>
    {photo.isMissing && <div className="missing-notice"><p>原图失联，缓存预览仍可查看。</p><Button size="small" disabled={busy} onClick={onRelink}>重新定位原图</Button></div>}
    {photo.width && <p className="micro">{photo.width} × {photo.height}</p>}
    <div className="rating-control" role="group" aria-label="人工星级"><span className="micro">你的评分</span>{[1, 2, 3, 4, 5].map(rating => <button key={rating} disabled={busy} aria-label={`人工评分 ${rating} 星`} aria-pressed={photo.rating === rating} onClick={() => onRate(photo.id, rating)} className={photo.rating >= rating ? 'filled' : ''}>★</button>)}<button disabled={busy || !photo.rating} onClick={() => onRate(photo.id, 0)} aria-label="清除人工评分">清除</button></div>
    {onBookMark && <div className="rating-control" role="group" aria-label="册页标记"><span className="micro">册页（印刷）</span>
      <button disabled={busy} aria-pressed={photo.bookMark === 'spread'} onClick={() => onBookMark(photo.id, photo.bookMark === 'spread' ? null : 'spread')}>跨页大图</button>
      <button disabled={busy} aria-pressed={photo.bookMark === 'full'} onClick={() => onBookMark(photo.id, photo.bookMark === 'full' ? null : 'full')}>整页图</button>
      <span className="micro">{photo.bookMark ? '印刷导出时独占一页' : '默认每页两张'}</span>
    </div>}
    <div className="inspector-buttons"><Button disabled={busy} variant={photo.status === 'keep' ? 'primary' : 'default'} onClick={() => onDecide(photo.id, 'keep')}>◇ {zh.keep}</Button>
      <Button disabled={busy} onClick={() => onDecide(photo.id, 'reject')}>× {zh.reject}</Button>
      {photo.status !== 'undecided' && <Button size="small" variant="quiet" className="span-two" disabled={busy} onClick={() => onDecide(photo.id, 'undecided')}>{zh.undecided}</Button>}</div>
    <label htmlFor="photo-note" className="micro">{zh.note}</label><textarea id="photo-note" className="text-input" rows={4} maxLength={2000} placeholder={zh.notePlaceholder} value={note} onChange={event => { setLocalNote(event.target.value); onDraft?.(event.target.value); }} />
    <div className="dialog-actions"><Button size="small" disabled={busy || note === photo.note} onClick={() => onNote(photo.id, note.trim())}>{zh.saveNote}</Button></div>
  </section>;
}
