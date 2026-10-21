import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e', fullyParallel: false, workers: 1,
  testIgnore: ['local-core.spec.ts', 'agent-core.spec.ts', 'beta-journey.spec.ts', 'memory-controls.spec.ts', 'diagnostics.spec.ts', 'v2-foundation.spec.ts', 'v2-proposals.spec.ts', 'm1-500.spec.ts', 'v3-workspace.spec.ts'],
  outputDir: 'test-results/preview',
  use: { locale: 'zh-CN', baseURL: 'http://localhost:1420', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure' },
  webServer: { command: 'npm run dev', url: 'http://localhost:1420', reuseExistingServer: !process.env.CI },
});
