import { useCallback, useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { arrayMove } from '@dnd-kit/sortable';
import type { Photo, PhotoStatus, Project, ProjectRepository } from '../domain/models';
import { useUIStore } from '../stores/uiStore';
import { useRoute, navigate } from './router';
import { useShortcuts } from './useShortcuts';
import { CommandPalette, type CommandEntry } from './CommandPalette';
import { ShortcutEditor, formatShortcut } from './ShortcutEditor';
import { useShortcutConfig } from './shortcutConfig';
import { AppShell } from './AppShell';
import { SideNav } from './SideNav';
import { Workspace } from './Workspace';
import { zh } from './copy';
import { useI18n } from '../i18n';
import { ProjectList } from '../features/project/ProjectList';
import { ProjectDialog } from '../features/project/ProjectDialog';
import { QuickStartModal, parseGoal } from '../features/quick-start/QuickStartModal';
import { PhotoInspector } from '../components/inspector/PhotoInspector';
import { CompareModal } from '../components/compare/CompareModal';
import { AgentStatus } from '../features/ai-editor/AgentStatus';
import { AISelectionDrawer } from '../features/ai-editor/AISelectionDrawer';
import { AIPanel, type QuickAgentAction } from '../features/ai-editor/AIPanel';
import { ProposalsPanel } from '../features/proposals/ProposalsPanel';
import { PreferencePanel } from '../features/preference-memory/PreferencePanel';
import { ExportDialog } from '../features/export/ExportDialog';
import { EmptyState } from '../components/shared/EmptyState';
import { Modal } from '../components/shared/Modal';
import { Button } from '../components/shared/Button';
import { isDesktopRepository } from '../services/TauriRepository';
import { ImportDialog } from '../features/photo-import/ImportDialog';
import { EditorVersionsDialog } from '../features/photo-import/EditorVersionsDialog';
import { VersionsDialog } from '../features/project/VersionsDialog';
import { ActivityDialog } from '../features/project/ActivityDialog';
import { Lightbox } from '../components/inspector/Lightbox';
import type { SelectionGesture } from '../domain/selection';
import { useAccount } from '../features/cloud-login/AccountProvider';
import { UpdateCheck } from '../components/shared/UpdateCheck';

type OpenDialog = { kind: 'edit' | 'create' | 'export' | 'help' | 'import' | 'editorVersions' | 'versions' | 'activity' | 'quickStart' | 'palette' } | { kind: 'compare'; left: Photo; right: Photo } | { kind: 'enlarge'; photo: Photo } | { kind: 'archive'; project: Project };

export function AppController({ repository }: { repository: ProjectRepository }) {
  useI18n();
  const local = isDesktopRepository(repository);
  const route = useRoute(local);
  const ui = useUIStore();
  const account = useAccount();
  const openAI = () => { if (ui.aiOpen) { ui.closeAgent(); return; } if (account) void account.requireSignIn('ai', ui.openAgent); else ui.openAgent(); };
  const shortcutConfig = useShortcutConfig();
  const client = useQueryClient();
  const [archived, setArchived] = useState(false);
  const query = useQuery({ queryKey: ['projects', archived], queryFn: () => repository.list(archived), refetchOnWindowFocus: local ? 'always' : false });
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [dialog, setDialog] = useState<OpenDialog | null>(null);
  const [pendingGoal, setPendingGoal] = useState<{ projectId: string; goal: string; count: number } | null>(null);
  const [quickAction, setQuickAction] = useState<(QuickAgentAction & { projectId: string }) | null>(null);
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
  const sharing = useQuery({ queryKey: ['sharing', project?.id], enabled: !!(local && repository.sharing && project), queryFn: () => local && repository.sharing && project ? repository.sharing.status(project.id) : Promise.resolve(null), refetchOnWindowFocus: true });
  const readOnly = !!sharing.data && sharing.data.authority !== 'local';
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
  const refresh = useCallback(() => { void client.invalidateQueries({ queryKey: ['projects'] }); void client.invalidateQueries({ queryKey: ['sharing'] }); }, [client]);
  useEffect(() => { if (!notice) return; const timer = window.setTimeout(() => setNotice(''), 4000); return () => window.clearTimeout(timer); }, [notice]);
  const openProject = (id: string) => { ui.reset(); ui.select(projects.find(item => item.id === id)?.photos[0]?.id ?? null); navigate(id); };
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
    if (readOnly) { setNotice('共享项目已暂停本机编辑，请在线继续审稿。'); return; }
    mutation.mutate(task, { onSuccess: () => { if (success) setNotice(success); if (close) setDialog(null); onDone?.(); } });
  };
  const decide = (id: string, status: PhotoStatus) => { if (project) act(() => repository.decide(project.id, id, status)); };
  const decideSelection = (status: PhotoStatus) => { if (project && targets.length) act(() => repository.decideMany(project.id, targets, status), `${targets.length} 张已${status === 'keep' ? '保留' : status === 'reject' ? '淘汰' : '设为待定'}`); };
  const rateSelection = (rating: number) => { if (project && targets.length) act(() => repository.rate(project.id, targets, rating), rating ? `人工评分已保存：${rating} 星` : '人工评分已清除'); };
  const undo = () => { if (project?.history?.undoGroupId) act(() => repository.undo(project.id), '已撤销上一步'); };
  const redo = () => { if (project?.history?.redoGroupId) act(() => repository.redo(project.id), '已重做上一步'); };
  const copyPlan = () => {
    if (!project) return;
    const stamp = new Date().toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' });
    act(() => repository.saveVersion(project.id, `方案副本 · ${stamp} · ${kept.length} 张`), '已复制当前序列方案；可在作品版本中对比或恢复');
  };
  const photoName = (id: string) => project?.photos.find(photo => photo.id === id)?.name ?? id;
  const aiCompare = local && project && selectedIds.length === 2 ? () => {
    const [a, b] = selectedIds;
    setQuickAction({ projectId: project.id, id: crypto.randomUUID(), count: 1, photoIds: [a, b],
      goal: `比较照片「${photoName(a)}」与「${photoName(b)}」：哪一张更能推动叙事主线？给出结论、理由与各自适合的位置；不直接修改选片状态。` });
    openAI();
  } : undefined;
  const aiOrder = local && project && selectedIds.length >= 2 ? () => {
    setQuickAction({ projectId: project.id, id: crypto.randomUUID(), count: selectedIds.length, photoIds: [...selectedIds],
      goal: `为选中的 ${selectedIds.length} 张照片生成两版不同的作品顺序方案（方案 A / 方案 B），各自解释编排逻辑、开合节奏与适用场景；输出为可审阅草案。` });
    openAI();
  } : undefined;
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
  useShortcuts({ shortcuts: shortcutConfig.shortcuts, photos: navigable, activeId: active?.id ?? null, view: ui.view, blocked: Boolean(dialog) || mutation.isPending || route.page !== 'workspace',
    onCommandPalette: () => setDialog(current => current?.kind === 'palette' ? null : { kind: 'palette' }),
    onSelect: select, onSelectAll: ui.selectAll, onClear: ui.clearSelection, onDecide: (_id, status) => decideSelection(status), onRate: rateSelection,
    onCompare: compare, onEnlarge: () => { if (active) setDialog({ kind: 'enlarge', photo: active }); }, onView: ui.setView,
    onUndo: undo, onRedo: redo, onSearch: () => document.getElementById('photo-search')?.focus(), onAgent: openAI, onMove: offset => {
      const from = kept.findIndex(photo => photo.id === active?.id);
      const to = from + offset;
      if (from >= 0 && to >= 0 && to < kept.length) reorder(arrayMove(kept.map(photo => photo.id), from, to));
    } });
  const keys = shortcutConfig.shortcuts;
  const paletteCommands: CommandEntry[] = [];
  if (route.page === 'workspace' && project) {
    const hint = (id: keyof typeof keys) => formatShortcut(keys[id]);
    if (!readOnly) {
      paletteCommands.push(
        { id: 'decide-keep', group: '决策', label: `保留选区（${targets.length} 张）`, hint: hint('keep'), run: () => decideSelection('keep') },
        { id: 'decide-reject', group: '决策', label: `淘汰选区（${targets.length} 张）`, hint: hint('reject'), run: () => decideSelection('reject') },
        { id: 'decide-undecided', group: '决策', label: `选区设为待定（${targets.length} 张）`, hint: hint('undecided'), run: () => decideSelection('undecided') },
        { id: 'rate-clear', group: '评分', label: '清除选区评分', hint: hint('rate0'), run: () => rateSelection(0) },
        ...[1, 2, 3, 4, 5].map(rating => ({ id: `rate-${rating}`, group: '评分', label: `选区评 ${rating} 星`, hint: hint(`rate${rating}` as keyof typeof keys), run: () => rateSelection(rating) })),
        { id: 'undo', group: '历史', label: '撤销上一步人工编辑', hint: hint('undo'), run: undo },
        { id: 'redo', group: '历史', label: '重做', hint: hint('redo'), run: redo },
        { id: 'import', group: '项目', label: '导入照片', run: () => setDialog({ kind: 'import' }) },
      );
    }
    paletteCommands.push(
      { id: 'view-grid', group: '视图', label: '切换到网格视图', hint: hint('viewGrid'), run: () => ui.setView('grid') },
      { id: 'view-sequence', group: '视图', label: '切换到作品序列', hint: hint('viewSequence'), run: () => ui.setView('sequence') },
      { id: 'compare', group: '检查', label: '比较两张照片', hint: hint('compare'), run: compare },
      { id: 'enlarge', group: '检查', label: '放大当前照片', hint: hint('enlarge'), run: () => { if (active) setDialog({ kind: 'enlarge', photo: active }); } },
      { id: 'select-all', group: '选择', label: '选择当前视图全部照片', hint: hint('selectAll'), run: () => ui.selectAll(navigable.map(photo => photo.id)) },
      { id: 'clear-selection', group: '选择', label: '清除选区', hint: hint('clearSelection'), run: ui.clearSelection },
      { id: 'search', group: '导航', label: '聚焦搜索框', hint: hint('search'), run: () => document.getElementById('photo-search')?.focus() },
      { id: 'agent', group: '导航', label: '打开/关闭 Focal AI', hint: hint('toggleAgent'), run: openAI },
      { id: 'versions', group: '项目', label: '查看/保存作品版本', run: () => setDialog({ kind: 'versions' }) },
      { id: 'activity', group: '项目', label: '按操作者浏览操作历史', run: () => setDialog({ kind: 'activity' }) },
      { id: 'copy-plan', group: '项目', label: '复制当前序列方案', run: copyPlan },
      { id: 'export', group: '项目', label: '导出作品', run: () => setDialog({ kind: 'export' }) },
    );
  }
  paletteCommands.push(
    { id: 'new-project', group: '项目', label: '新建项目', run: () => { setArchived(false); setPendingGoal(null); setDialog({ kind: 'create' }); } },
    { id: 'help', group: '帮助', label: '快捷键与帮助', run: () => setDialog({ kind: 'help' }) },
  );
  if (local) paletteCommands.push({ id: 'quick-start', group: '项目', label: '快速开始：用一句话创建项目', run: () => { setArchived(false); setDialog({ kind: 'quickStart' }); } });
  const panel = ui.inspectorOpen && route.page === 'workspace' && project ? <aside className="inspector" aria-label={zh.editorTitle}>
    <h2 className="editor-title">{zh.editorTitle}</h2>
    {active ? <PhotoInspector key={`${active.id}:${active.note}`} photo={active} busy={mutation.isPending || readOnly} onDecide={decide}
      onRate={(id, rating) => act(() => repository.rate(project.id, [id], rating))}
      onBookMark={local && project ? (id, mark) => act(() => repository.setBookMark(project.id, id, mark), mark === 'spread' ? '已设为跨页大图' : mark === 'full' ? '已设为整页图' : '已清除册页标记') : undefined}
      draft={drafts[active.id]} onDraft={note => setDrafts(previous => ({ ...previous, [active.id]: note }))}
      onNote={(id, note) => act(() => repository.note(project.id, id, note), zh.savedNote, false, () => setDrafts(previous => { const next = { ...previous }; delete next[id]; return next; }))}
      onRelink={local ? () => { act(async () => (await repository.relink(project.id, active.id)) ?? project); } : undefined}
      onEnlarge={photo => setDialog({ kind: 'enlarge', photo })} />
      : <EmptyState title="选择一张照片" hint="点击网格，开始检查画面。" />}
  </aside> : undefined;
  return <><AppShell inspector={panel} collapsed={!ui.sidebarOpen} nav={<SideNav cloudAuth={local ? repository.cloudAuth : undefined} project={route.page === 'workspace' ? project : undefined} page={route.page} view={ui.view} local={local} authority={sharing.data?.authority}
    diagnostics={local ? repository.diagnostics : undefined} onView={ui.setView}
    onProjects={() => { setArchived(false); navigate(null); }} onHelp={() => setDialog({ kind: 'help' })} />}>
    {query.isPending ? <p role="status" className="page-message">正在加载项目…</p> : query.isError ? <EmptyState title="项目加载失败" hint={query.error.message} action={<Button onClick={() => { void query.refetch(); }}>重试</Button>} />
      : route.page === 'projects' ? <ProjectList projects={projects} onOpen={openProject} onCreate={() => { setArchived(false); setPendingGoal(null); setDialog({ kind: 'create' }); }} onQuickStart={local ? () => { setArchived(false); setDialog({ kind: 'quickStart' }); } : undefined} local={local} archived={archived}
        onToggleArchive={() => setArchived(value => !value)} onArchive={item => setDialog({ kind: 'archive', project: item })}
        onRestore={item => { if (local) act(() => repository.restore(item.id), '项目已恢复', false, () => { setArchived(false); refresh(); }); }} />
      : project ? <Workspace key={project.id} canvas={local ? repository.canvas : undefined} publication={local ? repository.publication : undefined} project={project} visible={visible} kept={kept} active={active} selectedIds={selectedIds} onSelect={select} onBatchDecide={decideSelection} onBatchRate={rateSelection} busy={mutation.isPending || readOnly} local={local} sharing={sharing.data ?? undefined} onEdit={() => setDialog({ kind: 'edit' })} onImport={() => setDialog({ kind: 'import' })}
        onEditorVersions={local && repository.editorVersions ? () => setDialog({ kind: 'editorVersions' }) : undefined} onUndo={undo} onRedo={redo} onVersions={() => setDialog({ kind: 'versions' })} onActivity={() => setDialog({ kind: 'activity' })} onCopyPlan={copyPlan} onAICompare={aiCompare} onAIOrder={aiOrder} onCompare={compare} onExport={() => setDialog({ kind: 'export' })} onDecide={decide} onReorder={reorder} onEnlarge={photo => setDialog({ kind: 'enlarge', photo })} />
        : <EmptyState title={local ? '项目未找到' : '项目未找到'} hint={local ? '项目可能已归档，请返回项目列表查看。' : '请返回项目列表查看。'} action={<Button onClick={() => navigate(null)}>{zh.projects}</Button>} />}
  </AppShell>
    {ui.aiOpen && project && route.page === 'workspace' && <AISelectionDrawer key={project.id} projectTitle={project.title} onClose={ui.closeAgent}
      memoryOpen={ui.aiMemoryOpen} onMemoryChange={ui.setAIMemoryOpen}
      memory={<PreferencePanel events={project.events} repository={local ? repository : undefined} project={project} authority={sharing.data?.authority} />}>
      {(settingsOpen, closeSettings) => <AIPanel key={project.id} repository={local ? repository : undefined} project={project} selectedIds={selectedIds}
        initialGoal={pendingGoal?.projectId === project.id ? pendingGoal.goal : project.brief}
        initialCount={pendingGoal?.projectId === project.id ? pendingGoal.count : Math.min(parseGoal(project.brief).count, project.photos.length || 100)}
        quickAction={quickAction?.projectId === project.id ? quickAction : null} onQuickActionFired={() => setQuickAction(null)}
        settingsOpen={settingsOpen} onSettingsClose={closeSettings}
        onUpdated={updated => { client.setQueryData<Project[]>(['projects', false], previous => previous?.map(item => item.id === updated.id ? updated : item)); refresh(); }}>
        <ProposalsPanel project={project} enabled={local} /></AIPanel>}
    </AISelectionDrawer>}
    {local && project && !ui.aiOpen && <AgentStatus repository={repository} projectId={project.id} onOpen={openAI} />}
    {notice && <div role="status" className="toast">{notice}</div>}
    {dialog?.kind === 'quickStart' && local && <QuickStartModal busy={mutation.isPending} onClose={() => setDialog(null)} onCreate={(title, goal, count) => { mutation.mutate(() => repository.create(title, goal), { onSuccess: created => { setPendingGoal({ projectId: created.id, goal, count }); ui.reset(); navigate(created.id); setDialog({ kind: 'import' }); } }); }} />}
      {(dialog?.kind === 'create' || dialog?.kind === 'edit') && <ProjectDialog project={dialog.kind === 'edit' ? project : undefined} busy={mutation.isPending} onClose={() => setDialog(null)} onSave={(title, brief, tags) => {
      if (dialog.kind === 'edit' && project) act(async () => { const updated = await repository.edit(project.id, title, brief); if (tags) await repository.setTags(project.id, tags); return updated; }, local ? '项目已保存到本地' : zh.savedProject, true);
      else mutation.mutate(() => repository.create(title, brief), { onSuccess: created => { setDialog(null); ui.reset(); ui.select(null); navigate(created.id); } });
    }} />}
    {dialog?.kind === 'compare' && project && <CompareModal left={dialog.left} right={dialog.right} busy={mutation.isPending} onClose={() => setDialog(null)} onCompare={(preferred, reason) => act(() => repository.compare(project.id, dialog.left.id, dialog.right.id, preferred, reason), zh.savedCompare, true)} />}
    {dialog?.kind === 'versions' && project && <VersionsDialog project={project} busy={mutation.isPending} onClose={() => setDialog(null)}
      onSave={label => act(() => repository.saveVersion(project.id, label), '当前作品版本已保存')}
      onRestore={versionId => act(() => repository.restoreVersion(project.id, versionId), '版本已恢复，可撤销此操作', true)} />}
    {dialog?.kind === 'activity' && project && <ActivityDialog project={project} onClose={() => setDialog(null)} />}
    {dialog?.kind === 'export' && <ExportDialog repository={local ? repository : undefined} projectId={project?.id} revision={project?.revision} onChanged={refresh} onClose={() => setDialog(null)} />}
    {dialog?.kind === 'import' && (local ? project && <ImportDialog repository={repository} projectId={project.id} onClose={() => { setDialog(null); refresh(); }} onChanged={refresh} />
      : <Modal title={zh.import} description="请在桌面版导入照片。" onClose={() => setDialog(null)}><div className="dialog-actions"><Button onClick={() => setDialog(null)}>继续体验示例项目</Button></div></Modal>)}
    {dialog?.kind === 'archive' && local && <Modal title="确认归档项目" description={`“${dialog.project.title}”将移至归档列表，可随时恢复。原图、备注和决策日志全部保留。`} onClose={() => setDialog(null)}>
      <div className="dialog-actions"><Button onClick={() => setDialog(null)}>取消</Button><Button variant="primary" onClick={() => {
        void repository.archive(dialog.project.id).then(() => { setDialog(null); refresh(); }).catch(error => setNotice(`归档失败：${String(error)}`));
      }}>确认归档</Button></div></Modal>}
    {dialog?.kind === 'enlarge' && <Lightbox photos={navigable} activeId={dialog.photo.id} onClose={() => setDialog(null)} onNavigate={photo => { ui.select(photo.id); setDialog({ kind: 'enlarge', photo }); }} onDecide={(id, status) => decide(id, status)} onRate={(id, rating) => project && act(() => repository.rate(project.id, [id], rating), rating ? `人工评分已保存：${rating} 星` : '人工评分已清除')} />}
    {dialog?.kind === 'palette' && <CommandPalette commands={paletteCommands} onClose={() => setDialog(null)} />}
    {dialog?.kind === 'editorVersions' && local && project && repository.editorVersions && <EditorVersionsDialog repository={repository.editorVersions} projectId={project.id} readOnly={readOnly} onClose={() => setDialog(null)} />}
    {dialog?.kind === 'help' && <Modal title={zh.help} description={zh.helpHint} onClose={() => setDialog(null)}>
      <div style={{marginBottom: 16, paddingBottom: 12, borderBottom: '1px solid var(--border, #2a2a40)'}}><UpdateCheck /></div>
      <ShortcutEditor shortcuts={shortcutConfig.shortcuts} overrides={shortcutConfig.overrides} save={shortcutConfig.save} reset={shortcutConfig.reset} />
    </Modal>}
  </>;
}
