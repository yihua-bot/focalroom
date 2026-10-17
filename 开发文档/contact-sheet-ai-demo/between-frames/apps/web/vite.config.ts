import { fileURLToPath } from 'node:url';
import { mergeConfig } from 'vite';
import { workspaceConfig } from '../../scripts/workspace-vite.ts';
export default mergeConfig(workspaceConfig(fileURLToPath(new URL('.', import.meta.url)), 1424), { server: { proxy: { '/api/v2': { target: process.env.FOCALROOM_DEV_API_URL ?? 'http://127.0.0.1:3002', ws: true } } } });
