// Shortcut bindings store (mandate §14 — shortcuts editor). Every editor
// command has a remappable combo; defaults follow platform conventions:
// Meta on macOS, Ctrl elsewhere (IS_MAC is detected once via navigator.platform).
//
// Conflict handling has three safety classes (W4.5 integration requirement
// #3) — classification is derived here, never stored:
//   (a) 'rimloc-conflict' — ERROR: the combo is already bound to another
//       RimLoc command. The editor REFUSES to save such a customization.
//   (b) 'system-reserved' — WARNING: the combo belongs to the OS
//       (Cmd+Q / Cmd+W / Cmd+Tab on macOS; the Ctrl+Q / Ctrl+W equivalents
//       elsewhere — Alt+Tab and Ctrl+Alt+Del cannot even reach a webview).
//       Saved, but the user is warned the OS will likely intercept it.
//   (c) 'convention' — INFO: the combo shadows a common app convention
//       (Cmd+S save, Cmd+Z undo, …) without matching this command's own
//       default. Saved, flagged so the choice is conscious.
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

/** Safety class of a binding issue (see module docstring). */
export type ShortcutIssueClass = 'error' | 'warning' | 'info';
/** What kind of hazard the binding hits. */
export type ShortcutIssueKind = 'rimloc-conflict' | 'system-reserved' | 'convention';

export interface ShortcutIssue {
  id: ShortcutId;
  cls: ShortcutIssueClass;
  kind: ShortcutIssueKind;
  /** Combo signature the issue is about. */
  sig: string;
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

/** Signature of a binding: `mod+<key>` or `<key>`. */
export function signatureOf(b: ShortcutBinding): string {
  return `${b.mod ? 'mod+' : ''}${b.key.toLowerCase()}`;
}

// OS-reserved modifier combos per platform (W4.5 #3, class b). On macOS the
// hard-reserved trio is Cmd+Q (quit), Cmd+W (close window), Cmd+Tab (app
// switch). Windows/Linux use the same roles via Alt+F4 / Ctrl+W / Alt+Tab —
// only the Ctrl+W and Ctrl+Q equivalents can be expressed with the single-
// modifier binding model (Alt-combos and Ctrl+Alt+Del never reach a webview
// as key events), so those two are classified.
const SYSTEM_RESERVED_MAC = new Set(['q', 'w', 'tab']);
const SYSTEM_RESERVED_OTHER = new Set(['q', 'w']);

function isSystemReserved(b: ShortcutBinding): boolean {
  if (!b.mod) return false;
  const key = b.key.toLowerCase();
  return IS_MAC ? SYSTEM_RESERVED_MAC.has(key) : SYSTEM_RESERVED_OTHER.has(key);
}

// Common application conventions (W4.5 #3, class c): overriding one of these
// with a different command is allowed but deserves an info-level flag.
const CONVENTION_MOD_KEYS = new Set(['s', 'z', 'f', 'c', 'v', 'x', 'a', 'p']);

/**
 * Classify a binding for a command against the CURRENT bindings of the
 * others (candidate = what would be in effect after assignment). Returns
 * null when the combo is safe. Priority: rimloc conflict > system-reserved
 * > convention. Pure function — the editor uses it both for live candidates
 * and (bound to the stored state) for badges on existing rows.
 */
export function classifyBinding(
  id: ShortcutId,
  binding: ShortcutBinding,
  allBindings: Record<ShortcutId, ShortcutBinding>
): ShortcutIssue | null {
  const sig = signatureOf(binding);
  const rimlocClash = SHORTCUT_DEFS.some((d) => d.id !== id && signatureOf(allBindings[d.id]) === sig);
  if (rimlocClash) return { id, cls: 'error', kind: 'rimloc-conflict', sig };
  if (isSystemReserved(binding)) return { id, cls: 'warning', kind: 'system-reserved', sig };
  const isConvention = binding.mod && CONVENTION_MOD_KEYS.has(binding.key.toLowerCase());
  if (isConvention) {
    const ownDefault = SHORTCUT_DEFS.find((d) => d.id === id)?.defaultBinding;
    // Matching your own default (e.g. Cmd+S on "Save edit") IS the
    // convention — only flag when a convention combo moved to another command.
    if (ownDefault && signatureOf(ownDefault) !== sig) {
      return { id, cls: 'info', kind: 'convention', sig };
    }
  }
  return null;
}

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
      const sig = signatureOf(b);
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
    return (this.conflictMap().get(signatureOf(b))?.length ?? 0) > 1;
  }

  /** Full safety classification of a command's CURRENT binding (W4.5 #3). */
  issueFor(id: ShortcutId): ShortcutIssue | null {
    return classifyBinding(id, this.bindings[id], this.bindings);
  }

  /** All current issues across commands, for the summary notes. */
  issues(): ShortcutIssue[] {
    return SHORTCUT_DEFS.map((d) => this.issueFor(d.id)).filter((i): i is ShortcutIssue => i !== null);
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
