import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './e2e', testMatch: ['local-core.spec.ts', 'agent-core.spec.ts', 'beta-journey.spec.ts', 'memory-controls.spec.ts', 'diagnostics.spec.ts', 'v2-proposals.spec.ts', 'v3-workspace.spec.ts'], workers: 1, timeout: 120_000,
  outputDir: 'test-results/integration',
  use: { actionTimeout: 15_000, locale: 'zh-CN', baseURL: 'http://localhost:1420', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure' },
  webServer: [
    { command: 'npm run dev', url: 'http://localhost:1420', reuseExistingServer: !process.env.CI },
    { command: 'node "scripts/integration-server.mjs"', url: 'http://localhost:1422/health', reuseExistingServer: false },
  ],
});
