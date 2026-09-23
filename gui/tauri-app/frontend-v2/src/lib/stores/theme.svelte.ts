// Theme store: triad light/dark/system per GUI_DESIGN_SPEC §1.1.
// Choice lives in GUI-local settings (localStorage), never in the backend.
export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'rimloc.theme';

function readInitial(): ThemeMode {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'light' || v === 'dark' || v === 'system') return v;
  } catch {
    /* storage unavailable */
  }
  return 'dark'; // spec: dark is the default reference theme for a desktop tool
}

class ThemeStore {
  mode = $state<ThemeMode>(readInitial());

  constructor() {
    if (typeof window !== 'undefined' && window.matchMedia) {
      // Keep 'system' in sync with the OS preference while it is selected.
      window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
        if (this.mode === 'system') this.apply();
      });
    }
    this.apply();
  }

  set(mode: ThemeMode) {
    this.mode = mode;
    try {
      localStorage.setItem(STORAGE_KEY, mode);
    } catch {
      /* storage unavailable */
    }
    this.apply();
  }

  private apply() {
    const resolved =
      this.mode === 'system'
        ? window.matchMedia('(prefers-color-scheme: light)').matches
          ? 'light'
          : 'dark'
        : this.mode;
    document.documentElement.dataset.theme = resolved;
  }
}

export const theme = new ThemeStore();
