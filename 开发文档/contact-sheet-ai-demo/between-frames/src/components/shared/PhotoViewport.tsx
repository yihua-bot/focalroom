import { useEffect, useRef, useState } from 'react';
import type { InspectionImage, Photo } from '../../domain/models';
import { Button } from './Button';

export interface PhotoZoom { mode: 'fit' | 'actual'; scale: number; x: number; y: number }
export const fitZoom: PhotoZoom = { mode: 'fit', scale: 1, x: 0, y: 0 };
const clamp = (value: number, limit: number) => Math.max(-limit, Math.min(limit, value));

export function ZoomControls({ zoom, onChange, label, original = false }: { zoom: PhotoZoom; onChange: (zoom: PhotoZoom) => void; label: string; original?: boolean }) {
  const scale = (value: number) => onChange({ mode: 'actual', scale: Math.max(.1, Math.min(8, value)), x: 0, y: 0 });
  return <div className="zoom-controls" role="group" aria-label={`${label}缩放`}>
    <Button size="small" aria-label={`${label}适合屏幕`} aria-pressed={zoom.mode === 'fit'} onClick={() => onChange(fitZoom)}>适合屏幕</Button>
    <Button size="small" aria-label={`${label}100% ${original ? '像素' : '预览'}`} aria-pressed={zoom.mode === 'actual' && zoom.scale === 1} onClick={() => scale(1)}>100% {original ? '像素' : '预览'}</Button>
    <Button size="small" aria-label={`${label}缩小`} onClick={() => scale(zoom.scale / 1.5)}>−</Button>
    <Button size="small" aria-label={`${label}放大`} onClick={() => scale(zoom.scale * 1.5)}>+</Button>
    <span className="micro">{zoom.mode === 'fit' ? '完整构图' : `${Math.round(zoom.scale * 100)}%`}</span>
  </div>;
}

export function PhotoViewport({ photo, zoom, onChange }: { photo: Photo; zoom: PhotoZoom; onChange: (zoom: PhotoZoom) => void }) {
  const root = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [loaded, setLoaded] = useState({ src: '', width: 0, height: 0 });
  const [failed, setFailed] = useState(false);
  const [original, setOriginal] = useState<Partial<InspectionImage> & { error?: string }>({});
  const [pixelRatio, setPixelRatio] = useState(window.devicePixelRatio || 1);
  const loadOriginal = photo.loadOriginal;
  useEffect(() => {
    if (zoom.mode !== 'actual' || !loadOriginal) return;
    let active = true;
    loadOriginal().then(({ src, kind }) => { if (active) setOriginal({ src, kind }); }, error => {
      if (active) setOriginal({ error: error instanceof Error ? error.message : '原图不可用；当前显示缓存预览' });
    });
    return () => { active = false; };
  }, [zoom.mode, loadOriginal]);
  useEffect(() => {
    const resize = () => setPixelRatio(window.devicePixelRatio || 1);
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  }, []);
  const originalVisible = zoom.mode === 'actual' && Boolean(original.src);
  const sourceLabel = original.kind === 'embedded_preview' ? 'RAW 内嵌 JPEG 预览' : original.kind === 'derived_preview' ? 'RAW 派生预览' : original.kind === 'display_managed' ? '原图 · 显示器色彩' : '原图';
  const src = originalVisible ? original.src! : photo.previewSrc ?? photo.src;
  const natural = loaded.src === src ? loaded : { width: 0, height: 0 };
  const waitingOriginal = zoom.mode === 'actual' && loadOriginal && !original.src && !original.error;
  const drag = useRef<{ pointerId: number; left: number; top: number; x: number; y: number } | null>(null);
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setSize({ width: element.clientWidth, height: element.clientHeight }));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  // 原图按物理屏幕像素检视；缓存与适合屏幕按 CSS 像素布局。
  const scale = (zoom.mode === 'fit' || waitingOriginal) && natural.width && natural.height ? Math.min(size.width / natural.width, size.height / natural.height) : zoom.scale / (originalVisible ? pixelRatio : 1);
  const width = natural.width * scale;
  const height = natural.height * scale;
  const limitX = width ? Math.max(0, (width - size.width) / (2 * width)) : 0;
  const limitY = height ? Math.max(0, (height - size.height) / (2 * height)) : 0;
  const x = clamp(zoom.x, limitX);
  const y = clamp(zoom.y, limitY);
  const pannable = Boolean(limitX || limitY);
  return <div ref={root} className={`photo-viewport ${pannable ? 'pannable' : ''}`} data-testid="photo-viewport" data-mode={zoom.mode} data-source={originalVisible ? original.kind?.replace('_', '-') : 'preview'} data-scale={zoom.scale} data-pan={`${zoom.x},${zoom.y}`}
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
    <img key={src} src={src} alt={photo.name} draggable={false} onLoad={event => { setFailed(false); setLoaded({ src, width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight }); }} onError={() => {
      if (originalVisible) setOriginal({ error: `${original.kind === 'embedded_preview' ? '内嵌 JPEG' : '原图'}无法解码；当前显示缓存预览` });
      else setFailed(true);
    }}
      style={{ width, height, opacity: natural.width ? 1 : 0, transform: `translate(calc(-50% + ${x * width}px), calc(-50% + ${y * height}px))` }} />
    {originalVisible && original.kind !== 'original' && <span className="original-badge" role="note">{sourceLabel} · {original.kind === 'embedded_preview' ? '非 RAW 全分辨率' : original.kind === 'display_managed' ? '全分辨率 · 已按显示器 ICC 渲染' : '非全分辨率'}</span>}
    {failed && <p role="alert">预览无法加载，请重新打开项目检查缓存。</p>}
    {waitingOriginal && <p role="status">正在校验并加载原图…</p>}
    {zoom.mode === 'actual' && original.error && <p role="alert">{original.error}</p>}
    {originalVisible && <small className="inspection-source">{sourceLabel} · {Math.round(zoom.scale * 100)}% · 1 图像像素 = {zoom.scale} 屏幕像素；色彩由系统显示管理</small>}
    {!natural.width && !failed && !waitingOriginal && <p role="status">{originalVisible ? '正在加载原图…' : '正在加载预览…'}</p>}
  </div>;
}
