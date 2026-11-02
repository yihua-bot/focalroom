import { useState } from 'react';

/** 可配置快捷键动作清单。键位格式：单字符 / 'space' / 'escape' / 箭头，或 'mod+…'（⌘/Ctrl）组合。 */
export const SHORTCUT_ACTIONS = [
  { id: 'keep', labelKey: 'keepSelection', defaultKey: 'k', groupKey: 'shortcutGroupDecide' },
  { id: 'reject', labelKey: 'rejectSelection', defaultKey: 'x', groupKey: 'shortcutGroupDecide' },
  { id: 'undecided', labelKey: 'shortcutUndecided', defaultKey: 'u', groupKey: 'shortcutGroupDecide' },
  { id: 'rate1', labelKey: 'shortcutRate1', defaultKey: '1', groupKey: 'shortcutGroupRate' },
  { id: 'rate2', labelKey: 'shortcutRate2', defaultKey: '2', groupKey: 'shortcutGroupRate' },
  { id: 'rate3', labelKey: 'shortcutRate3', defaultKey: '3', groupKey: 'shortcutGroupRate' },
  { id: 'rate4', labelKey: 'shortcutRate4', defaultKey: '4', groupKey: 'shortcutGroupRate' },
  { id: 'rate5', labelKey: 'shortcutRate5', defaultKey: '5', groupKey: 'shortcutGroupRate' },
  { id: 'rate0', labelKey: 'clearRating', defaultKey: '0', groupKey: 'shortcutGroupRate' },
  { id: 'viewGrid', labelKey: 'switchToGrid', defaultKey: 'g', groupKey: 'shortcutGroupView' },
  { id: 'viewSequence', labelKey: 'switchToSequence', defaultKey: 's', groupKey: 'shortcutGroupView' },
  { id: 'compare', labelKey: 'compareTwoPhotos', defaultKey: 'c', groupKey: 'shortcutGroupInspect' },
  { id: 'enlarge', labelKey: 'openEnlargeView', defaultKey: 'space', groupKey: 'shortcutGroupInspect' },
  { id: 'clearSelection', labelKey: 'clearSelection', defaultKey: 'escape', groupKey: 'shortcutGroupSelect' },
  { id: 'selectAll', labelKey: 'selectAllInView', defaultKey: 'mod+a', groupKey: 'shortcutGroupSelect' },
  { id: 'search', labelKey: 'focusSearchField', defaultKey: 'mod+f', groupKey: 'shortcutGroupNavigate' },
  { id: 'toggleAgent', labelKey: 'toggleAgentShortcut', defaultKey: 'mod+shift+a', groupKey: 'shortcutGroupNavigate' },
  { id: 'commandPalette', labelKey: 'openCommandPalette', defaultKey: 'mod+k', groupKey: 'shortcutGroupNavigate' },
  { id: 'undo', labelKey: 'undoLastEdit', defaultKey: 'mod+z', groupKey: 'shortcutGroupHistory' },
  { id: 'redo', labelKey: 'common.redo', defaultKey: 'mod+shift+z', groupKey: 'shortcutGroupHistory' },
] as const;

export type ShortcutActionId = (typeof SHORTCUT_ACTIONS)[number]['id'];
export type ShortcutMap = Record<ShortcutActionId, string>;

const STORAGE_KEY = 'focalroom.shortcuts.v1';
export const DEFAULT_SHORTCUTS = Object.fromEntries(SHORTCUT_ACTIONS.map(action => [action.id, action.defaultKey])) as ShortcutMap;

const KEY_PATTERN = /^(mod\+shift\+|mod\+shift|mod\+)?([a-z0-9]|space|escape|arrow(left|right|up|down))$/;

/** 校验用户键位：非空、格式合法、不与默认冲突之外还须彼此唯一（重复由调用方合并时检测）。 */
export function normalizeShortcut(input: string): string | null {
  const value = input.trim().toLowerCase().replace(/\s+/g, '');
  return KEY_PATTERN.test(value) ? value : null;
}

/** 读取合并后的生效键位；localStorage 数据损坏或含非法值时整体回退默认。 */
export function loadShortcutOverrides(): Partial<ShortcutMap> {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};
    const overrides: Partial<ShortcutMap> = {};
    const validIds = new Set(SHORTCUT_ACTIONS.map(action => action.id));
    for (const [id, key] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof key !== 'string' || !validIds.has(id as ShortcutActionId)) continue;
      const normalized = normalizeShortcut(key);
      if (normalized) overrides[id as ShortcutActionId] = normalized;
    }
    return overrides;
  } catch {
    return {};
  }
}

export function effectiveShortcuts(overrides: Partial<ShortcutMap>): ShortcutMap {
  return { ...DEFAULT_SHORTCUTS, ...overrides };
}

/** 保存增量覆盖（与已存覆盖合并）；与其它动作键位重复时返回冲突动作，不落盘。 */
export function saveShortcutOverrides(delta: Partial<ShortcutMap>): ShortcutActionId | null {
  const overrides = { ...loadShortcutOverrides(), ...delta };
  const merged = effectiveShortcuts(overrides);
  const seen = new Map<string, ShortcutActionId>();
  for (const action of SHORTCUT_ACTIONS) {
    const key = merged[action.id];
    if (seen.has(key)) return action.id;
    seen.set(key, action.id);
  }
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
  } catch {
    /* 隐私模式下静默跳过持久化，仅本次会话生效。 */
  }
  return null;
}

export function resetShortcutOverrides() {
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* 同上。 */
  }
}

/** 键盘事件 → 规范化键位字符串（与配置格式一致）。 */
export function eventToShortcut(event: KeyboardEvent): string {
  const base = event.key === ' ' ? 'space' : event.key.toLowerCase();
  const mod = event.metaKey || event.ctrlKey ? 'mod+' : '';
  const shift = event.shiftKey && mod ? 'shift+' : '';
  return `${mod}${shift}${base}`;
}

/** React 状态封装：生效键位 + 保存/重置。 */
export function useShortcutConfig() {
  const [overrides, setOverrides] = useState<Partial<ShortcutMap>>(() => loadShortcutOverrides());
  const save = (delta: Partial<ShortcutMap>) => {
    const conflict = saveShortcutOverrides(delta);
    if (conflict) return conflict;
    setOverrides(loadShortcutOverrides());
    return null;
  };
  const reset = () => { resetShortcutOverrides(); setOverrides({}); };
  return { shortcuts: effectiveShortcuts(overrides), overrides, save, reset };
}
