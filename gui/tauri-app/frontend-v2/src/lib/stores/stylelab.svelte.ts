// Style Lab store — DEVELOPMENT-ONLY visual-direction switcher.
// Axes are applied as attributes on <html>: data-style / data-palette /
// data-density (data-theme is owned by theme.svelte.ts).
//
// Gating (must stay dev-only): the PANEL is visible only when
//   ?stylelab=1 in the URL (session-only, not persisted), OR
//   localStorage['rimloc.stylelab'] === '1'.
// Without the flag nothing is shown; attribute application is harmless in
// production because persisted values default to precision/indigo/comfortable,
// which equals the shipped baseline.
//
// URL params (?style=…&palette=…&density=…) override for the session without
// persisting — they exist for deterministic screenshots.

export type StyleId = 'precision' | 'aurora' | 'workshop' | 'editorial';
export type PaletteId = 'indigo' | 'blue' | 'emerald' | 'amber';
export type DensityId = 'comfortable' | 'compact';

export const STYLE_IDS: StyleId[] = ['precision', 'aurora', 'workshop', 'editorial'];
export const PALETTE_IDS: PaletteId[] = ['indigo', 'blue', 'emerald', 'amber'];
export const DENSITY_IDS: DensityId[] = ['comfortable', 'compact'];

const KEY_LAB = 'rimloc.stylelab';
const KEY_STYLE = 'rimloc.style.style';
const KEY_PALETTE = 'rimloc.style.palette';
const KEY_DENSITY = 'rimloc.style.density';

function readStorage(key: string, allowed: readonly string[], fallback: string): string {
  try {
    const v = localStorage.getItem(key);
    if (v && (allowed as readonly string[]).includes(v)) return v;
  } catch {
    /* storage unavailable */
  }
  return fallback;
}

function fromUrl(key: string, allowed: readonly string[]): string | null {
  const v = new URLSearchParams(window.location.search).get(key);
  return v && (allowed as readonly string[]).includes(v) ? v : null;
}

function store(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* storage unavailable */
  }
}

class StyleLabStore {
  /** Panel visibility — never true in production unless explicitly opted in. */
  enabled = $state<boolean>(
    fromUrl('stylelab', ['1', '0']) === '1' || readStorage(KEY_LAB, ['1'], '0') === '1'
  );

  /** True only when the lab is persisted (localStorage), not URL-flagged. */
  remember = $state<boolean>(readStorage(KEY_LAB, ['1'], '0') === '1');

  style = $state<StyleId>(
    (fromUrl('style', STYLE_IDS) ?? readStorage(KEY_STYLE, STYLE_IDS, 'precision')) as StyleId
  );
  palette = $state<PaletteId>(
    (fromUrl('palette', PALETTE_IDS) ?? readStorage(KEY_PALETTE, PALETTE_IDS, 'indigo')) as PaletteId
  );
  density = $state<DensityId>(
    (fromUrl('density', DENSITY_IDS) ?? readStorage(KEY_DENSITY, DENSITY_IDS, 'comfortable')) as DensityId
  );

  constructor() {
    this.apply();
  }

  setStyle(v: StyleId) {
    this.style = v;
    if (!fromUrl('style', STYLE_IDS)) store(KEY_STYLE, v);
    this.apply();
  }

  setPalette(v: PaletteId) {
    this.palette = v;
    if (!fromUrl('palette', PALETTE_IDS)) store(KEY_PALETTE, v);
    this.apply();
  }

  setDensity(v: DensityId) {
    this.density = v;
    if (!fromUrl('density', DENSITY_IDS)) store(KEY_DENSITY, v);
    this.apply();
  }

  /** Persist the lab itself. Turning it on keeps the panel across sessions. */
  setRemember(on: boolean) {
    store(KEY_LAB, on ? '1' : '0');
    this.remember = on;
    this.enabled = on || fromUrl('stylelab', ['1', '0']) === '1';
  }

  private apply() {
    const el = document.documentElement;
    el.dataset.style = this.style;
    el.dataset.palette = this.palette;
    el.dataset.density = this.density;
  }
}

export const stylelab = new StyleLabStore();
