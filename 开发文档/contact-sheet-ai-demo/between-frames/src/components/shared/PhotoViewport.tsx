import { useEffect, useRef, useState } from 'react';
import type { Photo } from '../../domain/models';
import { Button } from './Button';

export interface PhotoZoom { mode: 'fit' | 'actual'; scale: number; x: number; y: number }
export const fitZoom: PhotoZoom = { mode: 'fit', scale: 1, x: 0, y: 0 };
const clamp = (value: number, limit: number) => Math.max(-limit, Math.min(limit, value));

export function ZoomControls({ zoom, onChange, label }: { zoom: PhotoZoom; onChange: (zoom: PhotoZoom) => void; label: string }) {
  const scale = (value: number) => onChange({ mode: 'actual', scale: Math.max(.1, Math.min(8, value)), x: 0, y: 0 });
  return <div className="zoom-controls" role="group" aria-label={`${label}缩放`}>
    <Button size="small" aria-label={`${label}适合屏幕`} aria-pressed={zoom.mode === 'fit'} onClick={() => onChange(fitZoom)}>适合屏幕</Button>
    <Button size="small" aria-label={`${label}100% 预览`} aria-pressed={zoom.mode === 'actual' && zoom.scale === 1} onClick={() => scale(1)}>100% 预览</Button>
    <Button size="small" aria-label={`${label}缩小`} onClick={() => scale(zoom.scale / 1.5)}>−</Button>
    <Button size="small" aria-label={`${label}放大`} onClick={() => scale(zoom.scale * 1.5)}>+</Button>
    <span className="micro">{zoom.mode === 'fit' ? '完整构图' : `${Math.round(zoom.scale * 100)}%`}</span>
  </div>;
}

export function PhotoViewport({ photo, zoom, onChange }: { photo: Photo; zoom: PhotoZoom; onChange: (zoom: PhotoZoom) => void }) {
  const root = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [natural, setNatural] = useState({ width: 0, height: 0 });
  const [failed, setFailed] = useState(false);
  const drag = useRef<{ pointerId: number; left: number; top: number; x: number; y: number } | null>(null);
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setSize({ width: element.clientWidth, height: element.clientHeight }));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const scale = zoom.mode === 'fit' && natural.width && natural.height ? Math.min(size.width / natural.width, size.height / natural.height) : zoom.scale;
  const width = natural.width * scale;
  const height = natural.height * scale;
  const limitX = width ? Math.max(0, (width - size.width) / (2 * width)) : 0;
  const limitY = height ? Math.max(0, (height - size.height) / (2 * height)) : 0;
  const x = clamp(zoom.x, limitX);
  const y = clamp(zoom.y, limitY);
  const pannable = Boolean(limitX || limitY);
  return <div ref={root} className={`photo-viewport ${pannable ? 'pannable' : ''}`} data-testid="photo-viewport" data-mode={zoom.mode} data-scale={zoom.scale} data-pan={`${zoom.x},${zoom.y}`}
    onPointerDown={event => {
      if (!pannable || event.button !== 0) return;
      event.preventDefault();
      drag.current = { pointerId: event.pointerId, left: event.clientX, top: event.clientY, x, y };
      event.currentTarget.setPointerCapture(event.pointerId);
    }} onPointerMove={event => {
      const start = drag.current;
      if (!start || start.pointerId !== event.pointerId) return;
      onChange({ ...zoom, x: clamp(start.x + (event.clientX - start.left) / width, limitX), y: clamp(start.y + (event.clientY - start.top) / height, limitY) });
    }} onPointerUp={event => { if (drag.current?.pointerId === event.pointerId) drag.current = null; }} onPointerCancel={() => { drag.current = null; }} onLostPointerCapture={() => { drag.current = null; }}>
    <img src={photo.previewSrc ?? photo.src} alt={photo.name} draggable={false} onLoad={event => setNatural({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight })} onError={() => setFailed(true)}
      style={{ width, height, opacity: natural.width ? 1 : 0, transform: `translate(calc(-50% + ${x * width}px), calc(-50% + ${y * height}px))` }} />
    {failed && <p role="alert">预览无法加载，请重新打开项目检查缓存。</p>}
    {!natural.width && !failed && <p role="status">正在加载预览…</p>}
  </div>;
}
