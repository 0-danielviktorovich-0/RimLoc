// Runtime dispatcher for the three W7 source shortcut commands (lead review
// 029 #3). The W4.5 registry stays remapping-only; THIS module is the single
// runtime consumer for the source commands: a non-empty binding triggers the
// same action the palette runs; unbound bindings stay inert. Guards:
//  - never fires while typing in an editable target (input/textarea/select/
//    contentEditable) — editing must keep its keys;
//  - never fires while the command palette is open;
//  - entry-anchored actions require a selected workspace entry and report
//    the same honest "nothing selected" note as the palette (availability
//    kept in sync with palette actions).
// Single install, idempotent — safe under HMR and repeated mounts.
import { shortcuts } from '../stores/shortcuts.svelte';
import { palette } from '../stores/palette.svelte';
import { router, PROJECT_ROUTES } from '../router.svelte';
import { project } from '../stores/project.svelte';
import { source } from './store.svelte';

export type SourceCommandId = 'sourceOpen' | 'sourceBrowser' | 'sourceCompare';

function runSourceCommand(id: SourceCommandId): boolean {
  if (id === 'sourceBrowser') {
    if (!(PROJECT_ROUTES as string[]).includes(router.route)) router.navigate('workspace');
    source.openBrowser();
    return true;
  }
  const selected = project.selected;
  if (!selected) {
    source.noteMockAction('source.palette.noSelection');
    return true; // handled: honestly reported, not silently dropped
  }
  if (!(PROJECT_ROUTES as string[]).includes(router.route)) router.navigate('workspace');
  if (id === 'sourceOpen') source.openViewer(selected.id, 0);
  else source.openCompare(selected.id);
  return true;
}

function inEditableTarget(e: Event): boolean {
  const t = e.target as HTMLElement | null;
  if (!t) return false;
  if (t.isContentEditable) return true;
  const tag = t.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

function matchSourceCommand(e: KeyboardEvent): SourceCommandId | null {
  // bare modifiers/keys never match: a binding needs a real key
  const ids: SourceCommandId[] = ['sourceOpen', 'sourceBrowser', 'sourceCompare'];
  for (const id of ids) {
    const b = shortcuts.bindings[id];
    if (b.key === '') continue; // unbound: inert by design
    if (e.key.toLowerCase() !== b.key) continue;
    if (b.mod && !(e.metaKey || e.ctrlKey)) continue;
    if (!b.mod && (e.metaKey || e.ctrlKey || e.altKey)) continue;
    return id;
  }
  return null;
}

let installed = false;

/** Install the global keydown listener once. Returns an uninstall (tests). */
export function installSourceShortcutDispatcher(): () => void {
  if (installed) return () => {};
  installed = true;
  const handler = (e: KeyboardEvent) => {
    if (palette.open) return; // palette handles its own keys while open
    if (inEditableTarget(e)) return; // editing guard
    const id = matchSourceCommand(e);
    if (!id) return;
    e.preventDefault();
    runSourceCommand(id);
  };
  window.addEventListener('keydown', handler);
  return () => {
    window.removeEventListener('keydown', handler);
    installed = false;
  };
}
