// Shortcut bindings store (mandate §14 — shortcuts editor). Every editor
// command has a remappable combo; defaults follow platform conventions:
// Meta on macOS, Ctrl elsewhere (IS_MAC is detected once via navigator.platform).
//
// Conflict detection is derived, not stored: two commands bound to the same
// (mod, key) pair are highlighted by the editor, nothing is blocked silently.
// Bindings persist to localStorage like the rest of the GUI settings.

export type ShortcutId =
  | 'save'
  | 'undo'
  | 'search'
  | 'palette'
  | 'cancel'
  | 'nextUntranslated'
  | 'nextIssue'
  | 'prevIssue'
  | 'markReviewed';

export interface ShortcutBinding {
  /** KeyboardEvent.key, lowercased single character or named key. */
  key: string;
  /** true = platform modifier: Cmd on macOS, Ctrl elsewhere. */
  mod: boolean;
}

export const IS_MAC: boolean =
  typeof navigator !== 'undefined' && /mac/i.test(navigator.platform);

/** Human-readable single-key label (i18n-independent glyph names). */
export function keyLabel(key: string): string {
  const named: Record<string, string> = {
    escape: 'Esc',
    ' ': 'Space',
    arrowdown: '↓',
    arrowup: '↑',
    arrowleft: '←',
    arrowright: '→',
    pagedown: 'PgDn',
    pageup: 'PgUp',
    enter: 'Enter',
    tab: 'Tab'
  };
  return named[key.toLowerCase()] ?? key.toUpperCase();
}

/** Full combo label: "⌘S" on macOS, "Ctrl+S" elsewhere. */
export function comboLabel(b: ShortcutBinding): string {
  const mod = b.mod ? (IS_MAC ? '⌘' : 'Ctrl+') : '';
  return `${mod}${keyLabel(b.key)}`;
}

export interface ShortcutDef {
  id: ShortcutId;
  labelKey: string;
  defaultBinding: ShortcutBinding;
}

export const SHORTCUT_DEFS: ShortcutDef[] = [
  { id: 'save', labelKey: 'shortcuts.def.save', defaultBinding: { key: 's', mod: true } },
  { id: 'undo', labelKey: 'shortcuts.def.undo', defaultBinding: { key: 'z', mod: true } },
  { id: 'search', labelKey: 'shortcuts.def.search', defaultBinding: { key: 'f', mod: true } },
  { id: 'palette', labelKey: 'shortcuts.def.palette', defaultBinding: { key: 'k', mod: true } },
  { id: 'cancel', labelKey: 'shortcuts.def.cancel', defaultBinding: { key: 'escape', mod: false } },
  { id: 'nextUntranslated', labelKey: 'shortcuts.def.nextUntranslated', defaultBinding: { key: 'tab', mod: false } },
  { id: 'nextIssue', labelKey: 'shortcuts.def.nextIssue', defaultBinding: { key: 'n', mod: true } },
  { id: 'prevIssue', labelKey: 'shortcuts.def.prevIssue', defaultBinding: { key: 'p', mod: true } },
  { id: 'markReviewed', labelKey: 'shortcuts.def.markReviewed', defaultBinding: { key: 'm', mod: true } }
];

const STORAGE_KEY = 'rimloc.shortcuts';

function defaultBindings(): Record<ShortcutId, ShortcutBinding> {
  const out = {} as Record<ShortcutId, ShortcutBinding>;
  for (const d of SHORTCUT_DEFS) out[d.id] = { ...d.defaultBinding };
  return out;
}

function load(): Record<ShortcutId, ShortcutBinding> {
  const out = defaultBindings();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Record<ShortcutId, ShortcutBinding>>;
      for (const d of SHORTCUT_DEFS) {
        const b = parsed[d.id];
        if (b && typeof b.key === 'string') out[d.id] = { key: b.key.toLowerCase(), mod: Boolean(b.mod) };
      }
    }
  } catch {
    /* storage unavailable — defaults stay */
  }
  return out;
}

class ShortcutsStore {
  bindings = $state<Record<ShortcutId, ShortcutBinding>>(load());

  binding(id: ShortcutId): ShortcutBinding {
    return this.bindings[id];
  }

  /** Normalize an event into a binding (for the capture flow). */
  fromEvent(e: KeyboardEvent): ShortcutBinding | null {
    const key = e.key;
    if (!key || key === 'Shift' || key === 'Control' || key === 'Alt' || key === 'Meta') {
      return null; // bare modifier — keep waiting
    }
    let base = key.toLowerCase();
    if (base === 'esc') base = 'escape';
    return { key: base, mod: IS_MAC ? e.metaKey : e.ctrlKey };
  }

  assign(id: ShortcutId, binding: ShortcutBinding) {
    this.bindings[id] = binding;
    this.persist();
  }

  reset(id: ShortcutId) {
    const def = SHORTCUT_DEFS.find((d) => d.id === id);
    if (def) this.assign(id, { ...def.defaultBinding });
  }

  resetAll() {
    this.bindings = defaultBindings();
    this.persist();
  }

  /**
   * Conflict map: combo signature → ids bound to it. Signatures with more
   * than one id are duplicates the editor highlights.
   */
  conflictMap(): Map<string, ShortcutId[]> {
    const groups = new Map<string, ShortcutId[]>();
    for (const d of SHORTCUT_DEFS) {
      const b = this.bindings[d.id];
      const sig = `${b.mod ? 'mod+' : ''}${b.key.toLowerCase()}`;
      const arr = groups.get(sig) ?? [];
      arr.push(d.id);
      groups.set(sig, arr);
    }
    const out = new Map<string, ShortcutId[]>();
    for (const [sig, ids] of groups) {
      if (ids.length > 1) out.set(sig, ids);
    }
    return out;
  }

  isConflicting(id: ShortcutId): boolean {
    const b = this.bindings[id];
    const sig = `${b.mod ? 'mod+' : ''}${b.key.toLowerCase()}`;
    return (this.conflictMap().get(sig)?.length ?? 0) > 1;
  }

  private persist() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.bindings));
    } catch {
      /* storage unavailable */
    }
  }
}

export const shortcuts = new ShortcutsStore();
