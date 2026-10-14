import { useRef, useState } from 'react';
import type { Photo, PhotoStatus } from '../../domain/models';
import { Modal } from '../shared/Modal';
import { Button } from '../shared/Button';
import { PhotoViewport, ZoomControls, fitZoom } from '../shared/PhotoViewport';

const statusLabels: Record<PhotoStatus, string> = { keep: '已保留', reject: '已淘汰', undecided: '待定' };

// v1.1 版式增强（ADR 0009 增补）：沉浸审片——灯箱内直接决策与评分，
// 底部胶片条带状态标记；决策走与网格一致的正式命令通道。
export function Lightbox({ photos, activeId, onNavigate, onClose, onDecide, onRate }: {
  photos: Photo[]; activeId: string; onNavigate: (photo: Photo) => void; onClose: () => void;
  onDecide?: (id: string, status: PhotoStatus) => void; onRate?: (id: string, rating: number) => void;
}) {
  const [zoom, setZoom] = useState(fitZoom);
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
  return <Modal title={photo.name} description="沉浸审片：← / → 翻片，K 保留、X 淘汰、U 待定、0–5 评分，Space / Esc 关闭；底部胶片条可跳转。" onClose={onClose} wide initialFocusRef={focus}>
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
      <div className="lightbox-toolbar"><ZoomControls zoom={zoom} onChange={setZoom} label="大图" />
        <div className="review-status" role="status" data-status={photo.status}>
          <strong>{statusLabels[photo.status]}</strong>
          <span className="micro">{'★'.repeat(photo.rating)}{'☆'.repeat(5 - photo.rating)}</span>
          <div className="review-actions">
            <Button size="small" aria-pressed={photo.status === 'keep'} onClick={() => decide('keep')}>保留 K</Button>
            <Button size="small" aria-pressed={photo.status === 'reject'} onClick={() => decide('reject')}>淘汰 X</Button>
            <Button size="small" aria-pressed={photo.status === 'undecided'} onClick={() => decide('undecided')}>待定 U</Button>
            {[1, 2, 3, 4, 5].map(star => <Button key={star} size="small" aria-label={`评 ${star} 星`} aria-pressed={photo.rating === star} onClick={() => rate(star)}>{star}</Button>)}
          </div>
        </div>
        <div className="lightbox-nav">
          <Button size="small" aria-label="上一张照片" disabled={index <= 0} onClick={() => move(-1)}>←</Button><span className="micro">{index + 1} / {photos.length}</span>
          <Button size="small" aria-label="下一张照片" disabled={index >= photos.length - 1} onClick={() => move(1)}>→</Button></div></div>
      <PhotoViewport key={photo.id} photo={photo} zoom={zoom} onChange={setZoom} />
      <div className="review-filmstrip" aria-label="胶片条导航">{photos.map((item, position) => <button
        key={item.id} aria-label={`第 ${position + 1} 张 ${item.name}（${statusLabels[item.status]}）`} aria-current={item.id === photo.id}
        data-status={item.status} onClick={() => { setZoom(fitZoom); onNavigate(item); }}>
        <img src={item.src} alt={item.name} />
        {item.rating > 0 && <span className="filmstrip-rating">{item.rating}</span>}
      </button>)}</div>
    </div>
  </Modal>;
}
