import { useEffect } from 'react';
import type { Photo, PhotoStatus, WorkspaceView } from '../domain/models';
import type { SelectionGesture } from '../domain/selection';
import { eventToShortcut, type ShortcutActionId, type ShortcutMap } from './shortcutConfig';

export function useShortcuts({ shortcuts, photos, activeId, view, blocked, onCommandPalette, onSelect, onSelectAll, onClear, onDecide, onRate, onCompare, onEnlarge, onView, onSearch, onAgent, onUndo, onRedo, onMove }: {
  shortcuts: ShortcutMap; photos: Photo[]; activeId: string | null; view: WorkspaceView; blocked: boolean; onCommandPalette: () => void;
  onSelect: (id: string, gesture?: SelectionGesture) => void; onSelectAll: (ids: string[]) => void; onClear: () => void;
  onDecide: (id: string, status: PhotoStatus) => void; onRate: (rating: number) => void;
  onCompare: () => void; onEnlarge: () => void; onView: (view: WorkspaceView) => void;
  onSearch: () => void; onAgent: () => void; onUndo: () => void; onRedo: () => void; onMove: (offset: number) => void;
}) {
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      // ⌘K 命令面板在任何场景（含弹窗打开、输入框聚焦）都可切换；与浏览器冲突时显式拦截。
      if (!event.defaultPrevented && !event.repeat && eventToShortcut(event) === shortcuts.commandPalette) {
        event.preventDefault();
        onCommandPalette();
        return;
      }
      const target = event.target instanceof HTMLElement ? event.target : null;
      // 序列拖拽进行中：所有按键让位给 dnd-kit（含 Space 放置与方向键移动），避免全局动作（如 space=放大）抢占。
      if (document.querySelector('.sequence-card.dragging')) return;
      if (!blocked && !event.defaultPrevented && !event.repeat && !event.isComposing && target?.closest('.ai-drawer') && eventToShortcut(event) === shortcuts.toggleAgent) {
        event.preventDefault(); onAgent(); return;
      }
      if (blocked || event.defaultPrevented || event.repeat || event.altKey || event.isComposing || target?.closest('input, textarea, select, [contenteditable="true"], [role="dialog"], .drag-handle')) return;
      const combo = eventToShortcut(event);
      const action = (Object.keys(shortcuts) as ShortcutActionId[]).find(id => shortcuts[id] === combo);
      if (view !== 'grid' && view !== 'sequence' && action !== 'viewGrid' && action !== 'viewSequence' && action !== 'toggleAgent') return;
      if (!action) {
        // 方向键保留固定行为：左右切换（Shift 扩展选区），序列视图内上下移动。
        if (combo === 'arrowright' || combo === 'arrowleft') {
          const index = photos.findIndex(photo => photo.id === activeId);
          if (index < 0) return;
          event.preventDefault();
          onSelect(photos[Math.max(0, Math.min(photos.length - 1, index + (combo === 'arrowright' ? 1 : -1)))].id, { range: event.shiftKey });
          return;
        }
        if (view === 'sequence' && (combo === 'arrowup' || combo === 'arrowdown')) { event.preventDefault(); onMove(combo === 'arrowup' ? -1 : 1); }
        return;
      }
      const index = photos.findIndex(photo => photo.id === activeId);
      switch (action) {
        case 'undo': event.preventDefault(); onUndo(); return;
        case 'redo': event.preventDefault(); onRedo(); return;
        case 'search': event.preventDefault(); onSearch(); return;
        case 'selectAll': event.preventDefault(); onSelectAll(photos.map(photo => photo.id)); return;
        case 'toggleAgent': event.preventDefault(); onAgent(); return;
        case 'viewGrid': event.preventDefault(); onView('grid'); return;
        case 'viewSequence': event.preventDefault(); onView('sequence'); return;
        case 'clearSelection': event.preventDefault(); onClear(); return;
        case 'compare': if (index < 0) return; event.preventDefault(); onCompare(); return;
        case 'enlarge':
          if (index < 0 || (!target?.closest('button, a') || target?.closest('.photo-wrap, .sequence-photo'))) { event.preventDefault(); onEnlarge(); }
          return;
        case 'keep': case 'reject': case 'undecided':
          if (index < 0) return;
          event.preventDefault();
          onDecide(photos[index].id, action === 'keep' ? 'keep' : action === 'reject' ? 'reject' : 'undecided');
          return;
        case 'rate0': case 'rate1': case 'rate2': case 'rate3': case 'rate4': case 'rate5':
          event.preventDefault();
          onRate(Number(action.slice('rate'.length)));
          return;
      }
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, [shortcuts, photos, activeId, view, blocked, onCommandPalette, onSelect, onSelectAll, onClear, onDecide, onRate, onCompare, onEnlarge, onView, onSearch, onAgent, onUndo, onRedo, onMove]);
}
