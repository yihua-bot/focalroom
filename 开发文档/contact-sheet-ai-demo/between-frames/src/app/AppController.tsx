import { useCallback, useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { arrayMove } from '@dnd-kit/sortable';
import type { Photo, PhotoStatus, Project, ProjectRepository } from '../domain/models';
import { useUIStore } from '../stores/uiStore';
import { useRoute, navigate } from './router';
import { useShortcuts } from './useShortcuts';
import { AppShell } from './AppShell';
import { SideNav } from './SideNav';
import { Workspace } from './Workspace';
import { zh } from './copy';
import { ProjectList } from '../features/project/ProjectList';
import { ProjectDialog } from '../features/project/ProjectDialog';
import { PhotoInspector } from '../components/inspector/PhotoInspector';
import { CompareModal } from '../components/compare/CompareModal';
import { AgentStatus } from '../features/ai-editor/AgentStatus';
import { AIPanel } from '../features/ai-editor/AIPanel';
import { ProposalsPanel } from '../features/proposals/ProposalsPanel';
import { PreferencePanel } from '../features/preference-memory/PreferencePanel';
import { ExportDialog } from '../features/export/ExportDialog';
import { EmptyState } from '../components/shared/EmptyState';
import { Modal } from '../components/shared/Modal';
import { Button } from '../components/shared/Button';
import { isDesktopRepository } from '../services/TauriRepository';
import { ImportDialog } from '../features/photo-import/ImportDialog';
import { VersionsDialog } from '../features/project/VersionsDialog';
import { Lightbox } from '../components/inspector/Lightbox';
import type { SelectionGesture } from '../domain/selection';

type OpenDialog = { kind: 'edit' | 'create' | 'export' | 'help' | 'import' | 'versions' } | { kind: 'compare'; left: Photo; right: Photo } | { kind: 'enlarge'; photo: Photo } | { kind: 'archive'; project: Project };

export function AppController({ repository }: { repository: ProjectRepository }) {
  const local = isDesktopRepository(repository);
  const route = useRoute(local);
  const ui = useUIStore();
  const client = useQueryClient();
  const [archived, setArchived] = useState(false);
  const query = useQuery({ queryKey: ['projects', archived], queryFn: () => repository.list(archived), refetchOnWindowFocus: local ? 'always' : false });
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [dialog, setDialog] = useState<OpenDialog | null>(null);
  const [notice, setNotice] = useState('');
  useEffect(() => {
    if (!local) return;
    let stopped = false;
    let unsubscribe: (() => void) | undefined;
    void repository.subscribe((id, revision) => {
      const current = client.getQueriesData<Project[]>({ queryKey: ['projects'] }).flatMap(([, data]) => data ?? []).find(item => item.id === id);
      if (revision === undefined || !current || revision > (current.revision ?? 0)) void client.invalidateQueries({ queryKey: ['projects'] });
    }).then(cleanup => { if (stopped) cleanup(); else unsubscribe = cleanup; }).catch(() => {
      if (!stopped) setNotice('实时更新暂不可用；切换窗口后会重新读取项目。');
    });
    return () => { stopped = true; unsubscribe?.(); };
  }, [repository, local, client]);
  const projects = query.data ?? [];
  const project = projects.find(item => item.id === route.projectId);
  const [lastProjectId, setLastProjectId] = useState('sample-project');
  const navProject = project ?? projects.find(item => item.id === lastProjectId);
  const mutation = useMutation({ mutationFn: (task: () => Promise<Project>) => task(), onSuccess: updated => {
    if (updated.archived) {
      client.setQueryData<Project[]>(['projects', false], previous => previous?.filter(item => item.id !== updated.id));
      void client.invalidateQueries({ queryKey: ['projects'] });
      navigate(null);
      return;
    }
    client.setQueryData<Project[]>(['projects', false], previous => previous?.some(item => item.id === updated.id)
      ? previous.map(item => item.id === updated.id ? updated : item) : [...(previous ?? []), updated]);
  }, onError: error => { setNotice(`操作失败：${error.message}`); void client.invalidateQueries({ queryKey: ['projects'] }); } });
  const refresh = useCallback(() => { void client.invalidateQueries({ queryKey: ['projects'] }); }, [client]);
  useEffect(() => { if (!notice) return; const timer = window.setTimeout(() => setNotice(''), 4000); return () => window.clearTimeout(timer); }, [notice]);
  const openProject = (id: string) => { ui.reset(); ui.select(projects.find(item => item.id === id)?.photos[0]?.id ?? null); setLastProjectId(id); navigate(id); };
  const terms = ui.search.normalize('NFKC').toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
  const visible = project?.photos.filter(photo => (ui.filter === 'all' || photo.status === ui.filter) && terms.every(term => `${photo.name} ${photo.note} ${photo.index}`.normalize('NFKC').toLocaleLowerCase().includes(term))) ?? [];
  const kept = project?.sequence.map(id => project.photos.find(photo => photo.id === id)).filter((photo): photo is Photo => Boolean(photo)) ?? [];
  const navigable = ui.view === 'sequence' ? kept : visible;
  const active = navigable.find(photo => photo.id === ui.activeId) ?? navigable[0];
  const selectedIds = navigable.filter(photo => ui.selectedIds.includes(photo.id)).map(photo => photo.id);
  const targets = selectedIds.length ? selectedIds : active ? [active.id] : [];
  const select = (id: string, gesture?: SelectionGesture) => ui.selectRange(id, navigable.map(photo => photo.id), gesture);
  const act = (task: () => Promise<Project>, success?: string, close = false, onDone?: () => void) => {
    if (mutation.isPending) return;
    mutation.mutate(task, { onSuccess: () => { if (success) setNotice(success); if (close) setDialog(null); onDone?.(); } });
  };
  const decide = (id: string, status: PhotoStatus) => { if (project) act(() => repository.decide(project.id, id, status)); };
  const decideSelection = (status: PhotoStatus) => { if (project && targets.length) act(() => repository.decideMany(project.id, targets, status), `${targets.length} 张已${status === 'keep' ? '保留' : status === 'reject' ? '淘汰' : '设为待定'}`); };
  const rateSelection = (rating: number) => { if (project && targets.length) act(() => repository.rate(project.id, targets, rating), rating ? `人工评分已保存：${rating} 星` : '人工评分已清除'); };
  const undo = () => { if (project?.history?.undoGroupId) act(() => repository.undo(project.id), '已撤销上一步'); };
  const redo = () => { if (project?.history?.redoGroupId) act(() => repository.redo(project.id), '已重做上一步'); };
  const reorder = (ids: string[]) => { if (project) act(() => repository.reorder(project.id, ids), local ? '作品顺序已保存' : zh.savedSequence); };
  const compare = () => {
    if (!project || !active) return;
    if (selectedIds.length > 2) { setNotice('请保留两张选中照片后再比较。'); return; }
    if (selectedIds.length === 2) {
      const [left, right] = selectedIds.map(id => project.photos.find(photo => photo.id === id)!);
      setDialog({ kind: 'compare', left, right }); return;
    }
    const candidates = navigable;
    const index = candidates.findIndex(photo => photo.id === active.id);
    const right = candidates[(index + 1) % candidates.length];
    if (right && right.id !== active.id) { setNotice('比较当前照片与下一张；也可选中两张后比较。'); setDialog({ kind: 'compare', left: active, right }); }
    else setNotice('当前视图不足两张照片，请调整筛选或搜索。');
  };
  useShortcuts({ photos: navigable, activeId: active?.id ?? null, view: ui.view, blocked: Boolean(dialog) || mutation.isPending || route.page !== 'workspace',
    onSelect: select, onSelectAll: ui.selectAll, onClear: ui.clearSelection, onDecide: (_id, status) => decideSelection(status), onRate: rateSelection,
    onCompare: compare, onEnlarge: () => { if (active) setDialog({ kind: 'enlarge', photo: active }); }, onView: ui.setView,
    onUndo: undo, onRedo: redo, onSearch: () => document.getElementById('photo-search')?.focus(), onAgent: ui.toggleAgent, onMove: offset => {
      const from = kept.findIndex(photo => photo.id === active?.id);
      const to = from + offset;
      if (from >= 0 && to >= 0 && to < kept.length) reorder(arrayMove(kept.map(photo => photo.id), from, to));
    } });
  const panel = ui.inspectorOpen && route.page === 'workspace' && project ? <aside className="inspector">
    <span className="eyebrow">THE EDITOR / WORKING NOTES</span><h2 className="editor-title">{zh.editorTitle}</h2><p className="editor-hint">{zh.editorHint}</p>
    <div className="right-tabs" aria-label="检查器面板">{(['photo', 'ai', 'memory'] as const).map(key => <button key={key} className={ui.panel === key ? 'active' : ''} aria-pressed={ui.panel === key} onClick={() => ui.setPanel(key)}>{key === 'photo' ? '照片检查' : key === 'ai' ? 'AI 建议' : zh.memory}</button>)}</div>
    {ui.panel === 'photo' ? (active ? <PhotoInspector key={`${active.id}:${active.note}`} photo={active} busy={mutation.isPending} onDecide={decide}
      onRate={(id, rating) => act(() => repository.rate(project.id, [id], rating))}
      onBookMark={local && project ? (id, mark) => act(() => repository.setBookMark(project.id, id, mark), mark === 'spread' ? '已设为跨页大图' : mark === 'full' ? '已设为整页图' : '已清除册页标记') : undefined}
      draft={drafts[active.id]} onDraft={note => setDrafts(previous => ({ ...previous, [active.id]: note }))}
      onNote={(id, note) => act(() => repository.note(project.id, id, note), local ? '判断已保存到本地' : zh.savedNote, false, () => setDrafts(previous => { const next = { ...previous }; delete next[id]; return next; }))}
      onRelink={local ? () => { act(async () => (await repository.relink(project.id, active.id)) ?? project); } : undefined}
      onEnlarge={photo => setDialog({ kind: 'enlarge', photo })} />
      : <EmptyState title="选择一张照片" hint="点击网格，开始检查画面。" />)
      : ui.panel === 'ai' ? <><AIPanel repository={local ? repository : undefined} project={project} selectedIds={selectedIds} onUpdated={updated => { client.setQueryData<Project[]>(['projects', false], previous => previous?.map(item => item.id === updated.id ? updated : item)); refresh(); }} /><ProposalsPanel project={project} enabled={local} /></> : <PreferencePanel events={project.events} repository={local ? repository : undefined} project={project} />}
  </aside> : undefined;
  return <><AppShell inspector={panel} collapsed={!ui.sidebarOpen} nav={<SideNav project={navProject} page={route.page} view={ui.view} local={local}
    onView={view => { if (!navProject) return; if (route.page === 'projects') navigate(navProject.id); ui.setView(view); }}
    onProjects={() => { setArchived(false); navigate(null); }} onMemory={() => { if (navProject) navigate(navProject.id); ui.setPanel('memory'); }} onHelp={() => setDialog({ kind: 'help' })} />}>
    {query.isPending ? <p role="status" className="page-message">正在打开示例项目…</p> : query.isError ? <EmptyState title="项目加载失败" hint={query.error.message} action={<Button onClick={() => { void query.refetch(); }}>重试</Button>} />
      : route.page === 'projects' ? <ProjectList projects={projects} onOpen={openProject} onCreate={() => { setArchived(false); setDialog({ kind: 'create' }); }} local={local} archived={archived}
        onToggleArchive={() => setArchived(value => !value)} onArchive={item => setDialog({ kind: 'archive', project: item })}
        onRestore={item => { if (local) act(() => repository.restore(item.id), '项目已恢复', false, () => { setArchived(false); refresh(); }); }} />
      : project ? <Workspace project={project} visible={visible} kept={kept} active={active} selectedIds={selectedIds} onSelect={select} onBatchDecide={decideSelection} onBatchRate={rateSelection} busy={mutation.isPending} local={local} onEdit={() => setDialog({ kind: 'edit' })} onImport={() => setDialog({ kind: 'import' })}
        onUndo={undo} onRedo={redo} onVersions={() => setDialog({ kind: 'versions' })} onCompare={compare} onExport={() => setDialog({ kind: 'export' })} onDecide={decide} onReorder={reorder} onEnlarge={photo => setDialog({ kind: 'enlarge', photo })} />
        : <EmptyState title={local ? '项目未找到' : '项目不在本次预览中'} hint={local ? '项目可能已归档，请返回项目列表查看。' : '内存项目在刷新后重置，请返回项目列表。'} action={<Button onClick={() => navigate(null)}>{zh.projects}</Button>} />}
  </AppShell>
    {local && project && !(ui.inspectorOpen && ui.panel === 'ai') && <AgentStatus repository={repository} projectId={project.id} onOpen={() => ui.setPanel('ai')} />}
    {notice && <div role="status" className="toast">{notice}</div>}
    {(dialog?.kind === 'create' || dialog?.kind === 'edit') && <ProjectDialog project={dialog.kind === 'edit' ? project : undefined} busy={mutation.isPending} onClose={() => setDialog(null)} onSave={(title, brief, tags) => {
      if (dialog.kind === 'edit' && project) act(async () => { const updated = await repository.edit(project.id, title, brief); if (tags) await repository.setTags(project.id, tags); return updated; }, local ? '项目已保存到本地' : zh.savedProject, true);
      else mutation.mutate(() => repository.create(title, brief), { onSuccess: created => { setDialog(null); ui.reset(); ui.select(null); setLastProjectId(created.id); navigate(created.id); } });
    }} />}
    {dialog?.kind === 'compare' && project && <CompareModal left={dialog.left} right={dialog.right} busy={mutation.isPending} onClose={() => setDialog(null)} onCompare={(preferred, reason) => act(() => repository.compare(project.id, dialog.left.id, dialog.right.id, preferred, reason), zh.savedCompare, true)} />}
    {dialog?.kind === 'versions' && project && <VersionsDialog project={project} busy={mutation.isPending} onClose={() => setDialog(null)}
      onSave={label => act(() => repository.saveVersion(project.id, label), '当前作品版本已保存')}
      onRestore={versionId => act(() => repository.restoreVersion(project.id, versionId), '版本已恢复，可撤销此操作', true)} />}
    {dialog?.kind === 'export' && <ExportDialog repository={local ? repository : undefined} projectId={project?.id} onClose={() => setDialog(null)} />}
    {dialog?.kind === 'import' && (local && project ? <ImportDialog repository={repository} projectId={project.id} onClose={() => { setDialog(null); refresh(); }} onChanged={refresh} />
      : <Modal title={zh.import} description="浏览器只提供示例预览，请在 Tauri 桌面版导入真实照片。" onClose={() => setDialog(null)}><div className="dialog-actions"><Button onClick={() => setDialog(null)}>继续体验示例项目</Button></div></Modal>)}
    {dialog?.kind === 'archive' && local && <Modal title="确认归档项目" description={`“${dialog.project.title}”将移至归档列表，可随时恢复。原图、备注和决策日志全部保留。`} onClose={() => setDialog(null)}>
      <div className="dialog-actions"><Button onClick={() => setDialog(null)}>取消</Button><Button variant="primary" onClick={() => {
        void repository.archive(dialog.project.id).then(() => { setDialog(null); refresh(); }).catch(error => setNotice(`归档失败：${String(error)}`));
      }}>确认归档</Button></div></Modal>}
    {dialog?.kind === 'enlarge' && <Lightbox photos={navigable} activeId={dialog.photo.id} onClose={() => setDialog(null)} onNavigate={photo => { ui.select(photo.id); setDialog({ kind: 'enlarge', photo }); }} onDecide={(id, status) => decide(id, status)} onRate={(id, rating) => project && act(() => repository.rate(project.id, [id], rating), rating ? `人工评分已保存：${rating} 星` : '人工评分已清除')} />}
    {dialog?.kind === 'help' && <Modal title={zh.help} description={zh.helpHint} onClose={() => setDialog(null)}><dl className="shortcut-list">
      <dt><kbd>K / X / U</kbd></dt><dd>将选区明确设为保留 / 淘汰 / 待定；重复按键不会撤销</dd><dt><kbd>0–5</kbd></dt><dd>人工星级；0 清除评分</dd>
      <dt><kbd>← →</kbd></dt><dd>切换照片；Shift + 方向键扩展选区</dd><dt><kbd>C</kbd></dt><dd>比较选中的两张；未选两张时比较当前与下一张</dd>
      <dt><kbd>G / S</kbd></dt><dd>网格 / 作品序列</dd><dt><kbd>⌘F / ⌘A</kbd></dt><dd>搜索 / 选择当前视图所有照片（Windows 使用 Ctrl）</dd>
      <dt><kbd>⌘Z / ⌘⇧Z</kbd></dt><dd>撤销 / 重做一次人工编辑；输入框使用文本自身撤销</dd>
      <dt><kbd>⌘⇧A</kbd></dt><dd>显示 / 隐藏 Agent 面板</dd><dt><kbd>↑ ↓</kbd></dt><dd>在作品序列中移动当前照片</dd>
      <dt><kbd>Space</kbd></dt><dd>打开 / 关闭大图；排序把手聚焦时启动 / 完成键盘拖动</dd>
      <dt><kbd>Esc</kbd></dt><dd>关闭弹窗 / 取消拖拽 / 清除选区</dd></dl></Modal>}
  </>;
}
