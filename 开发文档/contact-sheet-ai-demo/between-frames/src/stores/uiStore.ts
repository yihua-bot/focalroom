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
  aiMode: 'chat' | 'task';
  setAiMode: (mode: 'chat' | 'task') => void;
  aiChat: Record<string, import('../domain/agent').ChatTurn[]>;
  pushChat: (projectId: string, turn: import('../domain/agent').ChatTurn) => void;
  popChat: (projectId: string, turn: import('../domain/agent').ChatTurn) => void;
  appendChat: (projectId: string, fragment: string) => void;
  settleChat: (projectId: string, content: string) => void;
  clearChat: (projectId: string) => void;
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
type ChatTurns = import('../domain/agent').ChatTurn[];
// 流式增量和最终回执都只作用在末条助手气泡上：末条不是助手就先新开一条，
// 用户刚发出的原文永远不会被改写。历史上限与 pushChat 同为 40 条。
const mapLastAssistant = (turns: ChatTurns, write: (previous: string | null) => string): ChatTurns => {
  const last = turns[turns.length - 1];
  const next = last?.role === 'assistant'
    ? [...turns.slice(0, -1), { ...last, content: write(last.content) }]
    : [...turns, { role: 'assistant' as const, content: write(null) }];
  return next.slice(-40);
};
export const useUIStore = create<UIState>(set => ({
  draftRevisionRequest: null,
  openDraftRevision: (projectId, reference, title, message, count) => set({ aiOpen: true, aiMemoryOpen: false, canvasTaskSource: null, view: 'canvas', panel: 'photo', draftRevisionRequest: { id: crypto.randomUUID(), projectId, reference, title, message, count } }),
  consumeDraftRevision: id => set(state => state.draftRevisionRequest?.id === id ? { draftRevisionRequest: null } : {}),
  aiDrafts: {},
  saveAIDraft: (projectId, draft) => set(state => ({ aiDrafts: { ...state.aiDrafts, [projectId]: draft } })),
  aiMode: 'task',
  setAiMode: aiMode => set({ aiMode }),
  aiChat: {},
  pushChat: (projectId, turn) => set(state => ({ aiChat: { ...state.aiChat, [projectId]: [...(state.aiChat[projectId] ?? []), turn].slice(-40) } })),
  // 只在末条与传入的乐观消息逐字相符时才撤回，避免误删已经到达的助手回复。
  popChat: (projectId, turn) => set(state => {
   const turns = state.aiChat[projectId] ?? [];
   const last = turns[turns.length - 1];
   return last && last.role === turn.role && last.content === turn.content ? { aiChat: { ...state.aiChat, [projectId]: turns.slice(0, -1) } } : {};
  }),
  // 流式增量：把刚到的一段文字追加到末条助手气泡（末条不是助手则新开一条）。
  appendChat: (projectId, fragment) => set(state => ({ aiChat: { ...state.aiChat, [projectId]: mapLastAssistant(state.aiChat[projectId] ?? [], previous => (previous ?? '') + fragment) } })),
  // 服务端回执是权威正文：整条覆盖末条助手气泡，避免增量拼接与最终结果不一致时留下半句。
  settleChat: (projectId, content) => set(state => ({ aiChat: { ...state.aiChat, [projectId]: mapLastAssistant(state.aiChat[projectId] ?? [], () => content) } })),
  clearChat: projectId => set(state => ({ aiChat: { ...state.aiChat, [projectId]: [] } })),
  ...initial, filter: 'all', grouping: 'none', view: 'grid', panel: 'photo',
  select: activeId => set({ activeId, selectedIds: activeId ? [activeId] : [], anchorId: activeId }),
  selectRange: (id, order, gesture = {}) => set(state => ({ activeId: id, selectedIds: selectPhotos(state.selectedIds, state.anchorId, id, order, gesture), anchorId: gesture.range ? state.anchorId ?? id : id })),
  selectAll: selectedIds => set({ selectedIds, activeId: selectedIds[0] ?? null, anchorId: selectedIds[0] ?? null }),
  clearSelection: () => set({ selectedIds: [], anchorId: null }),
  // 筛选与搜索只描述网格看法，不接管用户当前视图，也不动画布上下文。
  setSearch: search => set({ search, selectedIds: [], anchorId: null, activeId: null }),
  setFilter: filter => set({ filter, selectedIds: [], anchorId: null, activeId: null }),
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
  openAgent: () => set({ aiOpen: true, aiMemoryOpen: false, canvasTaskSource: null, panel: 'photo' }),
  closeAgent: () => set({ aiOpen: false }),
  toggleAgent: () => set(state => ({ aiOpen: !state.aiOpen, aiMemoryOpen: false, canvasTaskSource: null, panel: 'photo' })),
  reset: () => set({ ...initial, draftRevisionRequest: null, selectedIds: [...initial.selectedIds], filter: 'all', view: 'grid', panel: 'photo' }),
}));
