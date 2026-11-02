import { useEffect, useState } from 'react';
import { Button } from '../components/shared/Button';
import { SHORTCUT_ACTIONS, eventToShortcut, normalizeShortcut, type ShortcutActionId, type ShortcutMap } from '../app/shortcutConfig';
import { zh } from './copy';

export function formatShortcut(key: string): string {
  return key.replace('mod+', '⌘').replace('shift+', '⇧').replace('space', zh.spaceKey).replace('escape', 'Esc');
}

const FIXED_ROWS = () => [
  { keys: '← →', labelKey: 'fixedArrowPhotos' },
  { keys: '↑ ↓', labelKey: 'fixedArrowSequence' },
  { keys: 'Shift + 点击', labelKey: 'fixedShiftClick' },
];

/** 帮助弹窗内的快捷键编辑器：点击「修改」后捕获下一次按键，冲突不落盘。 */
export function ShortcutEditor({ shortcuts, overrides, save, reset }: {
  shortcuts: ShortcutMap; overrides: Partial<ShortcutMap>; save: (next: Partial<ShortcutMap>) => ShortcutActionId | null; reset: () => void;
}) {
  const [editing, setEditing] = useState<ShortcutActionId | null>(null);
  const [conflict, setConflict] = useState('');
  const labelOf = (id: ShortcutActionId) => { const action = SHORTCUT_ACTIONS.find(item => item.id === id); return action ? zh[action.labelKey] : id; };
  useEffect(() => {
    if (!editing) return;
    const capture = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') { setEditing(null); return; }
      const normalized = normalizeShortcut(eventToShortcut(event));
      if (!normalized) return;
      const clash = save({ [editing]: normalized });
      if (clash) { setConflict(zh.shortcutConflict.replace('{action}', labelOf(editing)).replace('{keys}', formatShortcut(normalized)).replace('{clash}', labelOf(clash))); return; }
      setConflict('');
      setEditing(null);
    };
    window.addEventListener('keydown', capture, true);
    return () => window.removeEventListener('keydown', capture, true);
  }, [editing, save]);
  const groups = [...new Set(SHORTCUT_ACTIONS.map(action => action.groupKey))];
  return <div className="shortcut-editor">
    {conflict && <p role="alert" className="shortcut-conflict">{conflict}</p>}
    {groups.map(group => <section key={group}>
      <h4>{zh[group]}</h4>
      <ul className="shortcut-edit-list">
        {SHORTCUT_ACTIONS.filter(action => action.groupKey === group).map(action => <li key={action.id}>
          <span>{zh[action.labelKey]}{overrides[action.id] && <em className="shortcut-custom">{zh.shortcutCustomised}</em>}</span>
          <span className="shortcut-edit-keys">
            {editing === action.id ? <kbd className="shortcut-capturing">{zh.pressNewKey}</kbd> : <kbd>{formatShortcut(shortcuts[action.id])}</kbd>}
            <Button size="small" disabled={editing !== null && editing !== action.id} aria-pressed={editing === action.id}
              onClick={() => { setConflict(''); setEditing(editing === action.id ? null : action.id); }}>{editing === action.id ? zh['project.cancel'] : zh.editShortcut}</Button>
          </span>
        </li>)}
      </ul>
    </section>)}
    <section>
      <h4>{zh.fixedKeysTitle}</h4>
      <ul className="shortcut-edit-list">
        {FIXED_ROWS().map(row => <li key={row.keys}><span>{zh[row.labelKey]}</span><span className="shortcut-edit-keys"><kbd>{row.keys}</kbd></span></li>)}
      </ul>
    </section>
    <div className="dialog-actions">
      <Button size="small" onClick={reset} disabled={!Object.keys(overrides).length}>{zh.resetShortcuts}</Button>
    </div>
  </div>;
}
