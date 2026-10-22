import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './e2e', testMatch: 'account-experience.spec.ts', workers: 1, timeout: 90_000,
  outputDir: 'test-results/account-experience',
  use: { baseURL: 'http://localhost:1420', locale: 'zh-CN', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure' },
  webServer: [
    { command: 'npm run dev', url: 'http://localhost:1420', reuseExistingServer: !process.env.CI },
    { command: 'node scripts/integration-server.mjs', url: 'http://localhost:1422/health', reuseExistingServer: false },
    { command: 'npm run dev:web', url: 'http://127.0.0.1:1424', reuseExistingServer: !process.env.CI },
  ],
});
