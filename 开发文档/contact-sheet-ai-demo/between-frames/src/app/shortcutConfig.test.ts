import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_SHORTCUTS, SHORTCUT_ACTIONS, effectiveShortcuts, eventToShortcut, loadShortcutOverrides, normalizeShortcut, resetShortcutOverrides, saveShortcutOverrides } from './shortcutConfig';

// 本 jsdom 环境未实现 Web Storage，用内存版替换并集中清理。
const store = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => store.get(key) ?? null,
  setItem: (key: string, value: string) => void store.set(key, value),
  removeItem: (key: string) => void store.delete(key),
  clear: () => store.clear(),
});

const press = (key: string, modifiers: { meta?: boolean; ctrl?: boolean; shift?: boolean } = {}) =>
  new KeyboardEvent('keydown', { key, metaKey: modifiers.meta, ctrlKey: modifiers.ctrl, shiftKey: modifiers.shift });

describe('快捷键配置', () => {
  beforeEach(() => {
    store.clear();
    resetShortcutOverrides();
  });
  afterEach(() => { store.clear(); });
  it('默认表覆盖全部动作且键位唯一', () => {
    expect(SHORTCUT_ACTIONS.length).toBeGreaterThanOrEqual(20);
    expect(Object.keys(DEFAULT_SHORTCUTS)).toHaveLength(SHORTCUT_ACTIONS.length);
    expect(new Set(Object.values(DEFAULT_SHORTCUTS)).size).toBe(SHORTCUT_ACTIONS.length);
  });
  it('键位规范化接受单键与 mod 组合并拒绝非法输入', () => {
    expect(normalizeShortcut('K')).toBe('k');
    expect(normalizeShortcut(' mod+Z ')).toBe('mod+z');
    expect(normalizeShortcut('mod+shift+z')).toBe('mod+shift+z');
    expect(normalizeShortcut('space')).toBe('space');
    expect(normalizeShortcut('')).toBeNull();
    expect(normalizeShortcut('ctrl+z')).toBeNull();
    expect(normalizeShortcut('mod+')).toBeNull();
  });
  it('保存合法覆盖并可读回；冲突键位不落盘并返回冲突动作', () => {
    expect(saveShortcutOverrides({ keep: 'b' })).toBeNull();
    expect(loadShortcutOverrides()).toEqual({ keep: 'b' });
    expect(effectiveShortcuts(loadShortcutOverrides()).keep).toBe('b');
    const conflict = saveShortcutOverrides({ reject: 'b' });
    expect(conflict).toBe('reject');
    expect(loadShortcutOverrides()).toEqual({ keep: 'b' });
  });
  it('损坏的持久化数据整体回退默认', () => {
    window.localStorage.setItem('focalroom.shortcuts.v1', '{bad json');
    expect(loadShortcutOverrides()).toEqual({});
    window.localStorage.setItem('focalroom.shortcuts.v1', JSON.stringify({ keep: 7, unknown: 'q', viewGrid: 'ctrl+x' }));
    expect(loadShortcutOverrides()).toEqual({});
  });
  it('键盘事件转规范化键位', () => {
    expect(eventToShortcut(press('k'))).toBe('k');
    expect(eventToShortcut(press(' ', {}))).toBe('space');
    expect(eventToShortcut(press('z', { meta: true }))).toBe('mod+z');
    expect(eventToShortcut(press('Z', { meta: true, shift: true }))).toBe('mod+shift+z');
    expect(eventToShortcut(press('ArrowRight'))).toBe('arrowright');
  });
  it('重置后回到默认', () => {
    saveShortcutOverrides({ keep: 'b' });
    resetShortcutOverrides();
    expect(loadShortcutOverrides()).toEqual({});
  });
});

describe('useShortcutConfig 状态封装', async () => {
  const { useShortcutConfig } = await import('./shortcutConfig');
  const { renderHook, act } = await import('@testing-library/react');
  beforeEach(() => { window.localStorage.clear(); resetShortcutOverrides(); });
  it('保存覆盖后生效键位更新，重置恢复默认', async () => {
    const { result } = renderHook(() => useShortcutConfig());
    expect(result.current.shortcuts.keep).toBe('k');
    act(() => { expect(result.current.save({ keep: 'b' })).toBeNull(); });
    expect(result.current.shortcuts.keep).toBe('b');
    expect(result.current.overrides).toEqual({ keep: 'b' });
    act(() => { result.current.reset(); });
    expect(result.current.shortcuts).toEqual(DEFAULT_SHORTCUTS);
    expect(result.current.overrides).toEqual({});
  });
  it('冲突保存返回冲突动作且不改状态', async () => {
    const { result } = renderHook(() => useShortcutConfig());
    act(() => { expect(result.current.save({ keep: 'b' })).toBeNull(); });
    act(() => { expect(result.current.save({ reject: 'b' })).toBe('reject'); });
    expect(result.current.shortcuts.reject).toBe('x');
  });
});
