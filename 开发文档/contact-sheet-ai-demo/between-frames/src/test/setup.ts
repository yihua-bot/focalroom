import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

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

afterEach(cleanup);
