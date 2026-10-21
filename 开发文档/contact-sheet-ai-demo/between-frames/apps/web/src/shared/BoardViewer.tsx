// G6 Web Board Viewer：协作者在浏览器中查看/批注画布。
// 只读画布 + 评论添加/解决；编辑仍在桌面端进行。
import { useEffect, useRef, useState } from 'react';
import { request, type Session } from '../shared-api';

interface BoardInfo { id: string; project_id: string; title: string; state: string }
interface NodeInfo {
  id: string; board_id: string; node_type: string;
  x: number; y: number; width: number; height: number;
  z_order: number; asset_id: string | null;
  content_json: unknown; style_json: unknown;
}
interface AnnotationInfo {
  id: string; board_id: string; x: number; y: number;
  author: string; body: string; resolved: boolean; created_at: string;
}

export function BoardViewer({ session, projectId, onBack }: {
  session: Session;
  projectId: string;
  onBack: () => void;
}) {
  const [boards, setBoards] = useState<BoardInfo[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [nodes, setNodes] = useState<NodeInfo[]>([]);
  const [annotations, setAnnotations] = useState<AnnotationInfo[]>([]);
  const [previewUrls, setPreviewUrls] = useState<Record<string, string>>({});
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(0.7);
  const containerRef = useRef<HTMLDivElement>(null);
  const panning = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);

  useEffect(() => {
    request<BoardInfo[]>('/boards', session, 'GET').catch(() => {}).then(list => {
      if (list) {
        setBoards(list);
        if (list.length > 0) setActiveBoard(list[0].id);
      }
    });
  }, [session, projectId]);

  useEffect(() => {
    if (!activeBoard) return;
    let active = true;
    // 获取节点和评论
    request<NodeInfo[]>(`/boards/${activeBoard}/nodes`, session, 'GET')
      .then(async ns => {
        if (!active || !ns) return;
        setNodes(ns);
        // 加载照片节点的预览 URL
        const assetIds = ns.filter(n => n.node_type === 'photo' && n.asset_id).map(n => n.asset_id!);
        const urls: Record<string, string> = {};
        for (const assetId of assetIds.slice(0, 50)) {
          try {
            const resp = await request<{url?: string}>(`/assets/${assetId}/preview-url`, session, 'GET').catch(() => null);
            if (resp?.url) urls[assetId] = resp.url;
          } catch { /* 逐张容错 */ }
        }
        if (active) setPreviewUrls(urls);
      }).catch(() => {});
    request<AnnotationInfo[]>(`/boards/${activeBoard}/annotations`, session, 'GET')
      .then(ans => { if (active) setAnnotations(ans ?? []); }).catch(() => {});
    return () => { active = false; };
  }, [session, activeBoard]);

  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    setZoom(z => Math.min(3, Math.max(0.15, z * factor)));
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button === 0) {
      panning.current = { x: e.clientX, y: e.clientY, panX: pan.x, panY: pan.y };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    }
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (panning.current) {
      setPan({
        x: panning.current.panX + (e.clientX - panning.current.x),
        y: panning.current.panY + (e.clientY - panning.current.y),
      });
    }
  };
  const onPointerUp = () => { panning.current = null; };

  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!activeBoard) return;
    const input = prompt('添加评论：');
    if (!input?.trim()) return;
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    const bx = (e.clientX - rect.left - pan.x) / zoom;
    const by = (e.clientY - rect.top - pan.y) / zoom;
    await request('/boards/annotations', session, 'POST', {
      boardId: activeBoard, x: bx, y: by, body: input.trim(),
    }).catch(() => {});
    const ans = await request<AnnotationInfo[]>(`/boards/${activeBoard}/annotations`, session, 'GET').catch(() => []);
    if (ans) setAnnotations(ans);
  };

  const resolveAnnotation = async (id: string, resolved: boolean) => {
    await request(`/boards/annotations/${id}/resolve`, session, 'POST', { resolved }).catch(() => {});
    setAnnotations(prev => prev.map(a => a.id === id ? { ...a, resolved } : a));
  };

  return <div className="board-viewer">
    <div className="board-toolbar">
      <button className="fr-text" onClick={onBack}>← 返回项目</button>
      <select value={activeBoard ?? ''} onChange={e => setActiveBoard(e.target.value)}>
        {boards.length === 0 && <option value="">无画布</option>}
        {boards.map(b => <option key={b.id} value={b.id}>{b.title} ({b.state})</option>)}
      </select>
      <span className="board-zoom">{Math.round(zoom * 100)}%</span>
    </div>
    <div
      ref={containerRef}
      className="board-surface"
      onWheel={onWheel}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerLeave={onPointerUp}
      onContextMenu={e => void onContextMenu(e)}
    >
      <div style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, transformOrigin: '0 0', position: 'absolute' }}>
        {nodes.filter(n => n.node_type === 'photo').map(node => (
          <div key={node.id} className="board-photo" style={{
            left: node.x, top: node.y, width: node.width, height: node.height, zIndex: node.z_order,
          }}>
            {node.asset_id && previewUrls[node.asset_id]
              ? <img src={previewUrls[node.asset_id]} alt="" style={{width:'100%',height:'100%',objectFit:'cover',borderRadius:'2px'}} />
              : <div className="board-photo-placeholder">📷</div>}
          </div>
        ))}
        {nodes.filter(n => n.node_type === 'sticky' || n.node_type === 'text').map(node => {
          const content = node.content_json && typeof node.content_json === 'object'
            ? String((node.content_json as Record<string, unknown>).text ?? '') : '';
          return <div key={node.id} className="board-sticky" style={{
            left: node.x, top: node.y, width: node.width, height: node.height, zIndex: node.z_order,
          }}>{content}</div>;
        })}
        {annotations.map(a => !a.resolved && (
          <div key={a.id} className="board-annotation" style={{ left: a.x, top: a.y }} title={`${a.author}: ${a.body}`}>
            💬
          </div>
        ))}
      </div>
      {annotations.length > 0 && (
        <div className="board-annotation-list">
          {annotations.map(a => (
            <div key={a.id} className={`board-annotation-item ${a.resolved ? 'resolved' : ''}`}>
              <strong>{a.author}</strong>
              <p>{a.body}</p>
              {!a.resolved
                ? <button onClick={() => void resolveAnnotation(a.id, true)}>✓ 解决</button>
                : <button onClick={() => void resolveAnnotation(a.id, false)}>↩ 重开</button>}
            </div>
          ))}
        </div>
      )}
    </div>
  </div>;
}
