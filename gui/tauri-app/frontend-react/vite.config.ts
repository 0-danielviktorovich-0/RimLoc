import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

// React UI lane (R1): Tauri dev server on a DEDICATED port — the frozen
// Svelte frontend-v2 keeps its own port/config untouched (mandate §3/§48).
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(__dirname, 'src') },
  },
  clearScreen: false,
  server: {
    port: 5175,
    strictPort: true,
    fs: { allow: [path.resolve(__dirname), path.resolve(__dirname, '../..')] },
  },
  build: { outDir: 'dist' },
})
