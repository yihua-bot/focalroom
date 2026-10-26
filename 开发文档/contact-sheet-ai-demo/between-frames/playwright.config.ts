import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e', fullyParallel: false, workers: 1,
  testIgnore: ['local-core.spec.ts', 'agent-core.spec.ts', 'beta-journey.spec.ts', 'memory-controls.spec.ts', 'diagnostics.spec.ts', 'v2-foundation.spec.ts', 'v2-proposals.spec.ts', 'm1-500.spec.ts', 'v3-workspace.spec.ts', 'editorial-review.spec.ts', 'focal-agent.spec.ts', 'focal-creative.spec.ts', 'account-experience.spec.ts'],
  outputDir: 'test-results/preview',
  use: { locale: 'zh-CN', baseURL: 'http://localhost:1420', viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure' },
  webServer: [
    { command: 'npm run dev', url: 'http://localhost:1420', reuseExistingServer: !process.env.CI },
    { command: 'FOCALROOM_TEST_RPC_PORT=1447 CARGO_TARGET_DIR=target-v1 node "scripts/integration-server.mjs"', url: 'http://localhost:1447/health', reuseExistingServer: !process.env.CI },
  ],
});
