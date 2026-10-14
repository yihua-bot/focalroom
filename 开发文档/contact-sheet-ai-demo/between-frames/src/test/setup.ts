import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

// jsdom 不做布局；尺寸和缩放几何由真实浏览器 E2E 验证。
vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} unobserve() {} });

afterEach(cleanup);
