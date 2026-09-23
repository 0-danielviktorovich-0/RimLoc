import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Mock-only scaffold: no Tauri plugin, no backend integration.
// The Tauri shell still serves ../frontend until this app graduates.
export default defineConfig({
  plugins: [svelte()],
  build: {
    target: 'es2022'
  }
});
