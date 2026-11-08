import { useRef, useState } from 'react';
import type { Photo, PhotoStatus } from '../../domain/models';
import { Modal } from '../shared/Modal';
import { Button } from '../shared/Button';
import { ColorManagedToggle, PhotoViewport, ZoomControls, fitZoom, photoSourceKey, type ColorManaged } from '../shared/PhotoViewport';
import { zh } from '../../app/copy';

const STATUS_LABEL_KEYS: Record<PhotoStatus, string> = { keep: 'selectionStatus_keep', reject: 'selectionStatus_reject', undecided: 'photo.undecided' };

// v1.1 版式增强（ADR 0009 增补）：沉浸审片——灯箱内直接决策与评分，
// 底部胶片条带状态标记；决策走与网格一致的正式命令通道。
export function Lightbox({ photos, activeId, onNavigate, onClose, onDecide, onRate, projectId }: {
  photos: Photo[]; activeId: string; onNavigate: (photo: Photo) => void; onClose: () => void;
  onDecide?: (id: string, status: PhotoStatus) => void; onRate?: (id: string, rating: number) => void;
  projectId?: string;
}) {
  const [zoom, setZoom] = useState(fitZoom);
  // 色彩管理版本按照片与预览版本存：每张片的显示器渲染来自它自己的 ICC，换片即失效，不能沿用。
  const [managed, setManaged] = useState<(ColorManaged & { sourceKey: string }) | null>(null);
  const focus = useRef<HTMLDivElement>(null);
  const index = photos.findIndex(photo => photo.id === activeId);
  const photo = photos[index];
  const move = (offset: number) => {
    const next = photos[index + offset];
    if (next) { setZoom(fitZoom); onNavigate(next); }
  };
  const decide = (status: PhotoStatus) => { if (photo && onDecide) onDecide(photo.id, status); };
  const rate = (rating: number) => { if (photo && onRate) onRate(photo.id, rating); };
  if (!photo) return null;
  const managedForPhoto = managed?.sourceKey === photoSourceKey(photo) ? managed : null;
  return <Modal title={photo.name} description={zh.lightboxHelp} onClose={onClose} wide initialFocusRef={focus}>
    <div ref={focus} tabIndex={-1} className="lightbox-body" onKeyDown={event => {
      if (event.target instanceof HTMLElement && event.target.closest('input,textarea,select,[contenteditable="true"]')) return;
      const key = event.key.toLowerCase();
      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); move(event.key === 'ArrowLeft' ? -1 : 1); }
      else if (key === 'k') { event.preventDefault(); decide('keep'); }
      else if (key === 'x') { event.preventDefault(); decide('reject'); }
      else if (key === 'u') { event.preventDefault(); decide('undecided'); }
      else if (/^[0-5]$/.test(key)) { event.preventDefault(); rate(Number(key)); }
      else if (event.key === ' ' && !(event.target instanceof HTMLElement && event.target.closest('button'))) { event.preventDefault(); onClose(); }
    }}>
      <div className="lightbox-toolbar"><ZoomControls zoom={zoom} onChange={setZoom} label={zh.zoomLabel} original={Boolean(photo.loadOriginal)} />
        <ColorManagedToggle key={photoSourceKey(photo)} photo={photo} projectId={projectId} managed={managedForPhoto}
          onChange={next => setManaged(next ? { ...next, sourceKey: photoSourceKey(photo) } : null)} />
        <div className="review-status" role="status" data-status={photo.status}>
          <strong>{zh[STATUS_LABEL_KEYS[photo.status]]}</strong>
          <span className="micro">{'★'.repeat(photo.rating)}{'☆'.repeat(5 - photo.rating)}</span>
          <div className="review-actions">
            <Button size="small" aria-pressed={photo.status === 'keep'} onClick={() => decide('keep')}>{zh.keepWithKey}</Button>
            <Button size="small" aria-pressed={photo.status === 'reject'} onClick={() => decide('reject')}>{zh.rejectWithKey}</Button>
            <Button size="small" aria-pressed={photo.status === 'undecided'} onClick={() => decide('undecided')}>{zh.undecidedWithKey}</Button>
            {[1, 2, 3, 4, 5].map(star => <Button key={star} size="small" aria-label={zh['shortcutRate' + star]} aria-pressed={photo.rating === star} onClick={() => rate(star)}>{star}</Button>)}
          </div>
        </div>
        <div className="lightbox-nav">
          <Button size="small" aria-label={zh.previousPhoto} disabled={index <= 0} onClick={() => move(-1)}>←</Button><span className="micro">{index + 1} / {photos.length}</span>
          <Button size="small" aria-label={zh.nextPhoto} disabled={index >= photos.length - 1} onClick={() => move(1)}>→</Button></div></div>
      <PhotoViewport key={photoSourceKey(photo)} photo={photo} zoom={zoom} onChange={setZoom} managed={managedForPhoto} />
      <div className="review-filmstrip" aria-label={zh.filmstripNav}>{photos.map((item, position) => <button
        key={item.id} aria-label={zh.filmstripItem.replace('{index}', String(position + 1)).replace('{name}', item.name).replace('{status}', zh[STATUS_LABEL_KEYS[item.status]])} aria-current={item.id === photo.id}
        data-status={item.status} onClick={() => { setZoom(fitZoom); onNavigate(item); }}>
        <img loading="lazy" src={item.src} alt={item.name} />
        {item.rating > 0 && <span className="filmstrip-rating">{item.rating}</span>}
      </button>)}</div>
    </div>
  </Modal>;
}
