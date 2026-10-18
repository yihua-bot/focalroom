import { useEffect, useState } from 'react';
import { Button } from '../components/shared/Button';
import { SHORTCUT_ACTIONS, eventToShortcut, loadShortcutOverrides, normalizeShortcut, type ShortcutActionId, type ShortcutMap } from '../app/shortcutConfig';

export function formatShortcut(key: string): string {
  return key.replace('mod+', '⌘').replace('shift+', '⇧').replace('space', '空格').replace('escape', 'Esc');
}

const FIXED_ROWS: { keys: string; label: string }[] = [
  { keys: '← →', label: '切换照片；Shift + 方向键扩展选区' },
  { keys: '↑ ↓', label: '在作品序列中移动当前照片' },
  { keys: 'Shift + 点击', label: '加入选区' },
];

/** 帮助弹窗内的快捷键编辑器：点击「修改」后捕获下一次按键，冲突不落盘。 */
export function ShortcutEditor({ shortcuts, overrides, save, reset }: {
  shortcuts: ShortcutMap; overrides: Partial<ShortcutMap>; save: (next: Partial<ShortcutMap>) => ShortcutActionId | null; reset: () => void;
}) {
  const [editing, setEditing] = useState<ShortcutActionId | null>(null);
  const [conflict, setConflict] = useState('');
  const labelOf = (id: ShortcutActionId) => SHORTCUT_ACTIONS.find(action => action.id === id)?.label ?? id;
  useEffect(() => {
    if (!editing) return;
    const capture = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') { setEditing(null); return; }
      const normalized = normalizeShortcut(eventToShortcut(event));
      if (!normalized) return;
      const next = { ...loadShortcutOverrides(), [editing]: normalized };
      const clash = save(next);
      if (clash) { setConflict(`「${labelOf(editing)}」想用 ${formatShortcut(normalized)}，与「${labelOf(clash)}」冲突，未保存。`); return; }
      setConflict('');
      setEditing(null);
    };
    window.addEventListener('keydown', capture, true);
    return () => window.removeEventListener('keydown', capture, true);
  }, [editing, save]);
  const groups = [...new Set(SHORTCUT_ACTIONS.map(action => action.group))];
  return <div className="shortcut-editor">
    {conflict && <p role="alert" className="shortcut-conflict">{conflict}</p>}
    {groups.map(group => <section key={group}>
      <h4>{group}</h4>
      <ul className="shortcut-edit-list">
        {SHORTCUT_ACTIONS.filter(action => action.group === group).map(action => <li key={action.id}>
          <span>{action.label}{overrides[action.id] && <em className="shortcut-custom">（已自定义）</em>}</span>
          <span className="shortcut-edit-keys">
            {editing === action.id ? <kbd className="shortcut-capturing">按新按键…</kbd> : <kbd>{formatShortcut(shortcuts[action.id])}</kbd>}
            <Button size="small" disabled={editing !== null && editing !== action.id} aria-pressed={editing === action.id}
              onClick={() => { setConflict(''); setEditing(editing === action.id ? null : action.id); }}>{editing === action.id ? '取消' : '修改'}</Button>
          </span>
        </li>)}
      </ul>
    </section>)}
    <section>
      <h4>固定按键</h4>
      <ul className="shortcut-edit-list">
        {FIXED_ROWS.map(row => <li key={row.keys}><span>{row.label}</span><span className="shortcut-edit-keys"><kbd>{row.keys}</kbd></span></li>)}
      </ul>
    </section>
    <div className="dialog-actions">
      <Button size="small" onClick={reset} disabled={!Object.keys(overrides).length}>恢复默认快捷键</Button>
    </div>
  </div>;
}
