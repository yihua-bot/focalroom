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

/** 画布指针工具：一次只允许一个，切换即退出上一个。 */
type CanvasTool = 'select' | 'link' | 'ink' | 'note';
/** 便签草稿：nodeId 为 null 表示尚未落地的新建。 */
type StickyDraft = { nodeId: string | null; x: number; y: number; text: string };

async function readCanvas(repository: CanvasRepository, boardId: string) {
  if (repository.getSnapshot) return repository.getSnapshot(boardId);
  const [nodes, annotations] = await Promise.all([repository.getNodes(boardId), repository.getAnnotations(boardId)]);
  return { nodes, annotations, board: undefined };
}
function stickyText(node: CanvasPhoto | undefined) {
  try { return String(JSON.parse(node?.contentJson ?? '{}').text ?? ''); } catch { return ''; }
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
  const [tool, setTool] = useState<CanvasTool>('select');
  const [linking, setLinking] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const [working, setWorking] = useState(false);
  const [pending, setPending] = useState<Array<{ boardId: string }>>([]);
  const [loadedBoard, setLoadedBoard] = useState<string | null>(null);
  const [comparison, setComparison] = useState<{ boardId: string; value: Awaited<ReturnType<NonNullable<CanvasRepository['compareDraft']>>> } | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [showBaseline, setShowBaseline] = useState(true);
  const [stickyDraft, setStickyDraft] = useState<StickyDraft | null>(null);
  const [ask, setAsk] = useState<{ kind: 'group' | 'comment'; value: string; x: number; y: number } | null>(null);
  const [openComment, setOpenComment] = useState<AnnotationInfo | null>(null);
  const [adopting, setAdopting] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const [inkPoints, setInkPoints] = useState<Array<{x:number;y:number}>>([]);
  // 连接/画笔/便签互斥，共用一个 tool；linkMode/inkMode/noteMode 只读派生，避免出现两个真源。
  const linkMode = tool === 'link';
  const inkMode = tool === 'ink';
  const noteMode = tool === 'note';
  const dragStart = useRef<{ x: number; y: number; nodeX: number; nodeY: number; positions: Map<string, { x: number; y: number }> } | null>(null);
  const panning = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  // 便签落点先记在这里，pointerup 才挂编辑框：mousedown 会把焦点交给 tabIndex=0 的画布，
  // 若 pointerdown 就挂载 autoFocus 的输入框，紧随其后的失焦会把这个空便签当成"用户放弃了"丢掉。
  const noteDrop = useRef<{ x: number; y: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const toBoard = (clientX: number, clientY: number) => {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return null;
    return { x: (clientX - rect.left - pan.x) / zoom, y: (clientY - rect.top - pan.y) / zoom };
  };
  const board = boards.find(value => value.id === activeBoard);
  const review = comparison?.boardId === activeBoard && comparison.value.draftRevision === board?.revision ? comparison.value : null;
  const editLocked = readOnly || activeBoard !== loadedBoard || pending.some(item => item.boardId === activeBoard) || Boolean(board?.owningTaskId) || board?.state === 'frozen';

  const exitTool = () => { setTool('select'); setLinking(null); setInkPoints([]); };

  const run = async (action: () => Promise<void>) => {
    if (readOnly || working) return;
    setWorking(true);
    setNotice('');
    try { await action(); }
    catch (error) {
      setNotice(error instanceof Error ? error.message : zh.canvasActionFailed);
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
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : zh.canvasLoadFailed); });
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
      }).catch(error => { if (active) setNotice(error instanceof Error ? error.message : zh.canvasPhotoLoadFailed); });
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
        .catch(error => { if (active) setNotice(error instanceof Error ? error.message : zh.canvasPlanLoadFailed); });
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
    if (!changed) setNotice(redo ? zh.canvasNothingToRedo : zh.canvasNothingToUndo);
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
    if (selectedPhotoIds.length > 300) throw new Error(zh.canvasPhotoLimit.replace('{count}', '300'));
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

  const groupSelected = async (label: string) => {
    if (!repository || !activeBoard || selectedNodeIds.size < 2) return;
    await repository.groupNodes(projectId, activeBoard, Array.from(selectedNodeIds), label);
    setSelectedNodeIds(new Set());
    const ns = await repository.getNodes(activeBoard);
    setNodes(ns);
  };

  const deleteSelected = async () => {
    if (!repository || !activeBoard || editLocked || selectedNodeIds.size === 0) return;
    const ids = [...selectedNodeIds];
    const deletable = ids.filter(id => !nodes.find(node => node.id === id)?.locked);
    for (const id of deletable) await repository.deleteNode(projectId, activeBoard, id);
    if (linking && deletable.includes(linking)) exitTool();
    setSelectedNodeIds(new Set());
    await loadBoards();
    const skipped = ids.length - deletable.length;
    const parts: string[] = [];
    if (deletable.length) parts.push(zh.canvasDeleteDone.replace('{count}', String(deletable.length)));
    if (skipped) parts.push(zh.canvasDeleteSkipped.replace('{count}', String(skipped)));
    if (parts.length) setNotice(parts.join(' · '));
  };

  // Esc 撤掉正在写的便签并退回选择工具。
  const cancelSticky = () => { setStickyDraft(null); setTool('select'); };

  const blurSticky = () => { if (stickyDraft) void run(() => commitSticky(stickyDraft)); };

  const editSticky = (node: CanvasPhoto) => { if (editLocked) return; setStickyDraft({ nodeId: node.id, x: node.x, y: node.y, text: stickyText(node) }); };

  // 卸载聚焦中的 textarea 不会再派发 blur，所以按钮提交与失焦提交只会命中一条路径，无需额外去重。
  const commitSticky = async (draft: StickyDraft) => {
    setStickyDraft(null);
    if (!repository || !activeBoard || editLocked) return;
    const text = draft.text.trim();
    if (draft.nodeId === null) {
      if (!text) return;
      await repository.addText(projectId, activeBoard, 'sticky', draft.x, draft.y, 200, 120, text, '#ffe082');
    } else if (!text) {
      // 清空即丢弃：内容命令不接受空正文，留下一个看不见的节点只会挡住后面的点击。
      await repository.deleteNode(projectId, activeBoard, draft.nodeId);
    } else if (text !== stickyText(nodes.find(node => node.id === draft.nodeId))) {
      await repository.setNodeText(projectId, activeBoard, draft.nodeId, text);
    }
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
  // React 的 onWheel 以 passive 方式挂载，preventDefault 会被忽略并连带滚动外层容器，缩放必须用原生非 passive 监听。
  // 每次缩放重新登记监听以拿到当前视口，代价只是两次 add/removeEventListener。
  useEffect(() => {
    const surface = containerRef.current;
    if (!surface) return;
    const handle = (event: WheelEvent) => {
      event.preventDefault();
      const rect = surface.getBoundingClientRect();
      // 1e-8 的下限能让画布缩成一个点且无从恢复；收到 5% 足够看清全局布局。
      const next = Math.min(4, Math.max(0.05, zoom * (event.deltaY < 0 ? 1.1 : 0.9)));
      const mx = event.clientX - rect.left;
      const my = event.clientY - rect.top;
      // 保持鼠标指向的画布点不变：screen = pan + zoom * board
      setPan({ x: mx - (mx - pan.x) * (next / zoom), y: my - (my - pan.y) * (next / zoom) });
      setZoom(next);
    };
    surface.addEventListener('wheel', handle, { passive: false });
    return () => surface.removeEventListener('wheel', handle);
  }, [zoom, pan]);

  // ── 交互：节点连接 / 便签落点 / 拖拽平移 ──
  const onNodeClick = async (nodeId: string) => {
    if (!linkMode || editLocked) return;
    if (linking === nodeId) { setLinking(null); return; }
    if (linking) {
      if (repository && activeBoard) {
        await repository.addArrow(projectId, activeBoard, linking, nodeId);
        const ns = await repository.getNodes(activeBoard);
        setNodes(ns);
      }
      exitTool();
    } else setLinking(nodeId);
  };

  // pointerdown 只记落点，编辑框到 pointerup 才挂载：那时 mousedown 的焦点转移已经结束，autoFocus 不会被画布抢回去。
  const dropNote = () => {
    const point = noteDrop.current;
    if (!point) return false;
    noteDrop.current = null;
    // 按住的过程中工具已经变了，说明这次按压作废（指针在别处松开），不该凭空落下便签。
    if (!noteMode) return false;
    setStickyDraft({ nodeId: null, x: point.x, y: point.y, text: '' });
    setTool('select');
    return true;
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (working) return;
    const target = e.target as HTMLElement;
    if (target.closest('textarea,input,select,[contenteditable="true"]')) return;
    const nodeEl = target.closest('[data-node-id]');
    if (noteMode && e.button === 0 && !nodeEl) {
      noteDrop.current = toBoard(e.clientX, e.clientY);
      return;
    }
    // 连接模式：点节点交给 click 处理，点空白即退出并照常平移——原先这里整体 return，用户会困在模式里出不来。
    if (linkMode) {
      if (nodeEl) return;
      exitTool();
    }
    if (inkMode && e.button === 0) {
      if (editLocked) return;
      const point = toBoard(e.clientX, e.clientY);
      if (point) setInkPoints([point]);
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      return;
    }
    if (e.button === 1 || e.button === 0) {
      if (nodeEl && e.button === 0) {
        const nodeId = (nodeEl as HTMLElement).dataset.nodeId!;
        const node = nodes.find(n => n.id === nodeId);
        if (node && !editLocked) {
          const selection = e.shiftKey ? new Set(selectedNodeIds) : selectedNodeIds.has(nodeId) ? new Set(selectedNodeIds) : new Set([nodeId]);
          if (e.shiftKey) { if (selection.has(nodeId)) selection.delete(nodeId); else selection.add(nodeId); }
          setSelectedNodeIds(selection);
          // 连线与墨迹的落点由端点决定，本身不参与拖拽。
          if (node.locked || node.nodeType === 'arrow' || node.nodeType === 'ink' || e.shiftKey || dragging) return;
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

  // 右键 = 在光标处钉一条评论；用应用内输入框替代浏览器 prompt()（原生弹窗无法四语、且与产品视觉割裂）。
  const onContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!repository || !activeBoard || editLocked) return;
    const point = toBoard(e.clientX, e.clientY);
    if (point) setAsk({ kind: 'comment', value: '', x: point.x, y: point.y });
  };

  const submitAsk = async () => {
    const current = ask;
    const value = current?.value.trim();
    if (!current || !value) { setAsk(null); return; }
    setAsk(null);
    if (current.kind === 'group') await groupSelected(value);
    else {
      if (!repository || !activeBoard) return;
      await repository.addAnnotation(projectId, activeBoard, current.x, current.y, value);
      const ans = await repository.getAnnotations(activeBoard);
      setAnnotations(ans);
    }
  };

  const resolveComment = async (annotation: AnnotationInfo, resolved: boolean) => {
    if (!repository) return;
    await repository.resolveAnnotation(projectId, annotation.id, resolved);
    setOpenComment(null);
    if (activeBoard) setAnnotations(await repository.getAnnotations(activeBoard));
  };

  const cancelGesture = () => {
    const start = dragStart.current;
    if (start) setNodes(current => current.map(node => {
      const origin = start.positions.get(node.id);
      return origin ? { ...node, ...origin } : node;
    }));
    if (resizing) setNodes(current => current.map(node => node.id === resizing.nodeId ? { ...node, width: resizing.startW, height: resizing.startH } : node));
    setResizing(null); setMarquee(null); setDragging(null);
    dragStart.current = null; panning.current = null; noteDrop.current = null; setInkPoints([]);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if ((event.target as HTMLElement).closest('input,textarea,select,[contenteditable="true"]')) return;
    if (event.key === 'Escape') {
      // Esc 先退进行中的工具/浮层，再回滚手势；preventDefault 挡住全局 clearSelection 顺带清掉照片选区。
      event.preventDefault();
      if (stickyDraft) { cancelSticky(); return; }
      if (ask) { setAsk(null); return; }
      if (openComment) { setOpenComment(null); return; }
      if (tool !== 'select') { exitTool(); return; }
      cancelGesture();
      return;
    }
    if ((event.key === 'Delete' || event.key === 'Backspace') && selectedNodeIds.size > 0) { event.preventDefault(); void run(deleteSelected); return; }
    if ((event.metaKey || event.ctrlKey) && event.key === '0') { event.preventDefault(); fitContent(); return; }
    if ((event.metaKey || event.ctrlKey) && event.key === '1') { event.preventDefault(); setZoom(1); return; }
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
    return <div className="canvas-empty"><p>{zh.canvasDesktopOnly}</p></div>;
  }

  return (
    <div className="canvas-wrapper" onKeyDown={onKeyDown}>
      <div className="canvas-toolbar" role="toolbar" aria-label={zh.canvasToolbar}>
        <select aria-label={zh.canvasBoardPick} value={activeBoard ?? ''} disabled={working} onChange={e => { setNodes([]); setAnnotations([]); setNotice(''); setActiveBoard(e.target.value); exitTool(); setStickyDraft(null); setPan({ x: 0, y: 0 }); setZoom(1); }}>
          {boards.length === 0 && <option value="">{zh.canvasNoBoard}</option>}
          {boards.map(b => <option key={b.id} value={b.id}>{b.title}</option>)}
        </select>
        <span className="canvas-tool-group">
          <Button size="small" disabled={creating || working || readOnly} onClick={() => void run(createBoard)}>{zh.canvasNewBoard}</Button>
          {board && !board.owningTaskId && repository.editorialHistory && <Button size="small" disabled={working} onClick={() => setHistoryOpen(true)}>{zh.editorialHistory}</Button>}
          {board?.owningTaskId && repository.simulateMerge && <Button size="small" disabled={working || !review} onClick={() => setReviewing(true)}>{zh.editorialOpen}</Button>}
          {board?.owningTaskId && <Button size="small" disabled={working || readOnly || Boolean(repository.compareDraft && !review)} onClick={() => setAdopting(true)}>{zh.editorialSaveIndependent}</Button>}
        </span>
        <span className="canvas-tool-sep" aria-hidden="true" />
        <span className="canvas-tool-group">
          <Button size="small" title={selectedPhotoIds.length ? zh.canvasPlaceHint : `${zh.canvasPlaceHint} · ${zh.canvasNoSelection}`}
            disabled={working || editLocked || !activeBoard || selectedPhotoIds.length === 0} onClick={() => void run(placeSelected)}>
            {zh.canvasPlaceSelected}（{selectedPhotoIds.length}）
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
        </span>
        <span className="canvas-tool-sep" aria-hidden="true" />
        <span className="canvas-tool-group">
          <Button size="small" aria-pressed={noteMode} title={zh.canvasStickyHint} disabled={working || editLocked || !activeBoard}
            onClick={() => { setStickyDraft(null); setTool(noteMode ? 'select' : 'note'); }}>{zh.canvasSticky}</Button>
          <Button size="small" aria-pressed={linkMode} title={zh.canvasLinkHint} disabled={working || editLocked || !activeBoard}
            onClick={() => { setLinking(null); setTool(linkMode ? 'select' : 'link'); }}>{zh.canvasLink}</Button>
          <Button size="small" aria-pressed={inkMode} title={zh.canvasInkHint} disabled={working || editLocked || !activeBoard}
            onClick={() => { setInkPoints([]); setTool(inkMode ? 'select' : 'ink'); }}>{zh.canvasInk}</Button>
        </span>
        <span className="canvas-tool-sep" aria-hidden="true" />
        <span className="canvas-tool-group">
          <Button size="small" title={selectedNodeIds.size < 2 ? zh.canvasNeedTwoNodes : zh.canvasGroup}
            disabled={working || editLocked || !activeBoard || selectedNodeIds.size < 2} onClick={() => setAsk({ kind: 'group', value: '', x: 0, y: 0 })}>{zh.canvasGroup}</Button>
          <Button size="small" title={selectedNodeIds.size === 0 ? zh.canvasNeedNodes : zh.canvasDeleteHint}
            disabled={working || editLocked || !activeBoard || selectedNodeIds.size === 0} onClick={() => void run(deleteSelected)}>{zh.canvasDelete}</Button>
          <Button size="small" title={selectedNodeIds.size === 0 ? zh.canvasNeedNodes : zh.canvasLock}
            disabled={working || editLocked || !activeBoard || selectedNodeIds.size === 0} onClick={() => void run(toggleLockSelected)}>{zh.canvasLock}</Button>
        </span>
        <span className="canvas-tool-sep" aria-hidden="true" />
        <span className="canvas-tool-group">
          <Button size="small" disabled={working || editLocked || !activeBoard} onClick={() => void run(() => restoreHistory(false))}>{zh.canvasUndo}</Button>
          <Button size="small" disabled={working || editLocked || !activeBoard} onClick={() => void run(() => restoreHistory(true))}>{zh.canvasRedo}</Button>
          <Button size="small" aria-label={zh.chatBoardFit} title={`${zh.chatBoardFit} · ⌘0`} disabled={working || !nodes.length} onClick={fitContent}><Maximize size={14} /></Button>
        </span>
        <span className="canvas-zoom-label">{Math.round(zoom * 100)}%</span>
      </div>
      {(tool !== 'select' || stickyDraft) && <div className="canvas-tool-status" role="status">
        <span>{tool === 'link' ? (linking ? zh.canvasLinkTo : zh.canvasLinkFrom) : tool === 'ink' ? zh.canvasInkHint : tool === 'note' ? zh.canvasStickyHint : zh.canvasStickyEditing}</span>
        {/* 便签编辑器自带保存/取消；这里再放一个取消会出现两个同名按钮，也会抢走失焦提交。 */}
        {tool !== 'select' && <Button size="small" variant="quiet" onMouseDown={event => event.preventDefault()} onClick={exitTool}>{zh.canvasCancel}</Button>}
      </div>}
      {pending.length > 0 && <div className="canvas-pending" role="status">
        <span>{zh.canvasPending}</span>
        {pending.map(item => <Button key={item.boardId} size="small" disabled={working || !repository.retryPending} onClick={() => void run(async () => {
          await repository.retryPending?.(projectId, item.boardId);
          await loadBoards();
          setNotice(zh.canvasPendingConfirmed);
        })}>{zh.canvasRetryConfirm}{pending.length > 1 ? ` · ${boards.find(board => board.id === item.boardId)?.title ?? ''}` : ''}</Button>)}
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
      <div
        ref={containerRef}
        className={`canvas-surface${inkMode ? " ink-mode" : ""}${linkMode ? " link-mode" : ""}${noteMode ? " note-mode" : ""}`}
        tabIndex={0}
        aria-label={zh.canvasSurface}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={() => {
          panning.current = null;
          if (dropNote()) return;
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
            linkSource={linking} onTextChange={editSticky}
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
          {stickyDraft && (() => {
            const origin = stickyDraft.nodeId ? nodes.find(node => node.id === stickyDraft.nodeId) : undefined;
            return <form className="canvas-sticky-editor" style={{ left: stickyDraft.x, top: stickyDraft.y, width: origin?.width ?? 200, height: origin?.height ?? 120 }}
              onSubmit={event => { event.preventDefault(); void run(() => commitSticky(stickyDraft)); }}>
              <textarea aria-label={zh.canvasNoteLabel} placeholder={zh.canvasNotePlaceholder} maxLength={5000} autoFocus
                disabled={working} value={stickyDraft.text}
                onChange={event => setStickyDraft(draft => draft ? { ...draft, text: event.target.value } : draft)}
                onBlur={blurSticky}
                onKeyDown={event => {
                  if (event.key === 'Escape') { event.preventDefault(); cancelSticky(); }
                  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') { event.preventDefault(); void run(() => commitSticky(stickyDraft)); }
                }} />
              <span className="canvas-sticky-editor-actions">
                <Button size="small" type="submit" disabled={working || !stickyDraft.text.trim()} onMouseDown={event => event.preventDefault()}>{zh.canvasSave}</Button>
                <Button size="small" variant="quiet" disabled={working} onMouseDown={event => event.preventDefault()} onClick={cancelSticky}>{zh.canvasCancel}</Button>
              </span>
            </form>;
          })()}
          {annotations.filter(annotation => !annotation.resolved).map(a => (
            <button key={a.id} type="button" className="canvas-annotation-pin" title={a.body} aria-label={zh.canvasAnnotation}
              style={{ left: a.x ?? 0, top: a.y ?? 0 }}
              onPointerDown={event => event.stopPropagation()} onClick={() => setOpenComment(a)}>
              💬
            </button>
          ))}
          {nodes.length === 0 && activeBoard && (
            <div className="canvas-hint" style={{ left: -pan.x / zoom + 40, top: -pan.y / zoom + 40 }}>
              {zh.canvasEmpty}
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
        <Button disabled={working} onClick={() => setAdopting(false)}>{zh.canvasCancel}</Button>
      </Modal>}
      {ask && <Modal title={ask.kind === 'group' ? zh.canvasGroupTitle : zh.canvasCommentTitle}
        description={ask.kind === 'group' ? zh.canvasGroupHint : zh.canvasCommentHint} onClose={() => setAsk(null)}>
        <textarea className="canvas-ask-input" aria-label={ask.kind === 'group' ? zh.canvasGroupLabel : zh.canvasCommentBody}
          placeholder={ask.kind === 'group' ? zh.canvasGroupPlaceholder : zh.canvasCommentPlaceholder} maxLength={2000} autoFocus
          value={ask.value} onChange={event => setAsk({ ...ask, value: event.target.value })}
          onKeyDown={event => { if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) { event.preventDefault(); void run(submitAsk); } }} />
        <div className="dialog-actions">
          <Button variant="primary" disabled={working || !ask.value.trim()} onClick={() => void run(submitAsk)}>{ask.kind === 'group' ? zh.canvasGroup : zh.canvasCommentAdd}</Button>
          <Button disabled={working} onClick={() => setAsk(null)}>{zh.canvasCancel}</Button>
        </div>
      </Modal>}
      {openComment && <Modal title={zh.canvasAnnotation} description={openComment.author} onClose={() => setOpenComment(null)}>
        <p className="canvas-comment-body">{openComment.body}</p>
        <div className="dialog-actions">
          <Button variant="primary" disabled={working || editLocked} onClick={() => void run(() => resolveComment(openComment, true))}>{zh.canvasAnnotationResolve}</Button>
          <Button disabled={working} onClick={() => setOpenComment(null)}>{zh.close}</Button>
        </div>
      </Modal>}
    </div>
  );
}
