import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './e2e', testMatch: 'v2-foundation.spec.ts', workers: 1,
  outputDir: 'test-results/v2-web',
  use: { baseURL: 'http://127.0.0.1:1424', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure' },
  webServer: { command: 'npm run dev:web', url: 'http://127.0.0.1:1424', reuseExistingServer: false },
});
