import { fileURLToPath } from 'node:url';
import { workspaceConfig } from '../../scripts/workspace-vite.ts';
export default workspaceConfig(fileURLToPath(new URL('.', import.meta.url)), 1424);
