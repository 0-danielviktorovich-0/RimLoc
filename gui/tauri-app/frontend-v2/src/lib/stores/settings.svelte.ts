// GUI-local settings store (mandate §15, spec §17). Everything here is
// device-local preference state persisted to localStorage; the backend never
// owns UI preferences. Appearance axes that already have dedicated stores
// (theme, style, palette, density) stay in theme.svelte.ts / stylelab.svelte.ts
// — this store covers the remaining task-grouped settings.
//
// Side effects are applied centrally in set(): motion writes a data-attribute
// (the Settings screen carries the matching :global rule), font size re-points
// the root typography tokens — components only ever read tokens, so this stays
// a token-level change, not a component override.

export type StartupMode = 'last-project' | 'home';
export type RimworldVersion = '1.4' | '1.5' | '1.6';
export type GlossaryMode = 'suggest' | 'enforce';
export type FontSizeId = 'small' | 'medium' | 'large';
export type MotionPref = 'system' | 'full' | 'reduced';
export type TargetLocaleId = 'ru' | 'de' | 'es' | 'fr' | 'pt-br' | 'ja' | 'zh-hans' | 'uk';
export type QualityMode = 'fast' | 'balanced' | 'max' | 'suggest';

export interface SettingsData {
  startup: StartupMode;
  checkUpdates: boolean;
  autoDetect: boolean;
  rimworldVersion: RimworldVersion;
  defaultTarget: TargetLocaleId;
  autosave: boolean;
  tmFirst: boolean;
  markAI: boolean;
  glossary: GlossaryMode;
  quality: QualityMode;
  fontSize: FontSizeId;
  softWrap: boolean;
  motion: MotionPref;
  sounds: boolean;
  systemNotifications: boolean;
}

const STORAGE_KEY = 'rimloc.settings';
const MOTION_STYLE_ID = 'rimloc-motion-style';

const DEFAULTS: SettingsData = {
  startup: 'last-project',
  checkUpdates: true,
  autoDetect: true,
  rimworldVersion: '1.6',
  defaultTarget: 'ru',
  autosave: true,
  tmFirst: true,
  markAI: true,
  glossary: 'suggest',
  quality: 'balanced',
  fontSize: 'medium',
  softWrap: true,
  motion: 'system',
  sounds: false,
  systemNotifications: false
};

function load(): SettingsData {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<SettingsData>) };
  } catch {
    /* storage unavailable */
  }
  return { ...DEFAULTS };
}

/** Root typography token overrides per font-size choice (base, dense). */
const FONT_SCALE: Record<FontSizeId, [string, string]> = {
  small: ['13px', '12px'],
  medium: ['14px', '13px'],
  large: ['16px', '15px']
};

class SettingsStore {
  data = $state<SettingsData>(load());

  constructor() {
    this.applyMotion();
    this.applyFont();
  }

  /** Single write path: assign, persist, then apply the one known side effect. */
  set<K extends keyof SettingsData>(key: K, value: SettingsData[K]) {
    this.data[key] = value;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.data));
    } catch {
      /* storage unavailable */
    }
    if (key === 'motion') this.applyMotion();
    if (key === 'fontSize') this.applyFont();
  }

  private applyMotion() {
    document.documentElement.dataset.motion = this.data.motion;
    // The reduced-motion override must outlive the Settings screen, so the
    // rule is injected once into <head> and toggled by the data-attribute
    // (complements the prefers-reduced-motion media rule in app.css).
    if (!document.getElementById(MOTION_STYLE_ID)) {
      const el = document.createElement('style');
      el.id = MOTION_STYLE_ID;
      el.textContent =
        "html[data-motion='reduced'] *, html[data-motion='reduced'] *::before," +
        ' html[data-motion=\'reduced\'] *::after { transition-duration: 1ms !important;' +
        ' animation-duration: 1ms !important; transform: none !important; }';
      document.head.appendChild(el);
    }
  }

  private applyFont() {
    const [base, dense] = FONT_SCALE[this.data.fontSize];
    const el = document.documentElement.style;
    el.setProperty('--text-base-size', base);
    el.setProperty('--text-dense-size', dense);
  }
}

export const settings = new SettingsStore();
