// Command palette state (mandate §17). The open/closed flag only; the global
// hotkey and the portal mount live in palette-boot.svelte.ts so this store
// stays free of component imports.
class PaletteStore {
  open = $state(false);

  toggle() {
    this.open = !this.open;
  }

  show() {
    this.open = true;
  }

  hide() {
    this.open = false;
  }
}

export const palette = new PaletteStore();
