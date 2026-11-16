import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// 빌드 결과를 Tauri frontendDist(../ui)로 보낸다.
export default defineConfig({
  plugins: [svelte()],
  base: './',
  build: { outDir: '../ui', emptyOutDir: true, target: 'es2022', assetsInlineLimit: 0 },
});
