import { create } from 'zustand';
import type { GridGrouping } from '../domain/grouping';
import type { InspectorPanel, PhotoFilter, WorkspaceView } from '../domain/models';
import { selectPhotos, type SelectionGesture } from '../domain/selection';

interface UIState {
  draftRevisionRequest: { id: string; projectId: string; reference: import('../domain/agent').DraftReference; title: string; message: string; count: number } | null;
  openDraftRevision: (projectId: string, reference: import('../domain/agent').DraftReference, title: string, message: string, count: number) => void;
  consumeDraftRevision: (id: string) => void;
  canvasTaskSource: import('../domain/agent').CanvasTaskSource | null;
  canvasBoardId: string | null;
  canvasBoardRequest: number;
  openCanvasAgent: (source: import('../domain/agent').CanvasTaskSource) => void;
  openCanvasBoard: (boardId: string) => void;
  aiOpen: boolean;
  aiMemoryOpen: boolean;
  setAIMemoryOpen: (open: boolean) => void;
  openAgent: () => void;
  closeAgent: () => void;
  aiDrafts: Record<string, { goal: string; count: number; scopeSelection: boolean }>;
  saveAIDraft: (projectId: string, draft: { goal: string; count: number; scopeSelection: boolean }) => void;
  activeId: string | null;
  selectedIds: string[];
  anchorId: string | null;
  search: string;
  filmstripOpen: boolean;
  sidebarOpen: boolean;
  filter: PhotoFilter;
  grouping: GridGrouping;
  view: WorkspaceView;
  panel: InspectorPanel;
  dense: boolean;
  inspectorOpen: boolean;
  select: (id: string | null) => void;
  selectRange: (id: string, order: string[], gesture?: SelectionGesture) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
  setSearch: (query: string) => void;
  toggleFilmstrip: () => void;
  toggleSidebar: () => void;
  toggleAgent: () => void;
  setFilter: (filter: PhotoFilter) => void;
  setGrouping: (grouping: GridGrouping) => void;
  setView: (view: WorkspaceView) => void;
  setPanel: (panel: InspectorPanel) => void;
  toggleDensity: () => void;
  toggleInspector: () => void;
  reset: () => void;
}
const initial = { canvasBoardRequest: 0, canvasTaskSource: null as import('../domain/agent').CanvasTaskSource | null, canvasBoardId: null as string | null, aiOpen: false, aiMemoryOpen: false, activeId: null as string | null, selectedIds: [] as string[], anchorId: null as string | null, search: '', filmstripOpen: true, sidebarOpen: true, filter: 'all' as PhotoFilter, grouping: 'none' as GridGrouping, view: 'grid' as WorkspaceView, panel: 'photo' as InspectorPanel, dense: false, inspectorOpen: true };
export const useUIStore = create<UIState>(set => ({
  draftRevisionRequest: null,
  openDraftRevision: (projectId, reference, title, message, count) => set({ aiOpen: true, aiMemoryOpen: false, canvasTaskSource: null, view: 'canvas', panel: 'photo', draftRevisionRequest: { id: crypto.randomUUID(), projectId, reference, title, message, count } }),
  consumeDraftRevision: id => set(state => state.draftRevisionRequest?.id === id ? { draftRevisionRequest: null } : {}),
  aiDrafts: {},
  saveAIDraft: (projectId, draft) => set(state => ({ aiDrafts: { ...state.aiDrafts, [projectId]: draft } })),
  ...initial, filter: 'all', grouping: 'none', view: 'grid', panel: 'photo',
  select: activeId => set({ activeId, selectedIds: activeId ? [activeId] : [], anchorId: activeId }),
  selectRange: (id, order, gesture = {}) => set(state => ({ activeId: id, selectedIds: selectPhotos(state.selectedIds, state.anchorId, id, order, gesture), anchorId: gesture.range ? state.anchorId ?? id : id })),
  selectAll: selectedIds => set({ selectedIds, activeId: selectedIds[0] ?? null, anchorId: selectedIds[0] ?? null }),
  clearSelection: () => set({ selectedIds: [], anchorId: null }),
  setSearch: search => set({ search, selectedIds: [], anchorId: null, activeId: null, view: 'grid' }),
  setFilter: filter => set({ filter, view: 'grid', selectedIds: [], anchorId: null, activeId: null }),
  setGrouping: grouping => set({ grouping }),
  setView: view => set(state => ({ view, aiOpen: false, aiMemoryOpen: false, panel: 'photo', inspectorOpen: true,
    selectedIds: view === 'canvas' ? state.selectedIds : [], anchorId: view === 'canvas' ? state.anchorId : null })),
  // 兼容旧记忆入口，统一打开 Focal AI 抽屉内的记忆页。
  setPanel: panel => set({ panel: 'photo', aiOpen: panel === 'memory', aiMemoryOpen: panel === 'memory', inspectorOpen: true }),
  setAIMemoryOpen: aiMemoryOpen => set({ aiMemoryOpen }),
  toggleDensity: () => set(state => ({ dense: !state.dense })),
  toggleInspector: () => set(state => ({ inspectorOpen: !state.inspectorOpen })),
  toggleFilmstrip: () => set(state => ({ filmstripOpen: !state.filmstripOpen })),
  toggleSidebar: () => set(state => ({ sidebarOpen: !state.sidebarOpen })),
  openCanvasAgent: canvasTaskSource => set({ aiOpen: true, aiMemoryOpen: false, canvasTaskSource, view: 'canvas', panel: 'photo' }),
  openCanvasBoard: canvasBoardId => set(state => ({ canvasBoardId, canvasBoardRequest: state.canvasBoardRequest + 1, view: 'canvas', aiOpen: false, aiMemoryOpen: false, panel: 'photo', selectedIds: [], activeId: null })),
  openAgent: () => set({ aiOpen: true, aiMemoryOpen: false, canvasTaskSource: null, view: 'grid', panel: 'photo' }),
  closeAgent: () => set({ aiOpen: false }),
  toggleAgent: () => set(state => ({ aiOpen: !state.aiOpen, aiMemoryOpen: false, canvasTaskSource: null, view: 'grid', panel: 'photo' })),
  reset: () => set({ ...initial, draftRevisionRequest: null, selectedIds: [...initial.selectedIds], filter: 'all', view: 'grid', panel: 'photo' }),
}));
