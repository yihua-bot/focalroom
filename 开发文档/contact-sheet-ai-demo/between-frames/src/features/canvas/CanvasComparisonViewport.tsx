import { useEffect, useRef, useState } from 'react';
import type { CanvasPhoto, EditorialChange } from '../../domain/canvas';
import { CanvasNodes } from './CanvasNodes';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

export function CanvasComparisonViewport({ mode, baseline, draft, merged, change, changes, onFocus, nodeIds }: {
  mode: 'merged' | 'split' | 'overlay'; baseline: CanvasPhoto[]; draft: CanvasPhoto[]; merged?: CanvasPhoto[];
  change?: EditorialChange; changes: EditorialChange[]; onFocus: (id: string) => void;
  nodeIds?: Record<string, string>;
}) {
  const [view, setView] = useState({ x: 32, y: 48, zoom: .6 });
  const surface = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; vx: number; vy: number } | null>(null);
  const focusNode = change?.after ?? change?.before;
  useEffect(() => {
    if (!focusNode) return;
    const timer = window.setTimeout(() => {
      const box = surface.current?.getBoundingClientRect();
      if (!box?.width) return;
      setView(v => ({ ...v, x: -(focusNode.x + focusNode.width / 2) * v.zoom, y: -(focusNode.y + focusNode.height / 2) * v.zoom }));
    }, 0);
    return () => window.clearTimeout(timer);
  }, [focusNode]);
  const fit = (nodes: CanvasPhoto[]) => {
    const box = surface.current?.getBoundingClientRect();
    const visible = nodes.filter(n => n.nodeType !== 'arrow' && n.visible !== false);
    if (!box || !visible.length) return;
    const x = Math.min(...visible.map(n => n.x)), y = Math.min(...visible.map(n => n.y));
    const right = Math.max(...visible.map(n => n.x + n.width)), bottom = Math.max(...visible.map(n => n.y + n.height));
    const width = box.width / (mode === 'split' ? 2 : 1);
    const zoom = Math.max(.001, Math.min(2, (width - 64) / Math.max(1, right - x), (box.height - 64) / Math.max(1, bottom - y)));
    setView({ x: -(x + right) / 2 * zoom, y: -(y + bottom) / 2 * zoom, zoom });
  };
  const focus = () => fit([change?.after ?? change?.before].filter((n): n is CanvasPhoto => Boolean(n)));
  const panels = mode === 'split' ? [{ label: zh.editorialOriginal, nodes: baseline }, { label: zh.editorialDraft, nodes: draft }] : [{ label: mode === 'merged' ? zh.editorialMode_merged : zh.editorialMode_overlay, nodes: mode === 'merged' ? merged ?? baseline : draft }];
  return <section className="editorial-viewport" aria-label={zh.editorialPreview}>
    <div className="editorial-viewport-tools"><Button size="small" onClick={() => fit([...baseline, ...draft])}>{zh.editorialFit}</Button><Button size="small" disabled={!change} onClick={focus}>{zh.editorialLocate}</Button><output aria-label={zh.editorialZoom}>{Math.round(view.zoom * 100)}%</output></div>
    <div className="editorial-preview-panels" ref={surface} tabIndex={0} onKeyDown={event => { if (event.shiftKey && event.key === '1') { event.preventDefault(); fit([...baseline, ...draft]); } }}>
      {panels.map(panel => <div className="editorial-preview-pane" key={panel.label} onWheel={event => {
        event.preventDefault(); const box = event.currentTarget.getBoundingClientRect();
        const zoom = Math.min(4, Math.max(.001, view.zoom * (event.deltaY < 0 ? 1.1 : .9)));
        const x = event.clientX - box.left - box.width / 2, y = event.clientY - box.top - box.height / 2;
        setView({ x: x - (x - view.x) / view.zoom * zoom, y: y - (y - view.y) / view.zoom * zoom, zoom });
      }} onPointerDown={event => { if (event.button !== 0) return; drag.current = { x: event.clientX, y: event.clientY, vx: view.x, vy: view.y }; event.currentTarget.setPointerCapture(event.pointerId); }} onPointerMove={event => {
        if (drag.current) setView(v => ({ ...v, x: drag.current!.vx + event.clientX - drag.current!.x, y: drag.current!.vy + event.clientY - drag.current!.y }));
      }} onPointerUp={() => { drag.current = null; }} onPointerCancel={() => { drag.current = null; }}>
        <span className="editorial-pane-label">{panel.label}</span>
        <div className="canvas-board" data-testid="editorial-transform" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})`, transformOrigin: '0 0' }}>
          <CanvasNodes nodes={panel.nodes} />
          {mode === 'overlay' && changes.map(c => {
            const before = c.before, after = c.after;
            return <div key={c.id} className="editorial-overlay-markers">
              {before && <div className="editorial-before-outline" style={{ left: before.x, top: before.y, width: before.width, height: before.height }} />}
              {before && after && (before.x !== after.x || before.y !== after.y) && <svg className="editorial-move-path"><line x1={before.x} y1={before.y} x2={after.x} y2={after.y} stroke="currentColor" strokeWidth="2" strokeDasharray="8 5" /></svg>}
              {(after ?? before) && <button type="button" className="editorial-diff-tag" onClick={() => onFocus(c.id)} style={{ left: (after ?? before)!.x, top: (after ?? before)!.y - 28 }}>{(c.kinds ?? [c.kind]).map(kind => zh[`editorialKind_${kind}`]).join(' · ')}</button>}
            </div>;
          })}
          {change && (() => { const n = panel.nodes.find(n => n.id === change.nodeId || n.id === change.sourceNodeId || n.id === nodeIds?.[change.nodeId]); return n ? <div className="editorial-focus-outline" style={{ left: n.x, top: n.y, width: n.width, height: n.height }} /> : null; })()}
        </div>
      </div>)}
    </div>
  </section>;
}
