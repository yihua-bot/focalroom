import { defineConfig } from '@playwright/test';
import base from './playwright.integration.config';
export default defineConfig({
 ...base, testMatch: ['focal-agent.spec.ts', 'focal-creative.spec.ts', 'agent-core.spec.ts', 'beta-journey.spec.ts', 'v3-workspace.spec.ts'], outputDir: 'test-results/focal-agent',
 webServer: (Array.isArray(base.webServer) ? base.webServer : []).map(server => ({ ...server, reuseExistingServer: false })),
});
