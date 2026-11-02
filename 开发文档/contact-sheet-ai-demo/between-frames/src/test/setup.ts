import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';
import { setCopyLocale } from '../app/copy';

// jsdom 不做布局；尺寸和缩放几何由真实浏览器 E2E 验证。
vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} unobserve() {} });

// 部分 jsdom 构建的 localStorage 是无方法空对象；本地命令回执队列（PersistentCommandQueue）依赖完整 Storage。
if (typeof window !== 'undefined' && typeof window.localStorage?.getItem !== 'function') {
  const memory = new Map<string, string>();
  const shim: Storage = {
    get length() { return memory.size; },
    key: index => [...memory.keys()][index] ?? null,
    getItem: key => memory.get(key) ?? null,
    setItem: (key, value) => void memory.set(key, String(value)),
    removeItem: key => void memory.delete(key),
    clear: () => void memory.clear(),
  };
  try {
    Object.defineProperty(window, 'localStorage', { configurable: true, writable: true, value: shim });
  } catch {
    vi.stubGlobal('localStorage', shim);
  }
}

// 用例按中文文案断言，界面语言必须先钉住：否则 detectLocale 跟随 jsdom 的 en-US，
// 整批组件渲染成英文，失败信息与真实缺陷混在一起。
try {
  window.localStorage.setItem('focalroom-locale', 'zh');
} catch {
  // 上面的 shim 兜底后仍不可用时，至少 setCopyLocale 生效。
}
setCopyLocale('zh');

afterEach(cleanup);
