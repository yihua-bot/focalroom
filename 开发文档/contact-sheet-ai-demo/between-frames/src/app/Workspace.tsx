import { Columns3, PanelRightClose, PanelRightOpen, Film, Search, Copy } from 'lucide-react';
import type { SelectionGesture } from '../domain/selection';
import type { Photo, PhotoFilter, PhotoStatus, Project } from '../domain/models';
import { groupPhotos, type GridGrouping } from '../domain/grouping';
import { useUIStore } from '../stores/uiStore';
import { NarrativeCanvas } from '../features/canvas/NarrativeCanvas';
import { PublicationPanel } from '../features/publishing/PublicationPanel';
import { zh } from './copy';
import { ProjectHeader } from '../features/project/ProjectHeader';
import { ContactSheetGrid } from '../components/contact-sheet/ContactSheetGrid';
import { SequenceBoard } from '../components/sequence/SequenceBoard';
import { SelectionDock } from '../components/sequence/SelectionDock';
import { EmptyState } from '../components/shared/EmptyState';
import { Button } from '../components/shared/Button';
import { SharedReviewLink } from '../components/shared/SharedReviewLink';
import type { CanvasRepository } from '../domain/canvas';
import type { PublicationRepository } from '../domain/publication';

export function Workspace({ project, visible, kept, active, selectedIds, busy, onEdit, onImport, onCompare, onExport, onDecide, onBatchDecide, onBatchRate, onSelect, onReorder, onEnlarge, onUndo, onRedo, onVersions, onActivity, onCopyPlan, onAICompare, onAIOrder, local = false, sharing, canvas, publication }: {
  project: Project; visible: Photo[]; kept: Photo[]; active?: Photo; busy: boolean;
  selectedIds: string[]; onSelect: (id: string, gesture?: SelectionGesture) => void;
  onBatchDecide: (status: PhotoStatus) => void; onBatchRate: (rating: number) => void;
  onEdit: () => void; onImport: () => void; onCompare: () => void; onExport: () => void;
  onDecide: (id: string, status: PhotoStatus) => void; onReorder: (ids: string[]) => void; onEnlarge: (photo: Photo) => void;
  onUndo: () => void; onRedo: () => void; onVersions: () => void; onActivity: () => void; onCopyPlan: () => void;
  onAICompare?: () => void; onAIOrder?: () => void;
  local?: boolean;
  sharing?: import('../domain/sharing').SharingStatus;
  canvas?: CanvasRepository;
  publication?: PublicationRepository;
}) {
  const ui = useUIStore();
  const filters: PhotoFilter[] = ['all', 'keep', 'undecided', 'reject'];
  const readOnly = !!sharing && sharing.authority !== 'local';
  return <><ProjectHeader project={project} onEdit={onEdit} onImport={onImport} onAI={ui.openAgent} readOnly={readOnly} />
    {readOnly && <div className="preview-banner">{sharing?.authority === 'shared' ? '共享项目 · 桌面只读，请在 Web 继续评片。' : '迁移未完成 · 编辑已暂停，请在导出窗口继续或取消迁移。'}{sharing?.remote?.webUrl && <SharedReviewLink projectId={project.id} url={sharing.remote.webUrl} />}</div>}
    {(ui.view === 'grid' || ui.view === 'sequence') && <div className="history-toolbar" role="group" aria-label="编辑历史">
      <Button size="small" disabled={busy || !project.history?.undoGroupId} onClick={onUndo} title="⌘ / Ctrl Z">撤销</Button>
      <Button size="small" disabled={busy || !project.history?.redoGroupId} onClick={onRedo} title="⌘ / Ctrl Shift Z">重做</Button>
      <Button size="small" disabled={busy} onClick={onVersions}>作品版本{project.versions?.length ? ` · ${project.versions.length}` : ''}</Button>
      <Button size="small" disabled={busy} onClick={onActivity}>操作历史{project.events.length ? ` · ${project.events.length}` : ''}</Button>
      <span className="micro">{project.history?.undoCount ?? 0} 次可撤销编辑</span>
    </div>}
    <div className="toolbar"><div className="filter-tabs" aria-label="照片筛选">{filters.map(filter => <button key={filter} aria-pressed={ui.filter === filter && ui.view === 'grid'}
      className={ui.filter === filter && ui.view === 'grid' ? 'active' : ''} onClick={() => ui.setFilter(filter)}>{String(zh['filters.' + filter] ?? filter)}<span className="count">{filter === 'all' ? project.photos.length : project.photos.filter(photo => photo.status === filter).length}</span></button>)}</div>
      <div className="toolbar-actions"><Button size="small" onClick={ui.toggleDensity} aria-label="切换网格密度" aria-pressed={ui.dense}><Columns3 size={15} /></Button>
        <Button size="small" disabled={project.photos.length < 2 || !active} onClick={onCompare}>{zh.compare}</Button><Button size="small" onClick={onExport}>{zh.export} ↗</Button>
        <Button size="small" onClick={ui.toggleFilmstrip} aria-label={ui.filmstripOpen ? '收起序列带' : '展开序列带'} aria-pressed={ui.filmstripOpen}><Film size={15} /></Button>
        <Button size="small" onClick={ui.toggleInspector} aria-label={ui.inspectorOpen ? zh.hidePanel : zh.showPanel}>{ui.inspectorOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}</Button></div>
      <div className="workspace-tools"><label className="search-field" htmlFor="photo-search"><Search size={14} /><input id="photo-search" type="search" aria-label="搜索照片文件名或备注" placeholder="搜索文件名或备注 · ⌘F" value={ui.search} onChange={event => ui.setSearch(event.target.value)} /></label>
        <label className="group-field">分组
          <select aria-label="网格分组" value={ui.grouping} onChange={event => ui.setGrouping(event.target.value as GridGrouping)}>
            <option value="none">不分组</option>
            <option value="status">按选片状态</option>
            <option value="rating">按人工评分</option>
            <option value="prefix">按文件名前缀</option>
          </select>
        </label>
        <span className="micro">⌘ / Ctrl 点选 · Shift 连选</span></div>
      {selectedIds.length > 1 && <div className="selection-tools" role="group" aria-label="批量选片"><span>已选 {selectedIds.length} 张</span>{(['keep', 'reject', 'undecided'] as const).map(status => <Button key={status} size="small" disabled={busy} onClick={() => onBatchDecide(status)}>{status === 'keep' ? '保留选区' : status === 'reject' ? '淘汰选区' : '选区待定'}</Button>)}
        <select aria-label="批量人工评分" disabled={busy} value="" onChange={event => onBatchRate(Number(event.target.value))}><option value="" disabled>人工评分…</option>{[0, 1, 2, 3, 4, 5].map(rating => <option key={rating} value={rating}>{rating ? `${rating} 星` : '清除评分'}</option>)}</select>
        {onAICompare && selectedIds.length === 2 && <Button size="small" disabled={busy} onClick={onAICompare} title="把两张选区照片交给AI 智能选片，比较叙事价值">AI 比较叙事价值</Button>}
        {onAIOrder && <Button size="small" disabled={busy} onClick={onAIOrder} title="把选区交给AI 智能选片，生成两版顺序方案">生成两版顺序</Button>}
        <Button size="small" variant="quiet" onClick={ui.clearSelection}>清除选区</Button></div>}
    </div>
    <div className="canvas-area"><div className="workspace-scroll"><div className="canvas-label"><span>{ui.view === 'grid' ? 'CONTACT SHEET' : ui.view === 'canvas' ? 'NARRATIVE CANVAS' : ui.view === 'publication' ? 'PUBLICATIONS' : 'SEQUENCE BOARD'}</span>
      <span>{ui.view === 'grid' ? `${visible.length} 张` : `${kept.length} 张`}</span>
      {ui.view === 'sequence' && kept.length > 0 && <Button size="small" disabled={busy} onClick={onCopyPlan} title="把当前作品序列保存为独立的方案副本，可在作品版本中对比与恢复"><Copy size={13} /> 复制序列方案</Button>}</div>
      {!local && <div className="preview-banner">{zh.preview}</div>}
      {ui.view === 'publication' && <PublicationPanel key={project.id} repository={publication} projectId={project.id} sequenceCount={kept.length} readOnly={readOnly} />}
      {ui.view === 'canvas' && <NarrativeCanvas key={project.id} repository={canvas} projectId={project.id} selectedPhotoIds={selectedIds} readOnly={readOnly} />}
            {ui.view === 'grid' ? (visible.length ? <div className="grid-groups">
        {groupPhotos(visible, ui.grouping).map(group => group.label ? <section key={group.key} className="grid-group" aria-label={`分组 ${group.label}`}>
          <h3 className="grid-group-head">{group.label}</h3>
          <ContactSheetGrid photos={group.photos} activeId={active?.id ?? null} selectedIds={selectedIds} dense={ui.dense} busy={busy} onSelect={onSelect} onDecide={onDecide} onEnlarge={onEnlarge} />
        </section> : <ContactSheetGrid key={group.key} photos={group.photos} activeId={active?.id ?? null} selectedIds={selectedIds} dense={ui.dense} busy={busy} onSelect={onSelect} onDecide={onDecide} onEnlarge={onEnlarge} />)}
      </div>
        : <EmptyState title={project.photos.length ? zh.emptyFilter : zh.emptyTitle} hint={project.photos.length ? '更改筛选条件。' : local ? '导入照片或文件夹。' : zh.emptyHint}
          action={<Button onClick={project.photos.length ? () => { ui.setFilter('all'); ui.setSearch(''); } : onImport}>{project.photos.length ? zh.allPhotos : local ? zh.import : `${zh.import} · 桌面版`}</Button>} />)
        : ui.view === 'sequence' ? (kept.length ? <SequenceBoard photos={kept} activeId={active?.id ?? null} busy={busy} onSelect={ui.select} onReorder={onReorder} />
          : <EmptyState title={zh.emptySequence} hint={zh.sequenceHint} action={<Button onClick={() => ui.setFilter('all')}>{zh.toWorkspace}</Button>} />) : null}
    </div>{ui.filmstripOpen && <SelectionDock photos={kept} onSelect={id => { ui.setFilter('all'); ui.setSearch(''); ui.select(id); }} onSequence={() => ui.setView('sequence')} />}</div>
  </>;
}
