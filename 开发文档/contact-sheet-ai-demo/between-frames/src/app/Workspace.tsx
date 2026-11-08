import { Columns3, PanelRightClose, PanelRightOpen, Film, Search, Copy } from 'lucide-react';
import type { SelectionGesture } from '../domain/selection';
import type { DesktopProjectRepository, Photo, PhotoFilter, PhotoStatus, Project } from '../domain/models';
import { groupPhotos, type GridGrouping } from '../domain/grouping';
import { useUIStore } from '../stores/uiStore';
import { NarrativeCanvas } from '../features/canvas/NarrativeCanvas';
import { PublicationPanel } from '../features/publishing/PublicationPanel';
import { zh } from './copy';
import { ProjectHeader } from '../features/project/ProjectHeader';
import { ProjectPreviewHealth } from '../features/project/ProjectPreviewHealth';
import { ProjectObservationVersions } from '../features/project/ProjectObservationVersions';
import { SidecarJudgementList } from '../features/photo-import/SidecarJudgementList';
import { ContactSheetGrid } from '../components/contact-sheet/ContactSheetGrid';
import { SequenceBoard } from '../components/sequence/SequenceBoard';
import { SelectionDock } from '../components/sequence/SelectionDock';
import { EmptyState } from '../components/shared/EmptyState';
import { Button } from '../components/shared/Button';
import { SharedReviewLink } from '../components/shared/SharedReviewLink';
import type { CanvasRepository } from '../domain/canvas';
import type { PublicationRepository } from '../domain/publication';
import { useAccount } from '../features/cloud-login/AccountProvider';
import { useI18n } from '../i18n';

export function Workspace({ project, visible, kept, active, selectedIds, busy, onEdit, onImport, onCompare, onExport, onDecide, onBatchDecide, onBatchRate, onSelect, onReorder, onEnlarge, onUndo, onRedo, onVersions, onActivity, onCopyPlan, onAICompare, onAIOrder, onEditorVersions, onChanged, local = false, sharing, canvas, publication, projectRepository }: {
  project: Project; visible: Photo[]; kept: Photo[]; active?: Photo; busy: boolean;
  selectedIds: string[]; onSelect: (id: string, gesture?: SelectionGesture) => void;
  onBatchDecide: (status: PhotoStatus) => void; onBatchRate: (rating: number) => void;
  onEdit: () => void; onImport: () => void; onCompare: () => void; onExport: () => void;
  onDecide: (id: string, status: PhotoStatus) => void; onReorder: (ids: string[]) => void; onEnlarge: (photo: Photo) => void;
  onUndo: () => void; onRedo: () => void; onVersions: () => void; onActivity: () => void; onCopyPlan: () => void;
  onAICompare?: () => void; onAIOrder?: () => void;
  onEditorVersions?: () => void;
  /** 图库侧这条回填判断后要用最新台账刷新：直接命令不走实时事件。 */
  onChanged?: () => void;
  local?: boolean;
  sharing?: import('../domain/sharing').SharingStatus;
  canvas?: CanvasRepository;
  publication?: PublicationRepository;
  projectRepository?: DesktopProjectRepository;
}) {
  const ui = useUIStore();
  const { t } = useI18n();
  const account = useAccount();
  const openAI = () => { if (account) void account.requireSignIn('ai', ui.openAgent); else ui.openAgent(); };
  const filters: PhotoFilter[] = ['all', 'keep', 'undecided', 'reject'];
  const photoTools = ui.view === 'grid' || ui.view === 'sequence';
  const gridTools = ui.view === 'grid';
  const readOnly = !!sharing && sharing.authority !== 'local';
  return <><ProjectHeader project={project} onEdit={onEdit} onImport={onImport} onAI={openAI} readOnly={readOnly} />
    {readOnly && <div className="preview-banner">{sharing?.authority === 'shared' ? zh.previewSharedProject : zh.previewSharePending}{sharing?.remote?.webUrl && <SharedReviewLink projectId={project.id} url={sharing.remote.webUrl} />}</div>}
    {(ui.view === 'grid' || ui.view === 'sequence') && <div className="history-toolbar" role="group" aria-label={zh.editHistory}>
      <Button size="small" disabled={busy || !project.history?.undoGroupId} onClick={onUndo} title="⌘ / Ctrl Z">{zh['common.undo']}</Button>
      <Button size="small" disabled={busy || !project.history?.redoGroupId} onClick={onRedo} title="⌘ / Ctrl Shift Z">{zh['common.redo']}</Button>
      <Button size="small" disabled={busy} onClick={onVersions}>{zh.workVersions}{project.versions?.length ? ` · ${project.versions.length}` : ''}</Button>
      {onEditorVersions && <Button size="small" onClick={onEditorVersions}>{t('editor.title', '调色版本')}</Button>}
      <Button size="small" disabled={busy} onClick={onActivity}>{zh.activityLog}{project.events.length ? ` · ${project.events.length}` : ''}</Button>
      <span className="micro">{zh.undoableEdits.replace('{count}', String(project.history?.undoCount ?? 0))}</span>
    </div>}
    {photoTools && local && !readOnly && <>
      <ProjectPreviewHealth repository={projectRepository} projectId={project.id} />
      <ProjectObservationVersions repository={projectRepository} projectId={project.id} />
      {/* 外部软件的判断不必等下一次导入才看得见：这里随时读一遍，落不落库还是由人点头。 */}
      {projectRepository?.sidecarProposals && <div className="local-file-panels"><SidecarJudgementList repository={projectRepository} projectId={project.id} onApplied={onChanged} /></div>}
    </>}
    <div className="toolbar">{gridTools && <div className="filter-tabs" aria-label={zh.photoFilters}>{filters.map(filter => <button key={filter} aria-pressed={ui.filter === filter}
      className={ui.filter === filter ? 'active' : ''} onClick={() => ui.setFilter(filter)}>{String(zh['filters.' + filter] ?? filter)}<span className="count">{filter === 'all' ? project.photos.length : project.photos.filter(photo => photo.status === filter).length}</span></button>)}</div>}
      <div className="toolbar-actions">{gridTools && <Button size="small" onClick={ui.toggleDensity} aria-label={zh.toggleGridDensity} aria-pressed={ui.dense}><Columns3 size={15} /></Button>}
        {photoTools && <Button size="small" disabled={project.photos.length < 2 || !active} onClick={onCompare}>{zh.compare}</Button>}<Button size="small" onClick={onExport}>{zh.export} ↗</Button>
        {photoTools && <Button size="small" onClick={ui.toggleFilmstrip} aria-label={ui.filmstripOpen ? zh.collapseFilmstrip : zh.expandFilmstrip} aria-pressed={ui.filmstripOpen}><Film size={15} /></Button>}
        <Button size="small" onClick={ui.toggleInspector} aria-label={ui.inspectorOpen ? zh.hidePanel : zh.showPanel}>{ui.inspectorOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}</Button></div>
      {gridTools && <div className="workspace-tools"><label className="search-field" htmlFor="photo-search"><Search size={14} /><input id="photo-search" type="search" aria-label={zh.searchPhotoAria} placeholder={zh.searchPhotoPlaceholder} value={ui.search} onChange={event => ui.setSearch(event.target.value)} /></label>
        <label className="group-field">{zh.groupLabel}
          <select aria-label={zh.gridGrouping} value={ui.grouping} onChange={event => ui.setGrouping(event.target.value as GridGrouping)}>
            <option value="none">{zh.groupNone}</option>
            <option value="status">{zh.groupByStatus}</option>
            <option value="rating">{zh.groupByRating}</option>
            <option value="prefix">{zh.groupByPrefix}</option>
          </select>
        </label>
        <span className="micro">{zh.selectionGestureHint}</span></div>}
      {photoTools && selectedIds.length > 1 && <div className="selection-tools" role="group" aria-label={zh.batchCuration}><span>{zh.selectedCount.replace('{count}', String(selectedIds.length))}</span>{(['keep', 'reject', 'undecided'] as const).map(status => <Button key={status} size="small" disabled={busy} onClick={() => onBatchDecide(status)}>{status === 'keep' ? zh.keepSelection : status === 'reject' ? zh.rejectSelection : zh.undecidedSelection}</Button>)}
        <select aria-label={zh.batchRatingAria} disabled={busy} value="" onChange={event => onBatchRate(Number(event.target.value))}><option value="" disabled>{zh.ratingMenuPlaceholder}</option>{[0, 1, 2, 3, 4, 5].map(rating => <option key={rating} value={rating}>{rating ? zh.starOption.replace('{count}', String(rating)) : zh.clearRating}</option>)}</select>
        {onAICompare && selectedIds.length === 2 && <Button size="small" disabled={busy} onClick={onAICompare} title={zh.aiCompareTitle}>{zh.aiCompareLabel}</Button>}
        {onAIOrder && <Button size="small" disabled={busy} onClick={onAIOrder} title={zh.aiOrderTitle}>{zh.aiOrderLabel}</Button>}
        <Button size="small" variant="quiet" onClick={ui.clearSelection}>{zh.clearSelection}</Button></div>}
    </div>
    <div className="canvas-area"><div className="workspace-scroll"><div className="canvas-label"><span>{ui.view === 'grid' ? 'CONTACT SHEET' : ui.view === 'canvas' ? 'NARRATIVE CANVAS' : ui.view === 'publication' ? 'PUBLICATIONS' : 'SEQUENCE BOARD'}</span>
      <span>{zh.photoCountShort.replace('{count}', String(ui.view === 'grid' ? visible.length : kept.length))}</span>
      {ui.view === 'sequence' && kept.length > 0 && <Button size="small" disabled={busy} onClick={onCopyPlan} title={zh.copyPlanTitle}><Copy size={13} /> {zh.copyPlanLabel}</Button>}</div>
      {!local && <div className="preview-banner">{zh.preview}</div>}
      {ui.view === 'publication' && <PublicationPanel key={project.id} repository={publication} projectId={project.id} sequenceCount={kept.length} readOnly={readOnly} />}
      {ui.view === 'canvas' && <NarrativeCanvas key={project.id} repository={canvas} projectId={project.id} selectedPhotoIds={selectedIds} readOnly={readOnly} />}
            {ui.view === 'grid' ? (visible.length ? <div className="grid-groups">
        {groupPhotos(visible, ui.grouping).map(group => group.label ? <section key={group.key} className="grid-group" aria-label={zh.groupAriaLabel.replace('{label}', group.label)}>
          <h3 className="grid-group-head">{group.label}</h3>
          <ContactSheetGrid photos={group.photos} activeId={active?.id ?? null} selectedIds={selectedIds} dense={ui.dense} busy={busy} onSelect={onSelect} onDecide={onDecide} onEnlarge={onEnlarge} />
        </section> : <ContactSheetGrid key={group.key} photos={group.photos} activeId={active?.id ?? null} selectedIds={selectedIds} dense={ui.dense} busy={busy} onSelect={onSelect} onDecide={onDecide} onEnlarge={onEnlarge} />)}
      </div>
        : <EmptyState title={project.photos.length ? zh.emptyFilter : zh.emptyTitle} hint={project.photos.length ? zh.emptyFilterHint : local ? zh.emptyImportHint : zh.emptyHint}
          action={<Button onClick={project.photos.length ? () => { ui.setFilter('all'); ui.setSearch(''); } : onImport}>{project.photos.length ? zh.allPhotos : local ? zh.import : `${zh.import} · 桌面版`}</Button>} />)
        : ui.view === 'sequence' ? (kept.length ? <SequenceBoard photos={kept} activeId={active?.id ?? null} busy={busy} onSelect={ui.select} onOpen={onEnlarge} onReorder={onReorder} />
          : <EmptyState title={zh.emptySequence} hint={zh.sequenceHint} action={<Button onClick={() => { ui.setFilter('all'); ui.setView('grid'); }}>{zh.toWorkspace}</Button>} />) : null}
    </div>{ui.filmstripOpen && <SelectionDock photos={kept} onSelect={id => { ui.setFilter('all'); ui.setSearch(''); ui.select(id); }} onSequence={() => ui.setView('sequence')} />}</div>
  </>;
}
