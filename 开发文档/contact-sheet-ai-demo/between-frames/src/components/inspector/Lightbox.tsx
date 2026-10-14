import { useRef, useState } from 'react';
import type { Photo } from '../../domain/models';
import { Modal } from '../shared/Modal';
import { Button } from '../shared/Button';
import { PhotoViewport, ZoomControls, fitZoom } from '../shared/PhotoViewport';

export function Lightbox({ photos, activeId, onNavigate, onClose }: { photos: Photo[]; activeId: string; onNavigate: (photo: Photo) => void; onClose: () => void }) {
  const [zoom, setZoom] = useState(fitZoom);
  const focus = useRef<HTMLDivElement>(null);
  const index = photos.findIndex(photo => photo.id === activeId);
  const photo = photos[index];
  const move = (offset: number) => {
    const next = photos[index + offset];
    if (next) { setZoom(fitZoom); onNavigate(next); }
  };
  if (!photo) return null;
  return <Modal title={photo.name} description="完整构图 · 100% 显示缓存预览的像素尺寸；放大后可拖动画面。← / → 翻片，Space / Esc 关闭。" onClose={onClose} wide initialFocusRef={focus}>
    <div ref={focus} tabIndex={-1} className="lightbox-body" onKeyDown={event => {
      if (event.target instanceof HTMLElement && event.target.closest('input,textarea,select,[contenteditable="true"]')) return;
      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); move(event.key === 'ArrowLeft' ? -1 : 1); }
      if (event.key === ' ' && !(event.target instanceof HTMLElement && event.target.closest('button'))) { event.preventDefault(); onClose(); }
    }}>
      <div className="lightbox-toolbar"><ZoomControls zoom={zoom} onChange={setZoom} label="大图" /><div className="lightbox-nav">
        <Button size="small" aria-label="上一张照片" disabled={index <= 0} onClick={() => move(-1)}>←</Button><span className="micro">{index + 1} / {photos.length}</span>
        <Button size="small" aria-label="下一张照片" disabled={index >= photos.length - 1} onClick={() => move(1)}>→</Button></div></div>
      <PhotoViewport key={photo.id} photo={photo} zoom={zoom} onChange={setZoom} />
    </div>
  </Modal>;
}
