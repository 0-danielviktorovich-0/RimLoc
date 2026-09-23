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

const app = mount(App, {
  target: document.getElementById('app')!
});

export default app;
