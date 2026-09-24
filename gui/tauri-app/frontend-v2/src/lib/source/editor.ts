// External editor launch templates (SOURCE_INSPECTOR_MANDATE §8). The launch
// plan is STRUCTURAL — executable + argument array with explicit
// substitutions — never a shell string. Paths, spaces and Unicode are
// literal array items; malformed or unknown substitutions are rejected
// before anything is stored. The demo "test" action only previews the argv
// array; nothing is launched (mock/pre-freeze).
//
// Future seam: this module is the client half of a framework-neutral
// `client.editors.plan(target)` call — the real backend will execute the
// plan; the GUI only builds and validates it.

export type EditorId = 'system' | 'zed' | 'vscode' | 'custom';

export interface EditorPreset {
  id: EditorId;
  labelKey: string;
  executable: string;
  argsTemplate: string[];
  /** Whether the preset pretends to be locally detectable in the demo. */
  detectedInDemo: boolean;
}

export const EDITOR_PRESETS: EditorPreset[] = [
  {
    id: 'system',
    labelKey: 'source.editor.preset.system',
    executable: '<system-default>',
    argsTemplate: ['{path}'],
    detectedInDemo: true
  },
  {
    id: 'zed',
    labelKey: 'source.editor.preset.zed',
    executable: 'zed',
    argsTemplate: ['{path}', '{line}'],
    detectedInDemo: true
  },
  {
    id: 'vscode',
    labelKey: 'source.editor.preset.vscode',
    executable: 'code',
    argsTemplate: ['--goto', '{path}:{line}:{column}'],
    detectedInDemo: true
  },
  {
    id: 'custom',
    labelKey: 'source.editor.preset.custom',
    executable: '',
    argsTemplate: [],
    detectedInDemo: false
  }
];

export interface EditorTarget {
  path: string;
  line: number | null;
  column: number | null;
}

export interface CustomEditorConfig {
  executable: string;
  /** Raw arg string as typed by the user (quotes allowed for grouping). */
  argsText: string;
}

export interface EditorLaunchPlan {
  executable: string;
  args: string[];
}

export type EditorLaunchPlanResult =
  | { ok: true; plan: EditorLaunchPlan }
  | { ok: false; reasonKey: string };

const PLACEHOLDER = /^\{(path|line|column)\}$/;

/** Result of the strict arg-text split: items, or an unbalanced-quote
 *  rejection (lead review 029 #2). Quotes always group ONE argument;
 *  spaces and Unicode stay literal inside items. */
export type SplitArgsResult = { ok: true; items: string[] } | { ok: false; reasonKey: string };

export function splitArgsTextStrict(text: string): SplitArgsResult {
  const out: string[] = [];
  let cur = '';
  let quoted = false;
  let hasContent = false;
  for (const ch of text) {
    if (ch === '"') {
      quoted = !quoted;
      hasContent = true;
      continue;
    }
    if (!quoted && /\s/.test(ch)) {
      if (hasContent) out.push(cur);
      cur = '';
      hasContent = false;
      continue;
    }
    cur += ch;
    hasContent = true;
  }
  if (quoted) return { ok: false, reasonKey: 'source.editor.error.unbalancedQuote' };
  if (hasContent) out.push(cur);
  return { ok: true, items: out };
}

/** Lenient split kept for live template preview (ignores unbalanced quotes). */
export function splitArgsText(text: string): string[] {
  const r = splitArgsTextStrict(text);
  return r.ok ? r.items : [];
}

/** A positive, safe integer — the only kind line/column may take. */
function isPositiveInt(v: number | null | undefined): v is number {
  return typeof v === 'number' && Number.isSafeInteger(v) && v > 0;
}

/** Resolve ONE placeholder token to its literal replacement. */
function resolvePlaceholder(name: string, target: EditorTarget): string {
  if (name === 'path') return target.path;
  if (name === 'line') {
    if (!isPositiveInt(target.line)) throw new PlaceholderError('source.editor.error.noLine');
    return String(target.line);
  }
  // column
  if (!isPositiveInt(target.column)) throw new PlaceholderError('source.editor.error.noColumn');
  return String(target.column);
}

class PlaceholderError extends Error {
  constructor(public reasonKey: string) {
    super(reasonKey);
  }
}

/**
 * Substitute {path}/{line}/{column} in ONE pass with a callback: the
 * replacement values are inserted LITERALLY and are never re-scanned, so
 * "$&"/"${}"/braces inside a resolved path stay exactly as they are, and
 * repeated tokens all resolve. Unknown or malformed placeholders are
 * rejected up front; line/column must be positive integers when required.
 * Pure — no side effects, no shell, nothing launches.
 */
export function buildLaunchPlan(
  template: { executable: string; argsTemplate: string[] },
  target: EditorTarget
): EditorLaunchPlanResult {
  const executable = template.executable.trim();
  if (!executable || executable === '<system-default>') {
    return { ok: false, reasonKey: 'source.editor.error.noExecutable' };
  }
  if (!target.path.trim()) {
    return { ok: false, reasonKey: 'source.editor.error.noPath' };
  }
  const TOKEN = /\{([a-zA-Z0-9_]*)\}/g;
  // ANY brace group is treated as a placeholder attempt — empty or unknown
  // names ({}, {bogus_1}, {LINE }) are rejected, not silently passed through.
  const ANY_BRACE = /\{([^{}]*)\}/g;
  const args: string[] = [];
  try {
    for (const raw of template.argsTemplate) {
      // validate every token in this argument before touching the string
      for (const m of raw.matchAll(ANY_BRACE)) {
        const name = m[1];
        if (name !== 'path' && name !== 'line' && name !== 'column') {
          return { ok: false, reasonKey: 'source.editor.error.unknownPlaceholder' };
        }
        if (name === 'line' || name === 'column') {
          const v = name === 'line' ? target.line : target.column;
          if (!isPositiveInt(v)) return { ok: false, reasonKey: name === 'line' ? 'source.editor.error.noLine' : 'source.editor.error.noColumn' };
        }
      }
      // single pass: callback results are literal by specification
      args.push(raw.replace(TOKEN, (_m, name: string) => resolvePlaceholder(name, target)));
    }
  } catch (e) {
    if (e instanceof PlaceholderError) return { ok: false, reasonKey: e.reasonKey };
    throw e;
  }
  return { ok: true, plan: { executable, args } };
}

// ------------------------------------------------------------- persistence
const STORAGE_KEY = 'rimloc.source.editor';

export interface StoredEditorChoice {
  preset: EditorId;
  custom: CustomEditorConfig;
}

export function loadEditorChoice(): StoredEditorChoice {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<StoredEditorChoice> | null;
      // Lead review 029 #2: corrupt localStorage must never break Settings —
      // every field is type-checked and unknown presets fall back to defaults.
      if (
        parsed &&
        typeof parsed === 'object' &&
        typeof parsed.preset === 'string' &&
        EDITOR_PRESETS.some((p) => p.id === parsed.preset) &&
        typeof parsed.custom === 'object' &&
        parsed.custom !== null &&
        typeof parsed.custom.executable === 'string' &&
        typeof parsed.custom.argsText === 'string'
      ) {
        return { preset: parsed.preset, custom: parsed.custom };
      }
    }
  } catch {
    // fall through to defaults
  }
  return { preset: 'system', custom: { executable: '', argsText: '{path}' } };
}

export function saveEditorChoice(choice: StoredEditorChoice): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choice));
  } catch {
    // persistence is best-effort in the demo
  }
}

/** Effective template for the chosen preset (custom uses stored config).
 *  Result-shaped: an unbalanced quote in the custom template is a typed
 *  rejection, never a silently mis-split argument. */
export function templateFor(choice: StoredEditorChoice): EditorLaunchPlanResult {
  if (choice.preset === 'custom') {
    const executable = choice.custom.executable.trim();
    if (!executable) return { ok: false, reasonKey: 'source.editor.error.noExecutable' };
    const split = splitArgsTextStrict(choice.custom.argsText);
    if (!split.ok) return split;
    return { ok: true, plan: { executable, args: split.items } };
  }
  const preset = EDITOR_PRESETS.find((p) => p.id === choice.preset) ?? EDITOR_PRESETS[0];
  return { ok: true, plan: { executable: preset.executable, args: preset.argsTemplate } };
}
