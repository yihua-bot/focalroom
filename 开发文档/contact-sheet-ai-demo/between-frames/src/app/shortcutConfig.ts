import { useState } from 'react';

/** 可配置快捷键动作清单。键位格式：单字符 / 'space' / 'escape' / 箭头，或 'mod+…'（⌘/Ctrl）组合。 */
export const SHORTCUT_ACTIONS = [
  { id: 'keep', label: '保留选区', defaultKey: 'k', group: '决策' },
  { id: 'reject', label: '淘汰选区', defaultKey: 'x', group: '决策' },
  { id: 'undecided', label: '选区设为待定', defaultKey: 'u', group: '决策' },
  { id: 'rate1', label: '评 1 星', defaultKey: '1', group: '评分' },
  { id: 'rate2', label: '评 2 星', defaultKey: '2', group: '评分' },
  { id: 'rate3', label: '评 3 星', defaultKey: '3', group: '评分' },
  { id: 'rate4', label: '评 4 星', defaultKey: '4', group: '评分' },
  { id: 'rate5', label: '评 5 星', defaultKey: '5', group: '评分' },
  { id: 'rate0', label: '清除评分', defaultKey: '0', group: '评分' },
  { id: 'viewGrid', label: '切换到网格视图', defaultKey: 'g', group: '视图' },
  { id: 'viewSequence', label: '切换到序列视图', defaultKey: 's', group: '视图' },
  { id: 'compare', label: '比较两张照片', defaultKey: 'c', group: '检查' },
  { id: 'enlarge', label: '打开放大视图', defaultKey: 'space', group: '检查' },
  { id: 'clearSelection', label: '清除选区', defaultKey: 'escape', group: '选择' },
  { id: 'selectAll', label: '选择当前视图全部照片', defaultKey: 'mod+a', group: '选择' },
  { id: 'search', label: '聚焦搜索框', defaultKey: 'mod+f', group: '导航' },
  { id: 'toggleAgent', label: '打开/关闭 AI 智能选片', defaultKey: 'mod+shift+a', group: '导航' },
  { id: 'commandPalette', label: '打开命令面板', defaultKey: 'mod+k', group: '导航' },
  { id: 'undo', label: '撤销上一步人工编辑', defaultKey: 'mod+z', group: '历史' },
  { id: 'redo', label: '重做', defaultKey: 'mod+shift+z', group: '历史' },
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
