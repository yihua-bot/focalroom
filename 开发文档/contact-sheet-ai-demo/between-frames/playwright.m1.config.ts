import { defineConfig } from '@playwright/test';
import integration from './playwright.integration.config';
export default defineConfig({
  ...integration,
  testMatch: 'm1-500.spec.ts',
  timeout: 240_000,
  use: { ...integration.use, actionTimeout: 15_000, trace: 'off' },
  outputDir: 'test-results/m1-500',
  webServer: [
    { command: 'npm run dev', url: 'http://localhost:1420', reuseExistingServer: !process.env.CI },
    { command: 'node "scripts/integration-server.mjs" --fixtures=m1-500 --release', url: 'http://localhost:1422/health', reuseExistingServer: false, timeout: 180_000 },
  ],
});
