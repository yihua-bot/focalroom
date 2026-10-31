import type { PointerEvent } from 'react';
import type { CanvasPhoto } from '../../domain/canvas';

function content(raw?: string | null): Record<string, unknown> {
  try { return JSON.parse(raw ?? '{}'); } catch { return {}; }
}
/** 正式画布与审稿共用节点展示；差异标记始终位于照片像素之外。 */
export function CanvasNodes({ nodes, selected = new Set(), dragging, editable = false, linkSource, onNodeClick, onTextChange, onResize }: {
  nodes: CanvasPhoto[]; selected?: Set<string>; dragging?: string | null; editable?: boolean; linkSource?: string | null;
  onNodeClick?: (id: string) => void; onTextChange?: (node: CanvasPhoto) => void;
  onResize?: (node: CanvasPhoto, event: PointerEvent<HTMLDivElement>) => void;
}) {
  return <>{nodes.filter(node => node.visible !== false).map(node => {
    const style = { left: node.x, top: node.y, width: node.width, height: node.height, zIndex: node.zOrder, transform: node.rotation ? `rotate(${node.rotation}deg)` : undefined };
    const state = `${dragging === node.id ? ' dragging' : ''}${selected.has(node.id) ? ' selected' : ''}${linkSource === node.id ? ' link-source' : ''}`;
    if (node.nodeType === 'photo') return <div key={node.id} data-node-id={node.id} className={`canvas-photo-node${state}`} style={style} onClick={() => onNodeClick?.(node.id)}>
      {node.thumbnailPath ? <img src={node.thumbnailPath} alt="" draggable={false} /> : <div className="canvas-photo-placeholder">📷</div>}
      {!node.locked && editable && <div className="canvas-resize-handle" onPointerDown={event => onResize?.(node, event)} />}
      {node.locked && <span className="canvas-lock-badge">🔒</span>}
    </div>;
    if (['sticky', 'text', 'shape', 'chapter'].includes(node.nodeType)) return <div key={node.id} data-node-id={node.id}
      className={`canvas-sticky-node${state}${String(content(node.contentJson).text ?? '') ? '' : ' canvas-sticky-empty'}`}
      style={{ ...style, background: String(content(node.styleJson).color ?? '#ffe082') }}
      onDoubleClick={() => { if (editable && ['sticky', 'text', 'shape'].includes(node.nodeType)) onTextChange?.(node); }}
      onClick={() => onNodeClick?.(node.id)}>{String(content(node.contentJson).text ?? '')}</div>;
    if (node.nodeType === 'reference') {
      const reference = content(node.contentJson);
      return <div key={node.id} data-node-id={node.id} className={`canvas-reference-node${state}`} style={style} onClick={() => onNodeClick?.(node.id)}>
        <strong className="canvas-reference-title">{String(reference.title ?? '')}</strong>
        {reference.kind === 'link' && reference.url ? <a className="canvas-reference-url" href={String(reference.url)} target="_blank" rel="noreferrer" onClick={event => event.stopPropagation()}>{String(reference.url)}</a> : null}
        {reference.content ? <p className="canvas-reference-content">{String(reference.content)}</p> : null}
        <small className="canvas-reference-meta">
          {[reference.author, reference.source, reference.license].filter(Boolean).map(String).join(' · ') || '参考'}
        </small>
      </div>;
    }
    if (node.nodeType === 'group') return <div key={node.id} data-node-id={node.id} className={`canvas-group-node${state}`} style={{ ...style, zIndex: -1 }}><span className="canvas-group-label">{String(content(node.contentJson).label ?? '')}</span></div>;
    let path: React.ReactNode;
    let hit: React.ReactNode;
    if (node.nodeType === 'ink') {
      const points = content(node.contentJson).points as Array<{ x: number; y: number }> | undefined;
      if (!Array.isArray(points) || points.length < 2) return null;
      const d = points.map((p, i) => `${i ? 'L' : 'M'} ${p.x} ${p.y}`).join(' ');
      path = <path d={d} fill="none" stroke={String(content(node.styleJson).color ?? '#ff6b6b')} strokeWidth={Number(content(node.styleJson).strokeWidth ?? 2)} strokeLinecap="round" strokeLinejoin="round" />;
      // 墨迹只有 2px 宽，直接点几乎点不中；叠一条透明粗描边承载选中。
      hit = <path d={d} fill="none" stroke="transparent" strokeWidth={16} strokeLinecap="round" style={{ pointerEvents: 'stroke' }} data-node-id={node.id} />;
    } else if (node.nodeType === 'arrow') {
      let ids: string[]; try { ids = JSON.parse(node.childrenJson ?? '[]'); } catch { return null; }
      if (ids.length !== 2) return null;
      const from = nodes.find(n => n.id === ids[0]), to = nodes.find(n => n.id === ids[1]);
      if (!from || !to) return null;
      path = <><defs><marker id={`ah-${node.id}`} markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="#6c5ce7" /></marker></defs><line x1={from.x + from.width / 2} y1={from.y + from.height / 2} x2={to.x + to.width / 2} y2={to.y + to.height / 2} stroke={selected.has(node.id) ? '#a29bfe' : '#6c5ce7'} strokeWidth={selected.has(node.id) ? 3 : 2} markerEnd={`url(#ah-${node.id})`} /></>;
      hit = <line x1={from.x + from.width / 2} y1={from.y + from.height / 2} x2={to.x + to.width / 2} y2={to.y + to.height / 2} stroke="transparent" strokeWidth={16} style={{ pointerEvents: 'stroke' }} data-node-id={node.id} />;
    } else return null;
    return <svg key={node.id} style={{ position: 'absolute', left: 0, top: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: node.zOrder, overflow: 'visible' }}>{hit}{path}</svg>;
  })}</>;
}
