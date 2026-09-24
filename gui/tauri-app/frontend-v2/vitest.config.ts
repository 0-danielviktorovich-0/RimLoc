import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Component-level regression tests for the GUI waves (jsdom + Svelte 5 mount,
// no testing-library). Same plugin pipeline as the production build.
export default defineConfig({
  plugins: [svelte()],
  // Svelte 5 ships client/server builds; force the client one so `mount`
  // works under jsdom (the documented Svelte+Vitest setup).
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    setupFiles: ['tests/setup.ts']
  }
});
