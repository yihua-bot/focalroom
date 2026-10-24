import { defineConfig } from '@playwright/test';
import editorial from './playwright.editorial.config';
export default defineConfig({ ...editorial, testMatch:['selection-revision.spec.ts','agent-core.spec.ts','beta-journey.spec.ts'], outputDir:'test-results/selection-review' });
