import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';

export function workspaceConfig(directory: string, port: number) {
  const root = resolve(directory);
  const repository = resolve(root, '../..');
  return defineConfig({
    root, plugins: [react(), tailwindcss()],
    resolve: { dedupe: ['react', 'react-dom', '@tanstack/react-query'] },
    server: { host: '127.0.0.1', port, strictPort: true, fs: { allow: [repository, resolve(repository, '../assets')] } },
    build: { outDir: resolve(root, 'dist'), emptyOutDir: true },
  });
}
