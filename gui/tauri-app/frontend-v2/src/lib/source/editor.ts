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

/** Split the typed args text into array items: whitespace separates,
 *  double quotes group one item (spaces/Unicode stay literal inside). */
export function splitArgsText(text: string): string[] {
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
  if (hasContent) out.push(cur);
  return out;
}

/** Substitute {path}/{line}/{column} literally; reject malformed/unknown
 *  placeholders and unusable targets. Pure — no side effects, no shell. */
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
  const args: string[] = [];
  for (const raw of template.argsTemplate) {
    const m = PLACEHOLDER.exec(raw);
    if (m) {
      const name = m[1];
      if (name === 'path') {
        args.push(target.path);
      } else if (name === 'line') {
        if (target.line === null) return { ok: false, reasonKey: 'source.editor.error.noLine' };
        args.push(String(target.line));
      } else {
        if (target.column === null || target.line === null) {
          return { ok: false, reasonKey: 'source.editor.error.noColumn' };
        }
        args.push(`${target.line}:${target.column}`);
      }
      continue;
    }
    // Any embedded placeholder inside a token that is not exactly one —
    // e.g. "{path}:{line}" — is handled here: only known placeholders may
    // appear, unknown names are rejected.
    const embedded = raw.match(/\{[a-zA-Z]+\}/g);
    if (embedded) {
      for (const ph of embedded) {
        if (!PLACEHOLDER.test(ph)) {
          return { ok: false, reasonKey: 'source.editor.error.unknownPlaceholder' };
        }
      }
      let value = raw;
      value = value.replace('{path}', target.path);
      if (value.includes('{line}')) {
        if (target.line === null) return { ok: false, reasonKey: 'source.editor.error.noLine' };
        value = value.replace('{line}', String(target.line));
      }
      if (value.includes('{column}')) {
        if (target.column === null || target.line === null) {
          return { ok: false, reasonKey: 'source.editor.error.noColumn' };
        }
        value = value.replace('{column}', String(target.column));
      }
      args.push(value);
      continue;
    }
    args.push(raw); // literal token: spaces/Unicode stay as typed
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
      const parsed = JSON.parse(raw) as StoredEditorChoice;
      if (EDITOR_PRESETS.some((p) => p.id === parsed.preset)) return parsed;
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

/** Effective template for the chosen preset (custom uses stored config). */
export function templateFor(choice: StoredEditorChoice): { executable: string; argsTemplate: string[] } {
  if (choice.preset === 'custom') {
    return {
      executable: choice.custom.executable,
      argsTemplate: splitArgsText(choice.custom.argsText)
    };
  }
  const preset = EDITOR_PRESETS.find((p) => p.id === choice.preset) ?? EDITOR_PRESETS[0];
  return { executable: preset.executable, argsTemplate: preset.argsTemplate };
}
