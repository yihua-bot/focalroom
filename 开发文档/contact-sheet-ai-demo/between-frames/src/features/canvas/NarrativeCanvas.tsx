// G3 Narrative Canvas：无限自由画布——照片节点拖拽放置。
// v3 蓝图 §03.4/§06：逻辑坐标独立于设备像素；照片引用资产不复制像素。
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/shared/Button';

export interface BoardInfo { id: string; projectId: string; title: string; state: string }
export interface CanvasNodeInfo {
  id: string; boardId: string; nodeType: string;
  x: number; y: number; width: number; height: number;
  rotation: number; zOrder: number;
  assetId: string | null; renditionRef: string | null;
}

interface CanvasPhoto extends CanvasNodeInfo {
  thumbnailPath?: string;
}

/** 无限画布：拖拽平移 + 滚轮缩放 + 照片节点拖拽放置。 */
export function NarrativeCanvas({ repository, projectId, selectedPhotoIds }: {
  repository?: { canvas: CanvasRepository };
  projectId: string;
  selectedPhotoIds: string[];
}) {
  const [boards, setBoards] = useState<BoardInfo[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [nodes, setNodes] = useState<CanvasPhoto[]>([]);
  const [creating, setCreating] = useState(false);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [dragging, setDragging] = useState<string | null>(null);
  const dragStart = useRef<{ x: number; y: number; nodeX: number; nodeY: number } | null>(null);
  const panning = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const boardList = useRef<BoardInfo[]>([]);
  // 节点跟随 activeBoard 变化加载（异步回调内 setState 合法）
  useEffect(() => {
    let active = true;
    if (repository) {
      repository.canvas.listBoards(projectId).then(list => {
        if (!active) return;
        boardList.current = list;
        setBoards(list);
        setActiveBoard(prev => prev ?? (list.length > 0 ? list[0].id : null));
      }).catch(() => {});
    }
    return () => { active = false; };
  }, [repository, projectId]);

  useEffect(() => {
    let active = true;
    if (repository && activeBoard) {
      repository.canvas.getNodes(activeBoard).then(ns => {
        if (active) setNodes(ns);
      }).catch(() => {});
    }
    return () => { active = false; };
  }, [repository, activeBoard]);

  const loadBoards = async () => {
    if (!repository) return;
    const list = await repository.canvas.listBoards(projectId);
    setBoards(list);
    if (list.length > 0) setActiveBoard(prev => prev ?? list[0].id);
    if (activeBoard) {
      const ns = await repository.canvas.getNodes(activeBoard);
      setNodes(ns);
    }
  };

  const createBoard = async () => {
    if (!repository || creating) return;
    setCreating(true);
    try {
      const board = await repository.canvas.createBoard(projectId, `画布 ${boards.length + 1}`);
      setActiveBoard(board.id);
      await loadBoards();
    } finally { setCreating(false); }
  };

  const placeSelected = async () => {
    if (!repository || !activeBoard || selectedPhotoIds.length === 0) return;
    const startX = -pan.x / zoom + 50;
    for (let i = 0; i < selectedPhotoIds.length; i++) {
      await repository.canvas.placePhoto(projectId, activeBoard, selectedPhotoIds[i],
        startX + i * 240, 50, 220, 160);
    }
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
  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button === 1 || e.button === 0) {
      const target = e.target as HTMLElement;
      const nodeEl = (target as HTMLElement).closest('[data-node-id]');
      if (nodeEl && e.button === 0) {
        const nodeId = (nodeEl as HTMLElement).dataset.nodeId!;
        const node = nodes.find(n => n.id === nodeId);
        if (node && !dragging) {
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
    if (dragging && dragStart.current && repository && activeBoard) {
      const node = nodes.find(n => n.id === dragging);
      const start = dragStart.current;
      if (node) {
        const dx = node.x - start.nodeX;
        const dy = node.y - start.nodeY;
        if (Math.abs(dx) > 1 || Math.abs(dy) > 1) {
          await repository.canvas.moveNodes(projectId, activeBoard, [dragging], dx, dy);
        }
      }
    }
    setDragging(null);
    dragStart.current = null;
    panning.current = null;
  };

  if (!repository) {
    return <div className="canvas-empty"><p>画布功能需要桌面版。</p></div>;
  }

  return (
    <div className="canvas-wrapper">
      <div className="canvas-toolbar">
        <select value={activeBoard ?? ''} onChange={e => setActiveBoard(e.target.value)}>
          {boards.length === 0 && <option value="">无画布</option>}
          {boards.map(b => <option key={b.id} value={b.id}>{b.title}</option>)}
        </select>
        <Button size="small" disabled={creating} onClick={() => void createBoard()}>新建画布</Button>
        <Button size="small" disabled={!activeBoard || selectedPhotoIds.length === 0} onClick={() => void placeSelected()}>
          放入选中照片（{selectedPhotoIds.length}）
        </Button>
        <span className="canvas-zoom-label">{Math.round(zoom * 100)}%</span>
      </div>
      <div
        ref={containerRef}
        className="canvas-surface"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerUp}
      >
        <div
          className="canvas-board"
          style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, transformOrigin: '0 0' }}
        >
          {nodes.map(node => (
            <div
              key={node.id}
              data-node-id={node.id}
              className={`canvas-photo-node ${dragging === node.id ? 'dragging' : ''}`}
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
          {nodes.length === 0 && activeBoard && (
            <div className="canvas-hint" style={{ left: -pan.x / zoom + 40, top: -pan.y / zoom + 40 }}>
              从工具栏选照片放置到画布
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export interface CanvasRepository {
  listBoards(projectId: string): Promise<BoardInfo[]>;
  createBoard(projectId: string, title: string): Promise<BoardInfo>;
  getNodes(boardId: string): Promise<CanvasPhoto[]>;
  placePhoto(projectId: string, boardId: string, assetId: string, x: number, y: number, w: number, h: number): Promise<CanvasNodeInfo>;
  moveNodes(projectId: string, boardId: string, nodeIds: string[], dx: number, dy: number): Promise<void>;
}
