import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath, URL } from 'node:url';

// Tauri serves the dev server on a fixed port and needs a predictable build.
export default defineConfig({
  plugins: [react()],
  resolve: {
    // Mirrors the `paths` entry in tsconfig.json.
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // WebKitGTK on Ubuntu 24.04 is 2.52, comfortably modern.
    target: 'es2022',
    minify: 'esbuild',
    sourcemap: false,
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        // Shiki is deliberately NOT grouped here. Its grammars are loaded
        // through per-language dynamic imports, so Rollup should emit one
        // small chunk per language; forcing them into a shared chunk would
        // pull every grammar down the first time any code block renders.
        manualChunks(id) {
          if (id.includes('node_modules/shiki') || id.includes('@shikijs')) return;
          if (
            id.includes('node_modules/react-markdown') ||
            id.includes('node_modules/remark') ||
            id.includes('node_modules/mdast') ||
            id.includes('node_modules/micromark') ||
            id.includes('node_modules/hast') ||
            id.includes('node_modules/unist')
          ) {
            return 'markdown';
          }
          if (id.includes('node_modules/react')) return 'react';
          return;
        },
      },
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
