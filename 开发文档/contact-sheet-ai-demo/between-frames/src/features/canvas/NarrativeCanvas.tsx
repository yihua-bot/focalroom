// G3 Narrative Canvas：无限自由画布——照片节点拖拽放置。
// v3 蓝图 §03.4/§06：逻辑坐标独立于设备像素；照片引用资产不复制像素。
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/shared/Button';

import type { AnnotationInfo, BoardInfo, CanvasPhoto, CanvasRepository } from '../../domain/canvas';
/** 无限画布：拖拽平移 + 滚轮缩放 + 照片节点拖拽放置。 */
export function NarrativeCanvas({ repository, projectId, selectedPhotoIds, readOnly = false }: {
  repository?: CanvasRepository;
  projectId: string;
  selectedPhotoIds: string[];
  readOnly?: boolean;
}) {
  const [boards, setBoards] = useState<BoardInfo[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [nodes, setNodes] = useState<CanvasPhoto[]>([]);
  const [creating, setCreating] = useState(false);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [dragging, setDragging] = useState<string | null>(null);
  const [annotations, setAnnotations] = useState<AnnotationInfo[]>([]);
  const [linking, setLinking] = useState<string | null>(null);
  const [linkMode, setLinkMode] = useState(false);
  const [notice, setNotice] = useState('');
  const [working, setWorking] = useState(false);
  const [noteOpen, setNoteOpen] = useState(false);
  const [noteText, setNoteText] = useState('');
  const [inkMode, setInkMode] = useState(false);
  const [inkPoints, setInkPoints] = useState<Array<{x:number;y:number}>>([]);
  const dragStart = useRef<{ x: number; y: number; nodeX: number; nodeY: number } | null>(null);
  const panning = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const run = async (action: () => Promise<void>) => {
    if (readOnly || working) return;
    setWorking(true);
    setNotice('');
    try { await action(); }
    catch (error) { setNotice(error instanceof Error ? error.message : '画布操作失败'); }
    finally { setWorking(false); }
  };
  // 节点跟随 activeBoard 变化加载（异步回调内 setState 合法）
  useEffect(() => {
    let active = true;
    if (repository) {
      repository.listBoards(projectId).then(list => {
        if (!active) return;
        setBoards(list);
        setActiveBoard(prev => prev ?? (list.length > 0 ? list[0].id : null));
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取画布失败'); });
    }
    return () => { active = false; };
  }, [repository, projectId]);

  useEffect(() => {
    let active = true;
    if (repository && activeBoard) {
      repository.getNodes(activeBoard).then(ns => {
        if (active) setNodes(ns);
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取照片失败'); });
      repository.getAnnotations(activeBoard).then(ans => {
        if (active) setAnnotations(ans);
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取批注失败'); });
    }
    return () => { active = false; };
  }, [repository, activeBoard]);

  const loadBoards = async () => {
    if (!repository) return;
    const list = await repository.listBoards(projectId);
    setBoards(list);
    if (list.length > 0) setActiveBoard(prev => prev ?? list[0].id);
    if (activeBoard) {
      const ns = await repository.getNodes(activeBoard);
      setNodes(ns);
    }
  };

  const createBoard = async () => {
    if (!repository || creating) return;
    setCreating(true);
    try {
      const board = await repository.createBoard(projectId, `画布 ${boards.length + 1}`);
      const list = await repository.listBoards(projectId);
      setBoards(list);
      setNodes([]);
      setAnnotations([]);
      setActiveBoard(board.id);
      setPan({ x: 0, y: 0 });
      setZoom(1);
    } finally { setCreating(false); }
  };

  const placeSelected = async () => {
    if (!repository || !activeBoard || selectedPhotoIds.length === 0) return;
    const startX = -pan.x / zoom + 50;
    for (let i = 0; i < selectedPhotoIds.length; i++) {
      await repository.placePhoto(projectId, activeBoard, selectedPhotoIds[i],
        startX + i * 240, 50, 220, 160);
    }
    await loadBoards();
  };

  const addSticky = async () => {
    if (!repository || !activeBoard || !noteText.trim()) return;
    const x = -pan.x / zoom + 60;
    let y = -pan.y / zoom + 60;
    const overlaps = nodes.filter(node => node.nodeType !== 'arrow' &&
      node.x < x + 200 && node.x + node.width > x && node.y < y + 120 && node.y + node.height > y);
    if (overlaps.length) y = Math.max(...overlaps.map(node => node.y + node.height)) + 24;
    await repository.addText(projectId, activeBoard, 'sticky', x, y, 200, 120, noteText.trim(), '#ffe082');
    setNoteText('');
    setNoteOpen(false);
    await loadBoards();
  };

  // ── 交互：滚轮缩放（以鼠标位置为锚点）──
  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    const newZoom = Math.min(4, Math.max(0.25, zoom * factor));
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    // 保持鼠标指向的画布点不变：screen = pan + zoom * board
    setPan({
      x: mx - (mx - pan.x) * (newZoom / zoom),
      y: my - (my - pan.y) * (newZoom / zoom),
    });
    setZoom(newZoom);
  };

  // ── 交互：空格/中键平移 + 节点拖拽 ──
  const onNodeClick = async (nodeId: string) => {
    if (!linkMode || readOnly) return;
    if (linking && linking !== nodeId) {
      if (repository && activeBoard) {
        await repository.addArrow(projectId, activeBoard, linking, nodeId);
        const ns = await repository.getNodes(activeBoard);
        setNodes(ns);
      }
      setLinking(null);
      setLinkMode(false);
    } else if (!linking) {
      setLinking(nodeId);
    }
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (working || linkMode) return;
    if (inkMode && e.button === 0) {
      if (readOnly) return;
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const bx = (e.clientX - rect.left - pan.x) / zoom;
        const by = (e.clientY - rect.top - pan.y) / zoom;
        setInkPoints([{ x: bx, y: by }]);
      }
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      return;
    }
    if (e.button === 1 || e.button === 0) {
      const target = e.target as HTMLElement;
      const nodeEl = (target as HTMLElement).closest('[data-node-id]');
      if (nodeEl && e.button === 0) {
        const nodeId = (nodeEl as HTMLElement).dataset.nodeId!;
        const node = nodes.find(n => n.id === nodeId);
        if (node && !node.locked && !readOnly && !dragging) {
          setDragging(nodeId);
          dragStart.current = { x: e.clientX, y: e.clientY, nodeX: node.x, nodeY: node.y };
        }
      } else {
        panning.current = { x: e.clientX, y: e.clientY, panX: pan.x, panY: pan.y };
      }
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (inkMode && inkPoints.length > 0) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const bx = (e.clientX - rect.left - pan.x) / zoom;
        const by = (e.clientY - rect.top - pan.y) / zoom;
        setInkPoints(prev => prev.length < 10_000 ? [...prev, { x: bx, y: by }] : prev);
      }
      return;
    }
    if (dragging && dragStart.current) {
      const dx = (e.clientX - dragStart.current.x) / zoom;
      const dy = (e.clientY - dragStart.current.y) / zoom;
      setNodes(prev => prev.map(n =>
        n.id === dragging
          ? { ...n, x: dragStart.current!.nodeX + dx, y: dragStart.current!.nodeY + dy }
          : n
      ));
    } else if (panning.current) {
      setPan({
        x: panning.current.panX + (e.clientX - panning.current.x),
        y: panning.current.panY + (e.clientY - panning.current.y),
      });
    }
  };

  const onPointerUp = async () => {
    if (inkMode && inkPoints.length > 1 && repository && activeBoard) {
      const xs = inkPoints.map(p => p.x);
      const ys = inkPoints.map(p => p.y);
      const minX = Math.min(...xs), maxX = Math.max(...xs);
      const minY = Math.min(...ys), maxY = Math.max(...ys);
      await repository.addInk(projectId, activeBoard,
        minX, minY, maxX - minX, maxY - minY, inkPoints);
      setInkPoints([]);
      const ns = await repository.getNodes(activeBoard);
      setNodes(ns);
      return;
    }
    setInkPoints([]);
    if (dragging && dragStart.current && repository && activeBoard) {
      const node = nodes.find(n => n.id === dragging);
      const start = dragStart.current;
      if (node) {
        const dx = node.x - start.nodeX;
        const dy = node.y - start.nodeY;
        if (Math.abs(dx) > 1 || Math.abs(dy) > 1) {
          await repository.moveNodes(projectId, activeBoard, [dragging], dx, dy);
        }
      }
    }
    setDragging(null);
    dragStart.current = null;
    panning.current = null;
  };

  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!repository || !activeBoard || readOnly) return;
    const input = prompt('添加评论：');
    if (!input?.trim()) return;
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const bx = (e.clientX - rect.left - pan.x) / zoom;
    const by = (e.clientY - rect.top - pan.y) / zoom;
    await repository.addAnnotation(projectId, activeBoard, bx, by, input.trim());
    const ans = await repository.getAnnotations(activeBoard);
    setAnnotations(ans);
  };

  if (!repository) {
    return <div className="canvas-empty"><p>画布功能需要桌面版。</p></div>;
  }

  return (
    <div className="canvas-wrapper">
      <div className="canvas-toolbar">
        <select aria-label="选择画布" value={activeBoard ?? ''} disabled={working} onChange={e => { setNodes([]); setAnnotations([]); setNotice(''); setActiveBoard(e.target.value); setLinking(null); setLinkMode(false); setPan({ x: 0, y: 0 }); setZoom(1); }}>
          {boards.length === 0 && <option value="">无画布</option>}
          {boards.map(b => <option key={b.id} value={b.id}>{b.title}</option>)}
        </select>
        <Button size="small" disabled={creating || working || readOnly} onClick={() => void run(createBoard)}>新建画布</Button>
        <Button size="small" disabled={working || readOnly || !activeBoard || selectedPhotoIds.length === 0} onClick={() => void run(placeSelected)}>
          放入选中照片（{selectedPhotoIds.length}）
        </Button>
        <Button size="small" aria-expanded={noteOpen} disabled={working || readOnly || !activeBoard} onClick={() => setNoteOpen(!noteOpen)}>便签</Button>
        <Button size="small" aria-pressed={linkMode} disabled={working || readOnly || !activeBoard} onClick={() => { setLinkMode(!linkMode); setLinking(null); setInkMode(false); }}>{linkMode ? (linking ? '选择连接终点' : '选择连接起点') : '连接'}</Button>
        <Button size="small" aria-pressed={inkMode} disabled={working || readOnly || !activeBoard} onClick={() => { setInkMode(!inkMode); setLinkMode(false); setLinking(null); }}>画笔</Button>
        <span className="canvas-zoom-label">{Math.round(zoom * 100)}%</span>
      </div>
      {noteOpen && <form className="canvas-note-editor" onSubmit={event => { event.preventDefault(); void run(addSticky); }}>
        <textarea aria-label="便签内容" placeholder="写下你的想法" maxLength={5000} value={noteText} disabled={working || readOnly} onChange={event => setNoteText(event.target.value)} />
        <Button size="small" disabled={working || readOnly || !noteText.trim()} type="submit">保存便签</Button>
        <Button size="small" disabled={working} onClick={() => setNoteOpen(false)}>取消</Button>
      </form>}
      <div
        ref={containerRef}
        className={`canvas-surface ${inkMode ? "ink-mode" : ""}`}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={() => {
          panning.current = null;
          if (dragging || inkPoints.length) void run(async () => {
            try { await onPointerUp(); }
            catch (error) {
              if (repository && activeBoard) setNodes(await repository.getNodes(activeBoard));
              throw error;
            }
            finally { setDragging(null); dragStart.current = null; setInkPoints([]); }
          });
        }}
        onPointerCancel={() => {
          if (dragging && dragStart.current) {
            const start = dragStart.current;
            setNodes(current => current.map(node => node.id === dragging ? { ...node, x: start.nodeX, y: start.nodeY } : node));
          }
          setDragging(null); dragStart.current = null; panning.current = null; setInkPoints([]);
        }}
        onContextMenu={e => { e.preventDefault(); void run(() => onContextMenu(e)); }}
      >
        <div
          className="canvas-board"
          style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, transformOrigin: '0 0' }}
        >
          {nodes.filter(node => node.nodeType === 'photo').map(node => (
            <div
              key={node.id}
              data-node-id={node.id}
              className={`canvas-photo-node ${dragging === node.id ? 'dragging' : ''}`}
              onClick={() => void run(() => onNodeClick(node.id))}
              style={{
                left: node.x, top: node.y, width: node.width, height: node.height,
                zIndex: node.zOrder,
                transform: node.rotation ? `rotate(${node.rotation}deg)` : undefined,
              }}
            >
              {node.thumbnailPath
                ? <img src={node.thumbnailPath} alt="" draggable={false} />
                : <div className="canvas-photo-placeholder">📷</div>}
            </div>
          ))}
          {nodes.filter(n => ['sticky', 'text', 'shape', 'chapter'].includes(n.nodeType)).map(node => {
            const content = node.contentJson ? (() => { try { return JSON.parse(node.contentJson!).text ?? ''; } catch { return ''; } })() : '';
            const color = node.styleJson ? (() => { try { return JSON.parse(node.styleJson!).color ?? '#ffe082'; } catch { return '#ffe082'; } })() : '#ffe082';
            return <div key={node.id} data-node-id={node.id}
              onClick={() => void run(() => onNodeClick(node.id))}
              className={`canvas-sticky-node ${dragging === node.id ? 'dragging' : ''}`}
              style={{ left: node.x, top: node.y, width: node.width, height: node.height, zIndex: node.zOrder, background: color }}
            >{content}</div>;
          })}
          {inkPoints.length > 1 && (() => {
            const d = inkPoints.map((p, i) => i === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`).join(' ');
            return <svg style={{ position: 'absolute', left: 0, top: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 9999, overflow: 'visible' }}>
              <path d={d} fill="none" stroke="#ff6b6b" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
            </svg>;
          })()}
          {nodes.filter(n => n.nodeType === 'ink').map(ink => {
            const pts = ink.contentJson ? (() => { try { return (JSON.parse(ink.contentJson!).points ?? []) as Array<{x:number;y:number}>; } catch { return []; } })() : [];
            if (pts.length < 2) return null;
            const d = pts.map((p, i) => i === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`).join(' ');
            const color = ink.styleJson ? (() => { try { return JSON.parse(ink.styleJson!).color ?? '#ff6b6b'; } catch { return '#ff6b6b'; } })() : '#ff6b6b';
            const sw = ink.styleJson ? (() => { try { return JSON.parse(ink.styleJson!).strokeWidth ?? 2; } catch { return 2; } })() : 2;
            return <svg key={ink.id} style={{ position: 'absolute', left: 0, top: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: ink.zOrder, overflow: 'visible' }}>
              <path d={d} fill="none" stroke={color} strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" />
            </svg>;
          })}
          {nodes.filter(n => n.nodeType === 'arrow').map(arrow => {
            const ids = arrow.childrenJson ? (() => { try { return JSON.parse(arrow.childrenJson!) as string[]; } catch { return null; } })() : null;
            if (!ids || ids.length !== 2) return null;
            const from = nodes.find(n => n.id === ids[0]);
            const to = nodes.find(n => n.id === ids[1]);
            if (!from || !to) return null;
            const x1 = from.x + from.width / 2;
            const y1 = from.y + from.height / 2;
            const x2 = to.x + to.width / 2;
            const y2 = to.y + to.height / 2;
            return <svg key={arrow.id} style={{ position: 'absolute', left: 0, top: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: arrow.zOrder, overflow: 'visible' }}>
              <defs><marker id={"ah-" + arrow.id} markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="#6c5ce7" /></marker></defs>
              <line x1={x1} y1={y1} x2={x2} y2={y2} stroke="#6c5ce7" strokeWidth={2} markerEnd={"url(#ah-" + arrow.id + ")"} />
            </svg>;
          })}
          {annotations.map(a => !a.resolved && (
            <div key={a.id} className="canvas-annotation-pin" title={a.body}
              style={{ left: a.x ?? 0, top: a.y ?? 0 }}>
              💬
            </div>
          ))}
          {nodes.length === 0 && activeBoard && (
            <div className="canvas-hint" style={{ left: -pan.x / zoom + 40, top: -pan.y / zoom + 40 }}>
              从工具栏选照片放置到画布
            </div>
          )}
        </div>
      </div>
      {notice && <p role="status" className="canvas-notice">{notice}</p>}
    </div>
  );
}
