import type { HTMLAttributes, PointerEvent } from 'react';
import type { CanvasPhoto } from '../../domain/canvas';
import { zh } from '../../app/copy';

function content(raw?: string | null): Record<string, unknown> {
  try { return JSON.parse(raw ?? '{}'); } catch { return {}; }
}
/** 正式画布与审稿共用节点展示；差异标记始终位于照片像素之外。 */
export function CanvasNodes({ nodes, selected = new Set(), dragging, editable = false, onNodeClick, onResize, onKeyboardSelect, photoNames = {} }: {
  nodes: CanvasPhoto[]; selected?: Set<string>; dragging?: string | null; editable?: boolean;
  onNodeClick?: (id: string) => void; onResize?: (node: CanvasPhoto, event: PointerEvent<HTMLDivElement>) => void;
  onKeyboardSelect?: (id:string,extend:boolean)=>void; photoNames?:Record<string,string>;
}) {
  const keyboard=(node:CanvasPhoto,label:string):HTMLAttributes<HTMLDivElement>=>({tabIndex:editable?0:undefined,role:editable?'button':undefined,'aria-label':label,'aria-pressed':editable?selected.has(node.id):undefined,onKeyDown:event=>{if(editable&&onKeyboardSelect&&(event.key==='Enter'||event.key===' ')){event.preventDefault();event.stopPropagation();onKeyboardSelect(node.id,event.shiftKey);}}});
  return <>{nodes.filter(node => node.visible !== false).map((node,index) => {
    const style = { left: node.x, top: node.y, width: node.width, height: node.height, zIndex: node.zOrder, transform: node.rotation ? `rotate(${node.rotation}deg)` : undefined };
    const state = `${dragging === node.id ? ' dragging' : ''}${selected.has(node.id) ? ' selected' : ''}`;
    if (node.nodeType === 'photo') return <div key={node.id} {...keyboard(node,photoNames[node.photoId??node.assetId??'']??zh.canvasPhotoItem.replace('{number}',String(index+1)))} data-node-id={node.id} className={`canvas-photo-node${state}`} style={style} onClick={() => onNodeClick?.(node.id)}>
      {node.thumbnailPath ? <img src={node.thumbnailPath} alt="" draggable={false} /> : <div className="canvas-photo-placeholder">📷</div>}
      {!node.locked && editable && <div className="canvas-resize-handle" onPointerDown={event => onResize?.(node, event)} />}
      {node.locked && <span className="canvas-lock-badge">🔒</span>}
    </div>;
    if (['sticky', 'text', 'shape', 'chapter'].includes(node.nodeType)) return <div key={node.id} {...keyboard(node,String(content(node.contentJson).text??zh.canvasNoteTool))} data-node-id={node.id} className={`canvas-sticky-node${state}`} style={{ ...style, background: String(content(node.styleJson).color ?? '#ffe082') }} onClick={() => onNodeClick?.(node.id)}>{String(content(node.contentJson).text ?? '')}</div>;
    if (node.nodeType === 'group') return <div key={node.id} {...keyboard(node,String(content(node.contentJson).label??zh.canvasGroupTool))} data-node-id={node.id} className={`canvas-group-node${state}`} style={{ ...style, zIndex: -1 }}><span className="canvas-group-label">{String(content(node.contentJson).label ?? '')}</span></div>;
    let path: React.ReactNode;
    if (node.nodeType === 'ink') {
      const points = content(node.contentJson).points as Array<{ x: number; y: number }> | undefined;
      if (!Array.isArray(points) || points.length < 2) return null;
      path = <path d={points.map((p, i) => `${i ? 'L' : 'M'} ${p.x} ${p.y}`).join(' ')} fill="none" stroke={String(content(node.styleJson).color ?? '#ff6b6b')} strokeWidth={Number(content(node.styleJson).strokeWidth ?? 2)} strokeLinecap="round" strokeLinejoin="round" />;
    } else if (node.nodeType === 'arrow') {
      let ids: string[]; try { ids = JSON.parse(node.childrenJson ?? '[]'); } catch { return null; }
      if (ids.length !== 2) return null;
      const from = nodes.find(n => n.id === ids[0]), to = nodes.find(n => n.id === ids[1]);
      if (!from || !to) return null;
      path = <><defs><marker id={`ah-${node.id}`} markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="#6c5ce7" /></marker></defs><line x1={from.x + from.width / 2} y1={from.y + from.height / 2} x2={to.x + to.width / 2} y2={to.y + to.height / 2} stroke="#6c5ce7" strokeWidth={2} markerEnd={`url(#ah-${node.id})`} /></>;
    } else return null;
    return <svg key={node.id} style={{ position: 'absolute', left: 0, top: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: node.zOrder, overflow: 'visible' }}>{path}</svg>;
  })}</>;
}
