// G3 Narrative Canvas：无限自由画布——照片节点拖拽放置。
// v3 蓝图 §03.4/§06：逻辑坐标独立于设备像素；照片引用资产不复制像素。
import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { useUIStore } from '../../stores/uiStore';
import { zh } from '../../app/copy';
import { useAccount } from '../cloud-login/AccountProvider';
import { CanvasNodes } from './CanvasNodes';
import { EditorialReviewWorkspace } from './EditorialReviewWorkspace';
import { EditorialHistoryDialog } from './EditorialHistoryDialog';
import { Maximize, Trash2, Minus, Plus, MousePointer2, StickyNote, Pencil, MoveUpRight, Undo2, Redo2, LockKeyhole, UnlockKeyhole, Images } from 'lucide-react';
import { insertionY } from './canvasLayout';
import './canvas-workspace.css';
import type { Photo } from '../../domain/models';
import { CanvasPhotoPicker } from './CanvasPhotoPicker';

import type { AnnotationInfo, BoardInfo, CanvasPhoto, CanvasRepository } from '../../domain/canvas';

async function readCanvas(repository: CanvasRepository, boardId: string) {
  if (repository.getSnapshot) return repository.getSnapshot(boardId);
  const [nodes, annotations] = await Promise.all([repository.getNodes(boardId), repository.getAnnotations(boardId)]);
  return { nodes, annotations, board: undefined };
}
/** 无限画布：拖拽平移 + 滚轮缩放 + 照片节点拖拽放置。 */
export function NarrativeCanvas({ repository, projectId, selectedPhotoIds, photos = [], sequencePhotoIds = [], readOnly = false }: {
  repository?: CanvasRepository;
  projectId: string;
  selectedPhotoIds: string[];
  photos?: Photo[];
  sequencePhotoIds?: string[];
  readOnly?: boolean;
}) {
  const account = useAccount();
  const boardRequest = useUIStore(state => state.canvasBoardRequest);
  const requestedBoard = useUIStore(state => state.canvasBoardId);
  const openCanvasAgent = useUIStore(state => state.openCanvasAgent);
  const openDraftRevision = useUIStore(state => state.openDraftRevision);
  const [boards, setBoards] = useState<BoardInfo[]>([]);
  const [boardsLoaded,setBoardsLoaded]=useState(false);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [nodes, setNodes] = useState<CanvasPhoto[]>([]);
  const [creating, setCreating] = useState(false);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
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
  const [photoPickerOpen, setPhotoPickerOpen] = useState(false);
  const [removal, setRemoval] = useState<{ boardId: string; nodeIds: string[] } | null>(null);
  const [grouping, setGrouping] = useState<{ boardId: string; nodeIds: string[]; label: string } | null>(null);
  const [comment,setComment]=useState<{boardId:string;x:number;y:number;body:string}|null>(null);
  const [noteText, setNoteText] = useState('');
  const [adopting, setAdopting] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const [inkMode, setInkMode] = useState(false);
  const [inkPoints, setInkPoints] = useState<Array<{x:number;y:number}>>([]);
  const dragStart = useRef<{ x: number; y: number; nodeX: number; nodeY: number; positions: Map<string, { x: number; y: number }> } | null>(null);
  const panning = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const operationLock = useRef(false);
  const board = boards.find(value => value.id === activeBoard);
  const review = comparison?.boardId === activeBoard && comparison.value.draftRevision === board?.revision ? comparison.value : null;
  const editLocked = readOnly || activeBoard !== loadedBoard || pending.some(item => item.boardId === activeBoard) || Boolean(board?.owningTaskId) || board?.state === 'frozen';
  const selectedNodes = nodes.filter(node => node.visible !== false && selectedNodeIds.has(node.id));
  const selectionLocked=selectedNodes.length>0&&selectedNodes.every(node=>node.locked);
  const placedPhotoIds = nodes.filter(node => node.nodeType === 'photo' && node.visible !== false).flatMap(node => node.photoId ?? node.assetId ?? []);
  const canRemove = !editLocked && selectedNodes.length > 0 && selectedNodes.length <= 300 && selectedNodes.every(node => !node.locked);

  const run = async (action: () => Promise<void>) => {
    if (readOnly || operationLock.current) return;
    operationLock.current = true;
    setWorking(true);
    setNotice('');
    try { await action(); }
    catch (error) {
      setNotice(error instanceof Error ? error.message : '画布操作失败');
      if ((error as { code?: string })?.code === 'REVISION_CONFLICT') {
        try { await loadBoards(); } catch { /* 保留首次冲突提示，用户可重新打开画布。 */ }
      }
    }
    finally { operationLock.current = false; setPending(repository?.pendingCommands?.(projectId) ?? []); setWorking(false); }
  };
  // 节点跟随 activeBoard 变化加载（异步回调内 setState 合法）
  useEffect(() => {
    let active = true;
    if (repository) {
      repository.listBoards(projectId).then(list => {
        if (!active) return;
        setBoards(list);
        setBoardsLoaded(true);
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

  const placeSelected = async (photoIds: string[]) => {
    if (!repository || !activeBoard || editLocked || photoIds.length === 0) return;
    if (photoIds.length > 300 || photoIds.some(id => !photos.some(photo => photo.id === id))) throw new Error(zh.canvasPhotoInvalid);
    const current = await readCanvas(repository, activeBoard);
    const placed = new Set(current.nodes.filter(node => node.nodeType === 'photo' && node.visible !== false).flatMap(node => node.photoId ?? node.assetId ?? []));
    photoIds = [...new Set(photoIds)].filter(id => !placed.has(id));
    if (!photoIds.length) { setNodes(current.nodes); setNotice(zh.canvasPhotoAllPlaced); return; }
    const startX = -pan.x / zoom + 50;
    const startY = insertionY(current.nodes, startX, -pan.y / zoom + 50, Math.min(5, photoIds.length) * 240 - 20, Math.ceil(photoIds.length / 5) * 180 - 20);
    if (repository.placePhotos) await repository.placePhotos(projectId, activeBoard, photoIds.map((assetId, i) => ({ assetId, x: startX + (i % 5) * 240, y: startY + Math.floor(i / 5) * 180, width: 220, height: 160 })));
    else for (let i = 0; i < photoIds.length; i++) await repository.placePhoto(projectId, activeBoard, photoIds[i], startX + (i % 5) * 240, startY + Math.floor(i / 5) * 180, 220, 160);
    await loadBoards();
    setPhotoPickerOpen(false);
  };

  const removeSelected = async () => {
    if (!repository || !removal || removal.boardId !== activeBoard || editLocked) return;
    if (removal.nodeIds.some(id => !nodes.some(node => node.id === id && !node.locked && node.visible !== false))) throw new Error(zh.canvasRemoveLocked);
    if (repository.deleteNodes) await repository.deleteNodes(projectId, activeBoard, removal.nodeIds);
    else if (removal.nodeIds.length === 1) await repository.deleteNode(projectId, activeBoard, removal.nodeIds[0]);
    else throw new Error(zh.canvasRemoveUnavailable);
    setRemoval(null); setSelectedNodeIds(new Set()); setLinking(null);
    await loadBoards();
    setNotice(zh.canvasRemoved);
  };

  const toggleLockSelected = async () => {
    if (!repository || !activeBoard || editLocked || selectedNodes.length === 0) return;
    const target = !selectionLocked;
    if (repository.setNodesLocked) await repository.setNodesLocked(projectId, activeBoard, selectedNodes.map(node=>node.id), target);
    else for (const node of selectedNodes) await repository.setNodeLocked(projectId, activeBoard, node.id, target);
    const ns = await repository.getNodes(activeBoard);
    setNodes(ns);
  };
  const clearSelection=()=>{setSelectedNodeIds(new Set());containerRef.current?.focus({preventScroll:true});};

  const groupSelected = async () => {
    if (!repository || !grouping || grouping.boardId !== activeBoard || grouping.nodeIds.length < 2 || !grouping.label.trim() || editLocked) return;
    await repository.groupNodes(projectId, activeBoard, grouping.nodeIds, grouping.label.trim());
    setGrouping(null);
    setSelectedNodeIds(new Set());
    const ns = await repository.getNodes(activeBoard);
    setNodes(ns);
  };

  const addSticky = async () => {
    if (!repository || !activeBoard || !noteText.trim()) return;
    const x = -pan.x / zoom + 60;
    const y = insertionY(nodes, x, -pan.y / zoom + 60, 200, 120);
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
  const zoomAt = useCallback((value: number, mx: number, my: number) => {
    const newZoom = Math.min(4, Math.max(1e-8, value));
    setPan({
      x: mx - (mx - pan.x) * (newZoom / zoom),
      y: my - (my - pan.y) * (newZoom / zoom),
    });
    setZoom(newZoom);
  }, [pan.x, pan.y, zoom]);
  const zoomCenter = (value: number) => { const rect=containerRef.current?.getBoundingClientRect();if(rect)zoomAt(value,rect.width/2,rect.height/2); };
  useEffect(() => {
    const surface=containerRef.current;if(!surface)return;
    const wheel=(event:WheelEvent)=>{
      event.preventDefault();if(dragging||resizing||working)return;
      if(event.ctrlKey||event.metaKey){const rect=surface.getBoundingClientRect();zoomAt(zoom*(event.deltaY<0?1.1:0.9),event.clientX-rect.left,event.clientY-rect.top);}
      else setPan(previous=>({x:previous.x-event.deltaX,y:previous.y-event.deltaY}));
    };
    surface.addEventListener('wheel',wheel,{passive:false});return()=>surface.removeEventListener('wheel',wheel);
  },[repository,zoom,zoomAt,dragging,resizing,working]);

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
        containerRef.current?.focus({ preventScroll: true });
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

  const onContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!repository || !activeBoard || editLocked) return;
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const bx = (e.clientX - rect.left - pan.x) / zoom;
    const by = (e.clientY - rect.top - pan.y) / zoom;
    setNotice('');setComment({boardId:activeBoard,x:bx,y:by,body:''});
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
    if ((event.target as HTMLElement).closest('[role="dialog"]')) return;
    if ((event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
    if (event.key === 'Escape') { cancelGesture(); setSelectedNodeIds(new Set()); setLinkMode(false); setLinking(null); setInkMode(false); return; }
    if (editLocked) return;
    const key = event.key.toLowerCase();
    if ((event.metaKey||event.ctrlKey) && key==='a') {
      event.preventDefault();setSelectedNodeIds(new Set(nodes.filter(node=>node.visible!==false&&node.nodeType!=='arrow').map(node=>node.id)));
    } else if ((key === 'delete' || key === 'backspace') && !event.metaKey && !event.ctrlKey && !event.altKey && canRemove && activeBoard) {
      event.preventDefault();
      if (!working) { setNotice(''); setRemoval({ boardId: activeBoard, nodeIds: selectedNodes.map(node => node.id) }); }
    } else if ((event.metaKey || event.ctrlKey) && (key === 'z' || key === 'y')) {
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
        <select aria-label={zh.canvasChooseBoard} value={activeBoard ?? ''} disabled={working} onChange={e => { setNodes([]); setAnnotations([]); setNotice(''); setActiveBoard(e.target.value); setLinking(null); setLinkMode(false); setPan({ x: 0, y: 0 }); setZoom(1); }}>
          {boards.length === 0 && <option value="">{zh.canvasNoBoards}</option>}
          {boards.map(b => <option key={b.id} value={b.id}>{b.title}</option>)}
        </select>
        <Button size="small" disabled={creating || working || readOnly} onClick={() => void run(createBoard)}>{zh.canvasNewBoard}</Button>
        {board && !board.owningTaskId && repository.editorialHistory && <Button size="small" disabled={working} onClick={() => setHistoryOpen(true)}>{zh.editorialHistory}</Button>}
        {board?.owningTaskId && repository.simulateMerge && <Button size="small" disabled={working || !review} onClick={() => setReviewing(true)}>{zh.editorialOpen}</Button>}
        {board?.owningTaskId && <Button size="small" disabled={working || readOnly || Boolean(repository.compareDraft && !review)} onClick={() => setAdopting(true)}>{zh.editorialSaveIndependent}</Button>}
        <Button size="small" disabled={working || editLocked || !activeBoard || photos.length === 0} onClick={() => setPhotoPickerOpen(true)}>
          {zh.canvasAddPhotos}
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
      </div>
      <div className="canvas-toolbar canvas-edit-tools" role="group" aria-label={zh.canvasEditTools}>
        <div className="canvas-tool-cluster"><Button size="small" aria-pressed={!linkMode&&!inkMode} onClick={()=>{setLinkMode(false);setLinking(null);setInkMode(false);}}><MousePointer2 size={14}/>{zh.canvasSelectTool}</Button>
          <Button size="small" aria-expanded={noteOpen} disabled={working || editLocked || !activeBoard} onClick={() => setNoteOpen(!noteOpen)}><StickyNote size={14}/>{zh.canvasNoteTool}</Button>
          <Button size="small" aria-pressed={linkMode} disabled={working || editLocked || !activeBoard} onClick={() => { setLinkMode(!linkMode); setLinking(null); setInkMode(false); }}><MoveUpRight size={14}/>{zh.canvasConnectTool}</Button>
          <Button size="small" aria-pressed={inkMode} disabled={working || editLocked || !activeBoard} onClick={() => { setInkMode(!inkMode); setLinkMode(false); setLinking(null); }}><Pencil size={14}/>{zh.canvasDrawTool}</Button></div>
        <div className="canvas-tool-cluster"><Button size="small" disabled={working || editLocked || !activeBoard} onClick={() => void run(() => restoreHistory(false))}><Undo2 size={14}/>{zh.canvasUndo}</Button>
          <Button size="small" disabled={working || editLocked || !activeBoard} onClick={() => void run(() => restoreHistory(true))}><Redo2 size={14}/>{zh.canvasRedo}</Button></div>

      </div>
      {photoPickerOpen && <CanvasPhotoPicker photos={photos} sequencePhotoIds={sequencePhotoIds} selectedPhotoIds={selectedPhotoIds} placedPhotoIds={placedPhotoIds} busy={working} canAdd={!editLocked} error={notice} onClose={() => setPhotoPickerOpen(false)} onAdd={ids => void run(() => placeSelected(ids))} />}
      {removal && <Modal title={zh.canvasRemoveConfirm} description={zh.canvasRemoveHint.replace('{count}', String(removal.nodeIds.length))} onClose={() => { if (!working) setRemoval(null); }}>
        {notice && <p role="alert">{notice}</p>}
        <div className="dialog-actions"><Button disabled={working} onClick={() => setRemoval(null)}>{zh.canvasPhotoCancel}</Button><Button variant="primary" disabled={working || editLocked || removal.boardId !== activeBoard} onClick={() => void run(removeSelected)}>{working ? zh.canvasRemoving : zh.canvasRemoveConfirm}</Button></div>
      </Modal>}
      {grouping&&<Modal title={zh.canvasGroupTool} description={zh.canvasGroupHint.replace('{count}',String(grouping.nodeIds.length))} onClose={()=>{if(!working)setGrouping(null);}}><form onSubmit={event=>{event.preventDefault();void run(groupSelected);}}>
        <label className="canvas-group-label-input">{zh.canvasGroupName}<input aria-label={zh.canvasGroupName} maxLength={50} value={grouping.label} disabled={working} placeholder={zh.canvasGroupPlaceholder} onChange={event=>setGrouping({...grouping,label:event.target.value})}/></label>
        {notice&&<p role="alert">{notice}</p>}<div className="dialog-actions"><Button disabled={working} onClick={()=>setGrouping(null)}>{zh.canvasPhotoCancel}</Button><Button variant="primary" type="submit" disabled={working||editLocked||grouping.boardId!==activeBoard||!grouping.label.trim()}>{zh.canvasCreateGroup}</Button></div>
      </form></Modal>}
      {comment&&<Modal title={zh.canvasAddComment} onClose={()=>{if(!working)setComment(null);}}><form onSubmit={event=>{event.preventDefault();if(!repository||editLocked||comment.boardId!==activeBoard||!comment.body.trim())return;void run(async()=>{await repository.addAnnotation(projectId,comment.boardId,comment.x,comment.y,comment.body.trim());setComment(null);setAnnotations(await repository.getAnnotations(comment.boardId));});}}>
        <label className="canvas-group-label-input">{zh.canvasCommentContent}<textarea aria-label={zh.canvasCommentContent} maxLength={5000} value={comment.body} disabled={working} onChange={event=>setComment({...comment,body:event.target.value})}/></label>
        {notice&&<p role="alert">{notice}</p>}<div className="dialog-actions"><Button disabled={working} onClick={()=>setComment(null)}>{zh.canvasPhotoCancel}</Button><Button variant="primary" type="submit" disabled={working||editLocked||comment.boardId!==activeBoard||!comment.body.trim()}>{zh.canvasSaveComment}</Button></div>
      </form></Modal>}
      {pending.length > 0 && <div className="canvas-pending" role="status">
        <span>有画布操作尚未确认。确认后可继续编辑。</span>
        {pending.map(item => <Button key={item.boardId} size="small" disabled={working || !repository.retryPending} onClick={() => void run(async () => {
          await repository.retryPending?.(projectId, item.boardId);
          await loadBoards();
          setNotice('上次操作已确认');
        })}>确认上次操作{pending.length > 1 ? ` · ${boards.find(board => board.id === item.boardId)?.title ?? '新画布'}` : ''}</Button>)}
      </div>}
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
        <textarea aria-label={zh.canvasNoteContent} placeholder={zh.canvasNotePlaceholder} maxLength={5000} value={noteText} disabled={working || editLocked} onChange={event => setNoteText(event.target.value)} />
        <Button size="small" disabled={working || editLocked || !noteText.trim()} type="submit">{zh.canvasSaveNote}</Button>
        <Button size="small" disabled={working} onClick={() => setNoteOpen(false)}>取消</Button>
      </form>}
      <div className="canvas-stage">
      <div
        ref={containerRef}
        className={`canvas-surface ${inkMode ? "ink-mode" : ""}`}
        tabIndex={0}
        aria-label={zh.canvasTitle}
        aria-busy={Boolean(activeBoard&&activeBoard!==loadedBoard)}
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
        onContextMenu={onContextMenu}
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
            photoNames={Object.fromEntries(photos.map(photo=>[photo.id,photo.name]))}
            onKeyboardSelect={(id,extend)=>{if(editLocked)return;if(linkMode){void run(()=>onNodeClick(id));return;}setSelectedNodeIds(previous=>{const next=extend?new Set(previous):new Set<string>();if(!extend||!next.delete(id))next.add(id);return next;});}}
            onNodeClick={id => void run(() => onNodeClick(id))}
            onResize={(node, event) => {
              event.stopPropagation();
              if (!node.locked && !working) {
                setResizing({ nodeId: node.id, startW: node.width, startH: node.height, x: event.clientX, y: event.clientY });
                event.currentTarget.setPointerCapture(event.pointerId);
              }
            }} />
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

        </div>
      </div>
      {notice && <p role="status" className="canvas-notice">{notice}</p>}
      {boardsLoaded&&(!activeBoard||activeBoard===loadedBoard)&&!nodes.some(node=>node.visible!==false)&&<section className="canvas-start" aria-label={zh.canvasGetStarted}>
        <Images size={28} aria-hidden="true"/>
        <h3>{activeBoard?zh.canvasEmptyTitle:zh.canvasNoBoards}</h3>
        <p>{readOnly?zh.canvasViewOnly:activeBoard?zh.canvasStartPhotos:zh.canvasStartBoard}</p>
        {!readOnly&&(!activeBoard?<Button variant="primary" disabled={creating||working} onClick={()=>void run(createBoard)}>{zh.canvasCreateFirst}</Button>:!editLocked&&<div>
          <Button variant="primary" disabled={working||!photos.length} onClick={()=>setPhotoPickerOpen(true)}>{zh.canvasChooseFirstPhotos}</Button>
          <Button disabled={working} onClick={()=>setNoteOpen(true)}>{zh.canvasStartNote}</Button>
        </div>)}
      </section>}
        {selectedNodes.length>0&&<div className="canvas-selection-actions" role="group" aria-label={zh.canvasSelectionActions}>
          <span>{zh.canvasSelectedCount.replace('{count}',String(selectedNodes.length))}</span>
          <Button size="small" title={selectedNodes.length>100?zh.canvasGroupLimit:undefined} disabled={working || editLocked || selectedNodes.length<2 || selectedNodes.length>100 || selectedNodes.some(node=>node.locked)} onClick={() => { if(activeBoard)setGrouping({boardId:activeBoard,nodeIds:selectedNodes.map(node=>node.id),label:''}); }}>{zh.canvasGroupTool}</Button>
          <Button size="small" disabled={working || editLocked} onClick={() => void run(toggleLockSelected)}>{selectionLocked?<UnlockKeyhole size={14}/>:<LockKeyhole size={14}/>} {selectionLocked?zh.canvasUnlockSelection:zh.canvasLockSelection}</Button>
          <Button size="small" disabled={working || !canRemove} title={selectedNodes.some(node => node.locked) ? zh.canvasRemoveLocked : selectedNodes.length>300?zh.canvasRemoveLimit:zh.canvasRemoveShortcut} onClick={() => { if (activeBoard) { setNotice(''); setRemoval({ boardId: activeBoard, nodeIds: selectedNodes.map(node => node.id) }); } }}><Trash2 size={14} />{zh.canvasRemove}{` (${selectedNodes.length})`}</Button>
          <Button size="small" disabled={working} onClick={clearSelection}>{zh.canvasClearSelection}</Button>
        </div>}
      </div>
      <footer className="canvas-statusbar"><span>{readOnly?zh.canvasViewOnly:linkMode?(linking?zh.canvasChooseConnectionEnd:zh.canvasChooseConnectionStart):inkMode?zh.canvasDrawingHint:zh.canvasNavigationHint}</span><div className="canvas-zoom-controls" role="group" aria-label={zh.canvasZoom}>
        <Button size="small" aria-label={zh.canvasZoomOut} disabled={working} onClick={()=>zoomCenter(zoom/1.2)}><Minus size={14}/></Button>
        <Button size="small" aria-label={zh.canvasResetZoom} disabled={working} onClick={()=>zoomCenter(1)}>{zoom<0.1?(zoom*100).toPrecision(2):Math.round(zoom*100)}%</Button>
        <Button size="small" aria-label={zh.canvasZoomIn} disabled={working} onClick={()=>zoomCenter(zoom*1.2)}><Plus size={14}/></Button>
        <Button size="small" aria-label={zh.chatBoardFit} title={zh.chatBoardFit} disabled={working||!nodes.length} onClick={fitContent}><Maximize size={14}/></Button>
      </div></footer>
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
