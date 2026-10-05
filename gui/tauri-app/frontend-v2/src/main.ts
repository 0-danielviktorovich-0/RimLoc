import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { router } from './lib/router.svelte';
import { theme } from './lib/stores/theme.svelte';

// Screenshot/session bootstrap: URL params force the route + theme for
// deterministic captures (?screen=workspace&theme=dark). The legacy
// ?screen=home|workspace values map onto hash routes; ?style=…&palette=…&
// density=… are owned by the Style Lab store.
{
  const params = new URLSearchParams(window.location.search);
  const screen = params.get('screen');
  if (screen === 'workspace') router.navigate('workspace');
  else if (screen === 'home') router.navigate('home');
  const forcedTheme = params.get('theme');
  if (forcedTheme === 'light' || forcedTheme === 'dark') theme.set(forcedTheme);
}

// Agent automation bridge — COMPILE-TIME gate (owner §B, 2026-10-01):
// the WDIO guest plugin chunk is emitted ONLY when the frontend is built
// with VITE_RIMLOC_AUTOMATION=1 (automation artifact). In a production
// build this whole path is dead code: no chunk ships, nothing to load,
// and no runtime env can conjure it. The Rust side pairs this with the
// cargo feature `automation-bridge` (no listener in production either).
declare global {
  interface Window {
    __RIMLOC_WDIO__?: boolean;
  }
}
const AUTOMATION_BUILD = import.meta.env.VITE_RIMLOC_AUTOMATION === '1';
function maybeLoadWdioBridge(): boolean {
  if (!AUTOMATION_BUILD) return false;
  if (window.__RIMLOC_WDIO__) {
    void import('@wdio/tauri-plugin');
    return true;
  }
  return false;
}
if (AUTOMATION_BUILD) {
  // Rust park loop evals the flag every 1.5s for only ~15s from SETUP —
  // on a cold packaged start the page can finish loading AFTER all 10
  // dispatches (observed: bridge never initialized under the wdio service
  // while manual spawns worked). Poll makes the handshake order-proof.
  if (!maybeLoadWdioBridge()) {
    window.addEventListener('rimloc:wdio', maybeLoadWdioBridge, { once: true });
    const handshake = setInterval(() => {
      if (maybeLoadWdioBridge()) clearInterval(handshake);
    }, 500);
  }
}

const app = mount(App, {
  target: document.getElementById('app')!
});

export default app;
