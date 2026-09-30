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

// Agent automation bridge (RIMLOC_AUTOMATION=1, frontier §10): the WDIO
// guest plugin loads ONLY in automation sessions — the Rust setup hook sets
// window.__RIMLOC_WDIO__ and dispatches 'rimloc:wdio' after the page is
// live (with retries), so both orders (flag first or listener first) work.
// User sessions never import the bridge at all.
declare global {
  interface Window {
    __RIMLOC_WDIO__?: boolean;
  }
}
function maybeLoadWdioBridge(): boolean {
  if (window.__RIMLOC_WDIO__) {
    void import('@wdio/tauri-plugin');
    return true;
  }
  return false;
}
if (!maybeLoadWdioBridge()) {
  window.addEventListener('rimloc:wdio', maybeLoadWdioBridge, { once: true });
}

const app = mount(App, {
  target: document.getElementById('app')!
});

export default app;
