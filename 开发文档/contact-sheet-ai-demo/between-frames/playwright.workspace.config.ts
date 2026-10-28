import { defineConfig } from '@playwright/test';
import publication from './playwright.publication.config';
export default defineConfig({...publication,testMatch:'v3-workspace.spec.ts',outputDir:'test-results/workspace'});
