import { Columns3, PanelRightClose, PanelRightOpen, Film, Search } from 'lucide-react';
import type { SelectionGesture } from '../domain/selection';
import type { Photo, PhotoFilter, PhotoStatus, Project } from '../domain/models';
import { useUIStore } from '../stores/uiStore';
import { zh } from './copy';
import { ProjectHeader } from '../features/project/ProjectHeader';
import { ContactSheetGrid } from '../components/contact-sheet/ContactSheetGrid';
import { SequenceBoard } from '../components/sequence/SequenceBoard';
import { SelectionDock } from '../components/sequence/SelectionDock';
import { EmptyState } from '../components/shared/EmptyState';
import { Button } from '../components/shared/Button';
import { SharedReviewLink } from '../components/shared/SharedReviewLink';

export function Workspace({ project, visible, kept, active, selectedIds, busy, onEdit, onImport, onCompare, onExport, onDecide, onBatchDecide, onBatchRate, onSelect, onReorder, onEnlarge, onUndo, onRedo, onVersions, local = false, sharing }: {
  project: Project; visible: Photo[]; kept: Photo[]; active?: Photo; busy: boolean;
  selectedIds: string[]; onSelect: (id: string, gesture?: SelectionGesture) => void;
  onBatchDecide: (status: PhotoStatus) => void; onBatchRate: (rating: number) => void;
  onEdit: () => void; onImport: () => void; onCompare: () => void; onExport: () => void;
  onDecide: (id: string, status: PhotoStatus) => void; onReorder: (ids: string[]) => void; onEnlarge: (photo: Photo) => void;
  onUndo: () => void; onRedo: () => void; onVersions: () => void;
  local?: boolean;
  sharing?: import('../domain/sharing').SharingStatus;
}) {
  const ui = useUIStore();
  const filters: PhotoFilter[] = ['all', 'keep', 'undecided', 'reject'];
  const readOnly = !!sharing && sharing.authority !== 'local';
  return <><ProjectHeader project={project} onEdit={onEdit} onImport={onImport} onAI={() => ui.setPanel('ai')} readOnly={readOnly} />
    {readOnly && <div className="preview-banner">{sharing?.authority === 'shared' ? '共享项目 · 桌面缓存只读，请在 Web 继续评片。' : '共享迁移中 · 本地写入已暂停，请在导出窗口继续或显式取消迁移。'}{sharing?.remote?.webUrl && <SharedReviewLink projectId={project.id} url={sharing.remote.webUrl} />}</div>}
    <div className="history-toolbar" role="group" aria-label="编辑历史">
      <Button size="small" disabled={busy || !project.history?.undoGroupId} onClick={onUndo} title="⌘ / Ctrl Z">撤销</Button>
      <Button size="small" disabled={busy || !project.history?.redoGroupId} onClick={onRedo} title="⌘ / Ctrl Shift Z">重做</Button>
      <Button size="small" disabled={busy} onClick={onVersions}>作品版本{project.versions?.length ? ` · ${project.versions.length}` : ''}</Button>
      <span className="micro">{project.history?.undoCount ?? 0} 次可撤销编辑 · {readOnly ? '本地只读快照' : local ? '本地保存' : '内存预览'}</span>
    </div>
    <div className="toolbar"><div className="filter-tabs" aria-label="照片筛选">{filters.map(filter => <button key={filter} aria-pressed={ui.filter === filter && ui.view === 'grid'}
      className={ui.filter === filter && ui.view === 'grid' ? 'active' : ''} onClick={() => ui.setFilter(filter)}>{zh.filters[filter]}<span className="count">{filter === 'all' ? project.photos.length : project.photos.filter(photo => photo.status === filter).length}</span></button>)}</div>
      <div className="toolbar-actions"><Button size="small" onClick={ui.toggleDensity} aria-label="切换网格密度" aria-pressed={ui.dense}><Columns3 size={15} /></Button>
        <Button size="small" disabled={project.photos.length < 2 || !active} onClick={onCompare}>{zh.compare}</Button><Button size="small" onClick={onExport}>{zh.export} ↗</Button>
        <Button size="small" onClick={ui.toggleFilmstrip} aria-label={ui.filmstripOpen ? '收起序列带' : '展开序列带'} aria-pressed={ui.filmstripOpen}><Film size={15} /></Button>
        <Button size="small" onClick={ui.toggleInspector} aria-label={ui.inspectorOpen ? '收起检查器' : '展开检查器'}>{ui.inspectorOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}</Button></div>
      <div className="workspace-tools"><label className="search-field" htmlFor="photo-search"><Search size={14} /><input id="photo-search" type="search" aria-label="搜索照片文件名或备注" placeholder="搜索文件名或备注 · ⌘F" value={ui.search} onChange={event => ui.setSearch(event.target.value)} /></label>
        <span className="micro">⌘ / Ctrl 点选 · Shift 连选</span></div>
      {selectedIds.length > 1 && <div className="selection-tools" role="group" aria-label="批量选片"><span>已选 {selectedIds.length} 张</span>{(['keep', 'reject', 'undecided'] as const).map(status => <Button key={status} size="small" disabled={busy} onClick={() => onBatchDecide(status)}>{status === 'keep' ? '保留选区' : status === 'reject' ? '淘汰选区' : '选区待定'}</Button>)}
        <select aria-label="批量人工评分" disabled={busy} value="" onChange={event => onBatchRate(Number(event.target.value))}><option value="" disabled>人工评分…</option>{[0, 1, 2, 3, 4, 5].map(rating => <option key={rating} value={rating}>{rating ? `${rating} 星` : '清除评分'}</option>)}</select>
        <Button size="small" variant="quiet" onClick={ui.clearSelection}>清除选区</Button></div>}
    </div>
    <div className="canvas-area"><div className="workspace-scroll"><div className="canvas-label"><span>{ui.view === 'grid' ? 'CONTACT SHEET' : 'SEQUENCE BOARD'}</span>
      <span>{ui.view === 'grid' ? `${visible.length} FRAMES · 完整构图 · K / X 快捷操作` : `${kept.length} 张 · 拖动把手排序 · ↑ / ↓ 移动`}</span></div>
      <div className="preview-banner">{readOnly ? '当前显示迁移时的本地快照，云端后续改动请在共享评片中查看。' : local ? '本地项目 · 所有已保存的判断和顺序将在重启后保留' : zh.preview}</div>
      {ui.view === 'grid' ? (visible.length ? <ContactSheetGrid photos={visible} activeId={active?.id ?? null} selectedIds={selectedIds} dense={ui.dense} busy={busy} onSelect={onSelect} onDecide={onDecide} onEnlarge={onEnlarge} />
        : <EmptyState title={project.photos.length ? zh.emptyFilter : zh.emptyTitle} hint={project.photos.length ? '换一个筛选，继续观看。' : local ? '导入照片或文件夹，开始你的第一次选片。原图保留在原目录。' : zh.emptyHint}
          action={<Button onClick={project.photos.length ? () => { ui.setFilter('all'); ui.setSearch(''); } : onImport}>{project.photos.length ? zh.allPhotos : local ? zh.import : `${zh.import} · 桌面版`}</Button>} />)
        : (kept.length ? <SequenceBoard photos={kept} activeId={active?.id ?? null} busy={busy} onSelect={ui.select} onReorder={onReorder} />
          : <EmptyState title={zh.emptySequence} hint={zh.sequenceHint} action={<Button onClick={() => ui.setFilter('all')}>{zh.toWorkspace}</Button>} />)}
    </div>{ui.filmstripOpen && <SelectionDock photos={kept} onSelect={id => { ui.setFilter('all'); ui.setSearch(''); ui.select(id); }} onSequence={() => ui.setView('sequence')} />}</div>
  </>;
}
