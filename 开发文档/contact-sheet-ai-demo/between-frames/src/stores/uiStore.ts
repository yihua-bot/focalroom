import { create } from 'zustand';
import type { GridGrouping } from '../domain/grouping';
import type { InspectorPanel, PhotoFilter, WorkspaceView } from '../domain/models';
import { selectPhotos, type SelectionGesture } from '../domain/selection';

interface UIState {
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
const initial = { activeId: 'sample-1', selectedIds: ['sample-1'], anchorId: 'sample-1', search: '', filmstripOpen: true, sidebarOpen: true, filter: 'all' as PhotoFilter, grouping: 'none' as GridGrouping, view: 'grid' as WorkspaceView, panel: 'photo' as InspectorPanel, dense: false, inspectorOpen: true };
export const useUIStore = create<UIState>(set => ({
  ...initial, filter: 'all', grouping: 'none', view: 'grid', panel: 'photo',
  select: activeId => set({ activeId, selectedIds: activeId ? [activeId] : [], anchorId: activeId, panel: 'photo' }),
  selectRange: (id, order, gesture = {}) => set(state => ({ activeId: id, panel: 'photo', selectedIds: selectPhotos(state.selectedIds, state.anchorId, id, order, gesture), anchorId: gesture.range ? state.anchorId ?? id : id })),
  selectAll: selectedIds => set({ selectedIds, activeId: selectedIds[0] ?? null, anchorId: selectedIds[0] ?? null }),
  clearSelection: () => set({ selectedIds: [], anchorId: null }),
  setSearch: search => set({ search, selectedIds: [], anchorId: null, activeId: null, view: 'grid' }),
  setFilter: filter => set({ filter, view: 'grid', selectedIds: [], anchorId: null, activeId: null }),
  setGrouping: grouping => set({ grouping }),
  setView: view => set({ view, selectedIds: [], anchorId: null }),
  setPanel: panel => set({ panel, inspectorOpen: true }),
  toggleDensity: () => set(state => ({ dense: !state.dense })),
  toggleInspector: () => set(state => ({ inspectorOpen: !state.inspectorOpen })),
  toggleFilmstrip: () => set(state => ({ filmstripOpen: !state.filmstripOpen })),
  toggleSidebar: () => set(state => ({ sidebarOpen: !state.sidebarOpen })),
  toggleAgent: () => set(state => state.inspectorOpen && state.panel === 'ai' ? { inspectorOpen: false } : { panel: 'ai', inspectorOpen: true }),
  reset: () => set({ ...initial, selectedIds: [...initial.selectedIds], filter: 'all', view: 'grid', panel: 'photo' }),
}));
