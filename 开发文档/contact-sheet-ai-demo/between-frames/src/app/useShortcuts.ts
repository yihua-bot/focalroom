import { useEffect } from 'react';
import type { Photo, PhotoStatus, WorkspaceView } from '../domain/models';
import type { SelectionGesture } from '../domain/selection';

export function useShortcuts({ photos, activeId, view, blocked, onSelect, onSelectAll, onClear, onDecide, onRate, onCompare, onEnlarge, onView, onSearch, onAgent, onUndo, onRedo, onMove }: {
  photos: Photo[]; activeId: string | null; view: WorkspaceView; blocked: boolean;
  onSelect: (id: string, gesture?: SelectionGesture) => void; onSelectAll: (ids: string[]) => void; onClear: () => void;
  onDecide: (id: string, status: PhotoStatus) => void; onRate: (rating: number) => void;
  onCompare: () => void; onEnlarge: () => void; onView: (view: WorkspaceView) => void;
  onSearch: () => void; onAgent: () => void; onUndo: () => void; onRedo: () => void; onMove: (offset: number) => void;
}) {
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      const target = event.target instanceof HTMLElement ? event.target : null;
      if (blocked || event.defaultPrevented || event.repeat || event.altKey || event.isComposing || target?.closest('input, textarea, select, [contenteditable="true"], [role="dialog"], .drag-handle')) return;
      const key = event.key.toLowerCase();
      if (event.metaKey || event.ctrlKey) {
        if (key === 'z') { event.preventDefault(); if (event.shiftKey) onRedo(); else onUndo(); return; }
        if (key === 'f') { event.preventDefault(); onSearch(); }
        if (key === 'a') { event.preventDefault(); if (event.shiftKey) onAgent(); else onSelectAll(photos.map(photo => photo.id)); }
        return;
      }
      if (key === 'g' || key === 's') { event.preventDefault(); onView(key === 'g' ? 'grid' : 'sequence'); return; }
      if (key === 'escape') { event.preventDefault(); onClear(); return; }
      const index = photos.findIndex(photo => photo.id === activeId);
      if (index < 0) return;
      if (key === 'k' || key === 'x' || key === 'u') { event.preventDefault(); onDecide(photos[index].id, key === 'k' ? 'keep' : key === 'x' ? 'reject' : 'undecided'); }
      if (/^[0-5]$/.test(key)) { event.preventDefault(); onRate(Number(key)); }
      if (key === ' ' && (!target?.closest('button, a') || target?.closest('.photo-wrap, .sequence-photo'))) { event.preventDefault(); onEnlarge(); }
      if (key === 'c') { event.preventDefault(); onCompare(); }
      if (key === 'arrowright' || key === 'arrowleft') {
        event.preventDefault(); onSelect(photos[Math.max(0, Math.min(photos.length - 1, index + (key === 'arrowright' ? 1 : -1)))].id, { range: event.shiftKey });
      }
      if (view === 'sequence' && (key === 'arrowup' || key === 'arrowdown')) { event.preventDefault(); onMove(key === 'arrowup' ? -1 : 1); }
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, [photos, activeId, view, blocked, onSelect, onSelectAll, onClear, onDecide, onRate, onCompare, onEnlarge, onView, onSearch, onAgent, onUndo, onRedo, onMove]);
}
