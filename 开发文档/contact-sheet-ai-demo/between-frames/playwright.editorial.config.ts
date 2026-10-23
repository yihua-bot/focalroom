import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './e2e', testMatch: ['editorial-review.spec.ts'], workers: 1, timeout: 120_000,
  outputDir: 'test-results/editorial-review',
  use: { actionTimeout: 15_000, locale: 'zh-CN', baseURL: 'http://localhost:1446', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure' },
  webServer: [
    { command: 'npm run dev -- --port 1446', url: 'http://localhost:1446', reuseExistingServer: false },
    { command: 'node "scripts/integration-server.mjs"', env: { FOCALROOM_TEST_RPC_PORT: '1447', FOCALROOM_TEST_ORIGIN: 'http://localhost:1446' }, url: 'http://localhost:1447/health', reuseExistingServer: false },
  ],
});
