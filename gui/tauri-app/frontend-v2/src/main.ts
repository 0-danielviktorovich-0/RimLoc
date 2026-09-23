import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { ui } from './lib/stores/ui.svelte';
import { theme } from './lib/stores/theme.svelte';

// Screenshot/session bootstrap: URL params force screen + theme for
// deterministic captures (?screen=workspace&theme=dark). Harmless otherwise.
{
  const params = new URLSearchParams(window.location.search);
  const screen = params.get('screen');
  if (screen === 'workspace' || screen === 'home') ui.screen = screen;
  const forcedTheme = params.get('theme');
  if (forcedTheme === 'light' || forcedTheme === 'dark') theme.set(forcedTheme);
}

const app = mount(App, {
  target: document.getElementById('app')!
});

export default app;
