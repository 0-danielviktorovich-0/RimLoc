// Command palette bootstrap (mandate §17): global Cmd/Ctrl+K hotkey plus a
// self-mounted portal so the palette is alive on every route.
//
// Why a boot module instead of a tag in App.svelte: App.svelte statically
// imports Phase2Stub.svelte, which imports THIS module as a side effect —
// so the palette initializes at app start without owning any shared file.
// The portal <div> is appended to <body> (outside #app), position: fixed,
// above header and dev panel; Svelte 5's mount() attaches the component.
import { mount } from 'svelte';
import CommandPalette from './components/CommandPalette.svelte';
import { palette } from './stores/palette.svelte';

if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      palette.toggle();
    }
  });

  const host = document.createElement('div');
  host.dataset.paletteHost = '';
  document.body.appendChild(host);
  mount(CommandPalette, { target: host });
}
