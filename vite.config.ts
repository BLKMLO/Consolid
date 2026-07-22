import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tauri from '@tauri-apps/vite-plugin-tauri';

export default defineConfig({
  plugins: [
    svelte(),
    tauri(),
  ],
  resolve: {
    alias: {
      '@': '/src',
      '@components': '/src/components',
      '@stores': '/src/stores',
      '@lib': '/src/lib',
    },
  },
  build: {
    target: ['es2021', 'chrome100', 'safari15'],
    minify: true,
    sourcemap: true,
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});
