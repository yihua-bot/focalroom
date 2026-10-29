// G3 Narrative Canvas：无限自由画布——照片节点拖拽放置。
// v3 蓝图 §03.4/§06：逻辑坐标独立于设备像素；照片引用资产不复制像素。
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { useUIStore } from '../../stores/uiStore';
import { zh } from '../../app/copy';
import { useAccount } from '../cloud-login/AccountProvider';
import { CanvasNodes } from './CanvasNodes';
import { EditorialReviewWorkspace } from './EditorialReviewWorkspace';
import { EditorialHistoryDialog } from './EditorialHistoryDialog';
import { Maximize } from 'lucide-react';

import type { AnnotationInfo, BoardInfo, CanvasPhoto, CanvasRepository } from '../../domain/canvas';

async function readCanvas(repository: CanvasRepository, boardId: string) {
  if (repository.getSnapshot) return repository.getSnapshot(boardId);
  const [nodes, annotations] = await Promise.all([repository.getNodes(boardId), repository.getAnnotations(boardId)]);
  return { nodes, annotations, board: undefined };
}
/** 无限画布：拖拽平移 + 滚轮缩放 + 照片节点拖拽放置。 */
export function NarrativeCanvas({ repository, projectId, selectedPhotoIds, readOnly = false }: {
  repository?: CanvasRepository;
  projectId: string;
  selectedPhotoIds: string[];
  readOnly?: boolean;
}) {
  const account = useAccount();
  const boardRequest = useUIStore(state => state.canvasBoardRequest);
  const requestedBoard = useUIStore(state => state.canvasBoardId);
  const openCanvasAgent = useUIStore(state => state.openCanvasAgent);
  const openDraftRevision = useUIStore(state => state.openDraftRevision);
  const [boards, setBoards] = useState<BoardInfo[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [nodes, setNodes] = useState<CanvasPhoto[]>([]);
  const [creating, setCreating] = useState(false);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [tileLayers, setTileLayers] = useState<Record<string, Array<{ url: string; left: number; top: number; width: number; height: number }>>>({});
  const tileCache = useRef(new Map<string, Array<{ url: string; left: number; top: number; width: number; height: number }>>());
  // G2-②：缩放超过 1.25× 后缩略图（320px）明显像素化——按显示需求选最粗满足级
  // 请求瓦片（单节点 ≤9 张，过多自动粗化），结果按 assetId:层级 缓存。
  useEffect(() => {
    if (!repository?.photoTile || zoom <= 1.25) return;
    let cancelled = false;
    void (async () => {
      const results: Record<string, Array<{ url: string; left: number; top: number; width: number; height: number }>> = {};
      const tasks: Array<Promise<void>> = [];
      for (const node of nodes) {
        if (node.nodeType !== 'photo' || !node.assetId || !node.sourceWidth || !node.sourceHeight || node.visible === false) continue;
        const need = Math.max(node.width, node.height) * zoom;
        let level = 0;
        let levelW = node.sourceWidth;
        let levelH = node.sourceHeight;
        while (Math.ceil(Math.max(levelW, levelH) / 2) >= need && Math.ceil(Math.max(levelW, levelH) / 2) >= 256) { level += 1; levelW = Math.ceil(levelW / 2); levelH = Math.ceil(levelH / 2); }
        while (Math.ceil(levelW / 256) * Math.ceil(levelH / 256) > 9 && Math.ceil(Math.max(levelW, levelH) / 2) >= 256) { level += 1; levelW = Math.ceil(levelW / 2); levelH = Math.ceil(levelH / 2); }
        if (Math.ceil(levelW / 256) * Math.ceil(levelH / 256) > 9) continue;
        const key = `${node.assetId}:${level}`;
        const cached = tileCache.current.get(key);
        if (cached) { results[node.id] = cached; continue; }
        const assetId = node.assetId;
        const cols = Math.ceil(levelW / 256);
        const rows = Math.ceil(levelH / 256);
        tasks.push((async () => {
          const tiles: Array<{ url: string; left: number; top: number; width: number; height: number }> = [];
          for (let ty = 0; ty < rows; ty += 1) {
            for (let tx = 0; tx < cols; tx += 1) {
              const tile = await repository.photoTile!(projectId, assetId, level, tx, ty);
              tiles.push({ url: tile.url, left: (node.width * tx * 256) / levelW, top: (node.height * ty * 256) / levelH, width: (node.width * tile.width) / levelW, height: (node.height * tile.height) / levelH });
            }
          }
          tileCache.current.set(key, tiles);
          results[node.id] = tiles;
        })());
      }
      await Promise.all(tasks).catch(() => { /* 瓦片失败保持缩略图，不阻塞画布 */ });
      if (!cancelled) setTileLayers(results);
    })();
    return () => { cancelled = true; };
  }, [zoom, nodes, repository, projectId]);
  const [dragging, setDragging] = useState<string | null>(null);
  const [selectedNodeIds, setSelectedNodeIds] = useState<Set<string>>(new Set());
  const [marquee, setMarquee] = useState<{x0:number;y0:number;x1:number;y1:number} | null>(null);
  const [resizing, setResizing] = useState<{nodeId:string;startW:number;startH:number;x:number;y:number} | null>(null);
  const [annotations, setAnnotations] = useState<AnnotationInfo[]>([]);
  const [linking, setLinking] = useState<string | null>(null);
  const [linkMode, setLinkMode] = useState(false);
  const [notice, setNotice] = useState('');
  const [working, setWorking] = useState(false);
  const [pending, setPending] = useState<Array<{ boardId: string }>>([]);
  const [loadedBoard, setLoadedBoard] = useState<string | null>(null);
  const [comparison, setComparison] = useState<{ boardId: string; value: Awaited<ReturnType<NonNullable<CanvasRepository['compareDraft']>>> } | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [showBaseline, setShowBaseline] = useState(true);
  const [noteOpen, setNoteOpen] = useState(false);
  const [noteText, setNoteText] = useState('');
  const [adopting, setAdopting] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const [inkMode, setInkMode] = useState(false);
  const [inkPoints, setInkPoints] = useState<Array<{x:number;y:number}>>([]);
  const dragStart = useRef<{ x: number; y: number; nodeX: number; nodeY: number; positions: Map<string, { x: number; y: number }> } | null>(null);
  const panning = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const board = boards.find(value => value.id === activeBoard);
  const review = comparison?.boardId === activeBoard && comparison.value.draftRevision === board?.revision ? comparison.value : null;
  const editLocked = readOnly || activeBoard !== loadedBoard || pending.some(item => item.boardId === activeBoard) || Boolean(board?.owningTaskId) || board?.state === 'frozen';

  const run = async (action: () => Promise<void>) => {
    if (readOnly || working) return;
    setWorking(true);
    setNotice('');
    try { await action(); }
    catch (error) {
      setNotice(error instanceof Error ? error.message : '画布操作失败');
      if ((error as { code?: string })?.code === 'REVISION_CONFLICT') {
        try { await loadBoards(); } catch { /* 保留首次冲突提示，用户可重新打开画布。 */ }
      }
    }
    finally { setPending(repository?.pendingCommands?.(projectId) ?? []); setWorking(false); }
  };
  // 节点跟随 activeBoard 变化加载（异步回调内 setState 合法）
  useEffect(() => {
    let active = true;
    if (repository) {
      repository.listBoards(projectId).then(list => {
        if (!active) return;
        setBoards(list);
        setPending(repository.pendingCommands?.(projectId) ?? []);
        setActiveBoard(prev => list.some(value => value.id === requestedBoard) ? requestedBoard : list.some(value => value.id === prev) ? prev : (list[0]?.id ?? null));
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取画布失败'); });
    }
    return () => { active = false; };
  }, [repository, projectId, requestedBoard, boardRequest]);

  useEffect(() => {
    let active = true;
    if (repository && activeBoard) {
      readCanvas(repository, activeBoard).then(snapshot => {
        if (!active) return;
        setNodes(snapshot.nodes); setAnnotations(snapshot.annotations); setLoadedBoard(activeBoard);
        if (snapshot.board) setBoards(list => list.map(board => board.id === activeBoard ? snapshot.board! : board));
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取照片失败'); });
    }
    return () => { active = false; };
  }, [repository, activeBoard]);

  useEffect(() => {
    let active = true;
    if (repository?.compareDraft && activeBoard && board?.owningTaskId) {
      repository.compareDraft(projectId, activeBoard).then(value => {
        if (active) {
          setComparison({ boardId: activeBoard, value });
          if (requestedBoard === activeBoard && repository.simulateMerge) setReviewing(true);
        }
      })
        .catch(error => { if (active) setNotice(error instanceof Error ? error.message : '读取方案失败'); });
    }
    return () => { active = false; };
  }, [repository, projectId, activeBoard, board?.owningTaskId, board?.revision, requestedBoard, boardRequest]);

  const loadBoards = async () => {
    if (!repository) return;
    const list = await repository.listBoards(projectId);
    let current = list;
    if (list.length > 0) setActiveBoard(prev => prev ?? list[0].id);
    if (activeBoard) {
      const snapshot = await readCanvas(repository, activeBoard);
      setNodes(snapshot.nodes); setAnnotations(snapshot.annotations); setLoadedBoard(activeBoard);
      if (snapshot.board) current = list.map(board => board.id === activeBoard ? snapshot.board! : board);
    }
    setBoards(current);
  };

  const restoreHistory = async (redo: boolean) => {
    if (!repository || !activeBoard || editLocked) return;
    const changed = await (redo ? repository.redo(projectId, activeBoard) : repository.undo(projectId, activeBoard));
    await loadBoards();
    setSelectedNodeIds(new Set());
    if (!changed) setNotice(redo ? '没有可重做的操作' : '没有可撤销的操作');
  };

  const adoptBoard = async () => {
    if (!repository || !activeBoard || readOnly) return;
    await repository.adoptBoard(projectId, activeBoard);
    setAdopting(false);
    await loadBoards();
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
    if (selectedPhotoIds.length > 300) throw new Error('每次最多放入 300 张照片，请分次选择。');
    const startX = -pan.x / zoom + 50;
    if (repository.placePhotos) await repository.placePhotos(projectId, activeBoard, selectedPhotoIds.map((assetId, i) => ({ assetId, x: startX + i * 240, y: 50, width: 220, height: 160 })));
    else for (let i = 0; i < selectedPhotoIds.length; i++) await repository.placePhoto(projectId, activeBoard, selectedPhotoIds[i], startX + i * 240, 50, 220, 160);
    await loadBoards();
  };

  const toggleLockSelected = async () => {
    if (!repository || !activeBoard || editLocked || selectedNodeIds.size === 0) return;
    const first = nodes.find(n => selectedNodeIds.has(n.id));
    if (!first) return;
    const target = !first.locked;
    if (repository.setNodesLocked) await repository.setNodesLocked(projectId, activeBoard, [...selectedNodeIds], target);
    else for (const id of selectedNodeIds) await repository.setNodeLocked(projectId, activeBoard, id, target);
    const ns = await repository.getNodes(activeBoard);
    setNodes(ns);
  };

  const groupSelected = async () => {
    if (!repository || !activeBoard || selectedNodeIds.size < 2) return;
    const label = prompt('分组标签：')?.trim();
    if (!label) return;
    await repository.groupNodes(projectId, activeBoard, Array.from(selectedNodeIds), label);
    setSelectedNodeIds(new Set());
    const ns = await repository.getNodes(activeBoard);
    setNodes(ns);
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
  const fitContent = () => {
    const rect = containerRef.current?.getBoundingClientRect();
    const visible = [...nodes, ...(showBaseline ? review?.baselineNodes ?? [] : [])].filter(node => node.nodeType !== 'arrow' && node.visible !== false);
    if (!rect || rect.width <= 64 || rect.height <= 64 || !visible.length) return;
    const left = Math.min(...visible.map(node => node.x));
    const top = Math.min(...visible.map(node => node.y));
    const right = Math.max(...visible.map(node => node.x + node.width));
    const bottom = Math.max(...visible.map(node => node.y + node.height));
    const scale = Math.min(1, (rect.width - 64) / Math.max(1, right - left), (rect.height - 64) / Math.max(1, bottom - top));
    setZoom(scale);
    setPan({ x: rect.width / 2 - (left + right) / 2 * scale, y: rect.height / 2 - (top + bottom) / 2 * scale });
  };
  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    const newZoom = Math.min(4, Math.max(1e-8, zoom * factor));
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
    if (!linkMode || editLocked) return;
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
      if (editLocked) return;
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
        if (node && !editLocked) {
          const selection = e.shiftKey ? new Set(selectedNodeIds) : selectedNodeIds.has(nodeId) ? new Set(selectedNodeIds) : new Set([nodeId]);
          if (e.shiftKey) { if (selection.has(nodeId)) selection.delete(nodeId); else selection.add(nodeId); }
          setSelectedNodeIds(selection);
          if (node.locked || e.shiftKey || dragging) return;
          setDragging(nodeId);
          const positions = new Map(nodes.filter(value => selection.has(value.id) && !value.locked).map(value => [value.id, { x: value.x, y: value.y }]));
          dragStart.current = { x: e.clientX, y: e.clientY, nodeX: node.x, nodeY: node.y, positions };
        }
      } else if (e.shiftKey) {
        // Shift + 空白拖拽 = 框选
        const rect = containerRef.current?.getBoundingClientRect();
        if (rect) {
          const bx = (e.clientX - rect.left - pan.x) / zoom;
          const by = (e.clientY - rect.top - pan.y) / zoom;
          setMarquee({ x0: bx, y0: by, x1: bx, y1: by });
        }
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      } else {
        // 普通空白点击：取消选择 + 平移
        setSelectedNodeIds(new Set());
        panning.current = { x: e.clientX, y: e.clientY, panX: pan.x, panY: pan.y };
      }
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (resizing) {
      const dx = (e.clientX - resizing.x) / zoom;
      const dy = (e.clientY - resizing.y) / zoom;
      const w = Math.min(10000, Math.max(20, resizing.startW + dx));
      const h = Math.min(10000, Math.max(20, resizing.startH + dy));
      setNodes(prev => prev.map(n => n.id === resizing.nodeId ? { ...n, width: w, height: h } : n));
      return;
    }
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
      const positions = dragStart.current.positions;
      setNodes(prev => prev.map(n => {
        const origin = positions.get(n.id);
        return origin ? { ...n, x: origin.x + dx, y: origin.y + dy } : n;
      }));
    } else if (marquee) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const bx = (e.clientX - rect.left - pan.x) / zoom;
        const by = (e.clientY - rect.top - pan.y) / zoom;
        setMarquee(prev => prev ? { ...prev, x1: bx, y1: by } : null);
      }
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
          await repository.moveNodes(projectId, activeBoard, Array.from(start.positions.keys()), dx, dy);
        }
      }
    }
    if (marquee) {
      // 计算与框选矩形相交的所有照片/便签节点
      const x0 = Math.min(marquee.x0, marquee.x1), x1 = Math.max(marquee.x0, marquee.x1);
      const y0 = Math.min(marquee.y0, marquee.y1), y1 = Math.max(marquee.y0, marquee.y1);
      const hit = new Set(nodes.filter(n =>
        n.nodeType !== 'arrow' && n.visible !== false &&
        n.x < x1 && n.x + n.width > x0 && n.y < y1 && n.y + n.height > y0
      ).map(n => n.id));
      setSelectedNodeIds(hit);
      setMarquee(null);
      return;
    }
    if (resizing) {
      const node = nodes.find(n => n.id === resizing.nodeId);
      if (node && repository && activeBoard) {
        await repository.resizeNode(projectId, activeBoard, node.id, node.width, node.height);
      }
      setResizing(null);
      return;
    }
    setDragging(null);
    dragStart.current = null;
    panning.current = null;
  };

  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!repository || !activeBoard || editLocked) return;
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

  const cancelGesture = () => {
    const start = dragStart.current;
    if (start) setNodes(current => current.map(node => {
      const origin = start.positions.get(node.id);
      return origin ? { ...node, ...origin } : node;
    }));
    if (resizing) setNodes(current => current.map(node => node.id === resizing.nodeId ? { ...node, width: resizing.startW, height: resizing.startH } : node));
    setResizing(null); setMarquee(null); setDragging(null);
    dragStart.current = null; panning.current = null; setInkPoints([]);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if ((event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
    if (event.key === 'Escape') { cancelGesture(); return; }
    if (editLocked) return;
    const key = event.key.toLowerCase();
    if ((event.metaKey || event.ctrlKey) && (key === 'z' || key === 'y')) {
      event.preventDefault();
      if (!dragging && !resizing) void run(() => restoreHistory(key === 'y' || event.shiftKey));
    } else if (key === 'l' && !event.metaKey && !event.ctrlKey) {
      event.preventDefault(); void run(toggleLockSelected);
    }
  };

  if (!repository) {
    return <div className="canvas-empty"><p>画布功能需要桌面版。</p></div>;
  }

  return (
    <div className="canvas-wrapper" onKeyDown={onKeyDown}>
      <div className="canvas-toolbar">
        <select aria-label="选择画布" value={activeBoard ?? ''} disabled={working} onChange={e => { setNodes([]); setAnnotations([]); setNotice(''); setActiveBoard(e.target.value); setLinking(null); setLinkMode(false); setPan({ x: 0, y: 0 }); setZoom(1); }}>
          {boards.length === 0 && <option value="">无画布</option>}
          {boards.map(b => <option key={b.id} value={b.id}>{b.title}</option>)}
        </select>
        <Button size="small" disabled={creating || working || readOnly} onClick={() => void run(createBoard)}>新建画布</Button>
        {board && !board.owningTaskId && repository.editorialHistory && <Button size="small" disabled={working} onClick={() => setHistoryOpen(true)}>{zh.editorialHistory}</Button>}
        {board?.owningTaskId && repository.simulateMerge && <Button size="small" disabled={working || !review} onClick={() => setReviewing(true)}>{zh.editorialOpen}</Button>}
        {board?.owningTaskId && <Button size="small" disabled={working || readOnly || Boolean(repository.compareDraft && !review)} onClick={() => setAdopting(true)}>{zh.editorialSaveIndependent}</Button>}
        <Button size="small" disabled={working || editLocked || !activeBoard || selectedPhotoIds.length === 0} onClick={() => void run(placeSelected)}>
          放入选中照片（{selectedPhotoIds.length}）
        </Button>
        <Button size="small" disabled={working || editLocked || !board?.revision} onClick={() => void run(async () => {
          if (!board?.revision || !repository) return;
          const snapshot = await readCanvas(repository, board.id);
          setNodes(snapshot.nodes); setAnnotations(snapshot.annotations);
          if (snapshot.board) setBoards(list => list.map(value => value.id === board.id ? snapshot.board! : value));
          const source = { boardId: board.id, revision: snapshot.board?.revision ?? board.revision, selectedNodeIds: [...selectedNodeIds] };
          if (account) await account.requireSignIn('ai', () => openCanvasAgent(source));
          else openCanvasAgent(source);
        })}>{zh.chatArrangeBoard}</Button>
        <Button size="small" aria-expanded={noteOpen} disabled={working || editLocked || !activeBoard} onClick={() => setNoteOpen(!noteOpen)}>便签</Button>
        <Button size="small" aria-pressed={linkMode} disabled={working || editLocked || !activeBoard} onClick={() => { setLinkMode(!linkMode); setLinking(null); setInkMode(false); }}>{linkMode ? (linking ? '选择连接终点' : '选择连接起点') : '连接'}</Button>
        <Button size="small" aria-pressed={inkMode} disabled={working || editLocked || !activeBoard} onClick={() => { setInkMode(!inkMode); setLinkMode(false); setLinking(null); }}>画笔</Button>
        <Button size="small" disabled={working || editLocked || !activeBoard || selectedNodeIds.size < 2} onClick={() => void run(groupSelected)}>分组</Button>
        <Button size="small" disabled={working || editLocked || !activeBoard} onClick={() => void run(() => restoreHistory(false))}>撤销</Button>
        <Button size="small" disabled={working || editLocked || !activeBoard} onClick={() => void run(() => restoreHistory(true))}>重做</Button>
        <Button size="small" disabled={working || editLocked || !activeBoard || selectedNodeIds.size === 0} onClick={() => void run(toggleLockSelected)}>锁定 / 解锁</Button>
        <Button size="small" aria-label={zh.chatBoardFit} title={zh.chatBoardFit} disabled={working || !nodes.length} onClick={fitContent}><Maximize size={14} /></Button>
        <span className="canvas-zoom-label">{zoom < 0.1 ? (zoom * 100).toPrecision(2) : Math.round(zoom * 100)}%</span>
      </div>
      {pending.length > 0 && <div className="canvas-pending" role="status">
        <span>有画布操作尚未确认。确认后可继续编辑。</span>
        {pending.map(item => <Button key={item.boardId} size="small" disabled={working || !repository.retryPending} onClick={() => void run(async () => {
          await repository.retryPending?.(projectId, item.boardId);
          await loadBoards();
          setNotice('上次操作已确认');
        })}>确认上次操作{pending.length > 1 ? ` · ${boards.find(board => board.id === item.boardId)?.title ?? '新画布'}` : ''}</Button>)}
      </div>}
      {notice && <p role="status" className="canvas-notice">{notice}</p>}
      {board?.owningTaskId && repository.compareDraft && <div className="canvas-review" aria-label={zh.chatBoardChanges}>
        {review ? <>
          <div className="canvas-review-summary"><strong>{zh.chatBoardChanges}</strong>
            {(['add', 'change', 'remove'] as const).map(kind => <span key={kind}>{({ add: zh.chatBoardAdded, change: zh.chatBoardChanged, remove: zh.chatBoardRemoved })[kind]} {review.changes.filter(item => item.kind === kind).length}</span>)}
            {review.baselineNodes.length > 0 && <label><input type="checkbox" checked={showBaseline} onChange={event => setShowBaseline(event.target.checked)} />{zh.chatBoardOverlay}</label>}
          </div>
          {review.sourceChanged && <p>{zh.chatBoardSourceChanged}</p>}
        </> : <Button size="small" disabled={working} onClick={() => void run(loadBoards)}>{zh.chatBoardRefresh}</Button>}
      </div>}
      {noteOpen && <form className="canvas-note-editor" onSubmit={event => { event.preventDefault(); void run(addSticky); }}>
        <textarea aria-label="便签内容" placeholder="写下你的想法" maxLength={5000} value={noteText} disabled={working || editLocked} onChange={event => setNoteText(event.target.value)} />
        <Button size="small" disabled={working || editLocked || !noteText.trim()} type="submit">保存便签</Button>
        <Button size="small" disabled={working} onClick={() => setNoteOpen(false)}>取消</Button>
      </form>}
      <div
        ref={containerRef}
        className={`canvas-surface ${inkMode ? "ink-mode" : ""}`}
        tabIndex={0}
        aria-label="叙事画布"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={() => {
          panning.current = null;
          if (dragging || resizing || marquee || inkPoints.length) void run(async () => {
            try { await onPointerUp(); }
            catch (error) {
              if (repository && activeBoard) setNodes(await repository.getNodes(activeBoard));
              throw error;
            }
            finally { setDragging(null); setResizing(null); setMarquee(null); dragStart.current = null; setInkPoints([]); }
          });
        }}
        onPointerCancel={cancelGesture}
        onContextMenu={e => { e.preventDefault(); void run(() => onContextMenu(e)); }}
      >
        <div
          className="canvas-board"
          style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, transformOrigin: '0 0' }}
        >
          {showBaseline && review?.baselineNodes.filter(node => node.nodeType !== 'arrow').map(node => <div key={'baseline-' + node.id} className="canvas-baseline-node" data-testid="canvas-baseline-node"
            style={{ left: node.x, top: node.y, width: node.width, height: node.height }}>
            <span>{zh.chatBoardOriginal}</span>
          </div>)}
          <CanvasNodes nodes={nodes} selected={selectedNodeIds} dragging={dragging} editable={!editLocked}
            onNodeClick={id => void run(() => onNodeClick(id))}
            onResize={(node, event) => {
              event.stopPropagation();
              if (!node.locked && !working) {
                setResizing({ nodeId: node.id, startW: node.width, startH: node.height, x: event.clientX, y: event.clientY });
                event.currentTarget.setPointerCapture(event.pointerId);
              }
            }} />
          {zoom > 1.25 && Object.entries(tileLayers).map(([nodeId, tiles]) => {
            const node = nodes.find(item => item.id === nodeId);
            if (!node || node.visible === false) return null;
            return <div key={'tiles-' + nodeId} className="canvas-tile-layer" aria-hidden="true"
              style={{ left: node.x, top: node.y, width: node.width, height: node.height, zIndex: node.zOrder }}>
              {tiles.map((tile, index) => <img key={index} src={tile.url} alt="" draggable={false}
                style={{ position: 'absolute', left: tile.left, top: tile.top, width: tile.width, height: tile.height }} />)}
            </div>;
          })}
          {marquee && (() => {
            const x = Math.min(marquee.x0, marquee.x1);
            const y = Math.min(marquee.y0, marquee.y1);
            const w = Math.abs(marquee.x1 - marquee.x0);
            const h = Math.abs(marquee.y1 - marquee.y0);
            return <div className="canvas-marquee" style={{ position: 'absolute', left: x, top: y, width: w, height: h, pointerEvents: 'none', zIndex: 9998 }} />;
          })()}
          {inkPoints.length > 1 && (() => {
            const d = inkPoints.map((p, i) => i === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`).join(' ');
            return <svg style={{ position: 'absolute', left: 0, top: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 9999, overflow: 'visible' }}>
              <path d={d} fill="none" stroke="#ff6b6b" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
            </svg>;
          })()}
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
      {reviewing && review && board && <EditorialReviewWorkspace key={`${board.id}-${review.draftRevision}`} repository={repository} projectId={projectId} board={board} comparison={review} readOnly={readOnly}
        onRevise={() => {
          const revise = () => { setReviewing(false); openDraftRevision(projectId,{ kind: 'board', id: board.id, revision: review.draftRevision },board.title,zh.editorialReviseGoal,Math.max(1,(review.draftNodes ?? []).filter(n => n.nodeType === 'photo').length)); };
          if (account) void account.requireSignIn('ai',revise); else revise();
        }}
        onClose={() => { setReviewing(false); setPending(repository.pendingCommands?.(projectId) ?? []); }} onApplied={async target => {
          const list = await repository.listBoards(projectId);
          const snapshot = await readCanvas(repository, target);
          setBoards(list.map(b => b.id === target && snapshot.board ? snapshot.board : b));
          setNodes(snapshot.nodes); setAnnotations(snapshot.annotations); setLoadedBoard(target); setActiveBoard(target);
          setSelectedNodeIds(new Set()); setReviewing(false); setNotice(zh.editorialApplied);
        }}
        onSaveIndependent={() => { setReviewing(false); setAdopting(true); }} />}
      {historyOpen && activeBoard && <EditorialHistoryDialog key={activeBoard} repository={repository} projectId={projectId} boardId={activeBoard} readOnly={readOnly} onClose={() => { setHistoryOpen(false); setPending(repository.pendingCommands?.(projectId) ?? []); }} onChanged={loadBoards} />}
      {adopting && <Modal title={zh.editorialSaveIndependent} description={zh.editorialIndependentDescription} onClose={() => { if (!working) setAdopting(false); }}>
        <Button disabled={working || readOnly} onClick={() => void run(adoptBoard)}>{zh.editorialSaveIndependent}</Button>
        <Button disabled={working} onClick={() => setAdopting(false)}>取消</Button>
      </Modal>}
    </div>
  );
}
