// Source store (W7): UI state for the SOURCE tab, read-only viewer, advanced
// browser, compare, context menu and the external-change banner. All source
// FACTS come from the fixture registry (fixtures.ts) — the store only
// resolves and transitions; it computes no precedence and reads no files.
// The clipboard helper reports success only after a fulfilled write and
// exposes a selectable fallback otherwise (same contract as W4.5 export).
import { DEFAULT_SCENARIO_ID, SOURCE_SCENARIOS } from './fixtures';
import {
  buildLaunchPlan,
  templateFor,
  type EditorTarget,
  type StoredEditorChoice
} from './editor';
import type {
  CompareTriple,
  MockActionFeedback,
  SourceEntryData,
  SourceFile,
  SourceScenario,
  SourceUsage
} from './types';

export { SOURCE_SCENARIOS };
export type { StoredEditorChoice };

/** Viewer target (lead review 029 #1): an entry+usage pair OR an explicit
 *  fixture file. The explicit form is what generated / shadowed / unmapped /
 *  missing files use — the viewer renders THAT file, never a first-entry
 *  fallback of unrelated content. */
export type ViewerTarget =
  | { kind: 'entry'; entryId: string; usageIndex: number }
  | { kind: 'file'; path: string };

type MenuState = { entryId: string; x: number; y: number } | null;

class SourceStore {
  /** Active demo scenario (stable ids, SOURCE_INSPECTOR §18). */
  scenarioId = $state(DEFAULT_SCENARIO_ID);
  /** Which entry's context menu is open and where. */
  menu = $state<MenuState>(null);
  /** Open read-only viewer target. */
  viewer = $state<ViewerTarget | null>(null);
  /** Advanced source browser modal. */
  browser = $state(false);
  /** Compare modal for an entry with a fixture triple. */
  compare = $state<string | null>(null);
  /** External-change banner state (mock transitions; no watcher). */
  changeState = $state<'hidden' | 'visible' | 'dismissed'>('hidden');
  /** Honest demo feedback for OS actions ("would open", never "opened"). */
  lastMockAction = $state<MockActionFeedback | null>(null);
  /** Clipboard fallback: text that could not be written, shown selectable. */
  clipboardFallback = $state<{ id: string; labelKey: string; text: string } | null>(null);

  private mockSeq = 0;
  private changeInitialized = $state(false);

  get scenario(): SourceScenario {
    return SOURCE_SCENARIOS.find((s) => s.id === this.scenarioId) ?? SOURCE_SCENARIOS[0];
  }

  setScenario(id: string) {
    if (!SOURCE_SCENARIOS.some((s) => s.id === id)) return;
    this.scenarioId = id;
    // Lead review 029 #1: a scenario switch invalidates stale overlay/action
    // state — the old viewer file, compare triple, clipboard fallback, menu
    // and toast all belong to the previous demo dataset.
    this.viewer = null;
    this.compare = null;
    this.clipboardFallback = null;
    this.menu = null;
    this.lastMockAction = null;
    this.changeInitialized = true;
    this.changeState = this.scenario.externalChange ? 'visible' : 'hidden';
  }

  /** Source facts for an entry under the ACTIVE scenario, or null when the
   *  demo scenario does not cover it (honest empty state — no fake data). */
  contextFor(entryId: string): SourceEntryData | null {
    return this.scenario.entries[entryId] ?? null;
  }

  /** Ids the active scenario can demonstrate (for the honest empty state). */
  coveredEntryIds(): string[] {
    return Object.keys(this.scenario.entries);
  }

  fileFor(path: string): SourceFile {
    return (
      this.scenario.files[path] ?? {
        path,
        displayPath: path,
        kind: 'effective',
        missing: true,
        reasonKey: 'source.missing.reason'
      }
    );
  }

  // ------------------------------------------------------------- transitions
  openMenu(entryId: string, x: number, y: number) {
    this.menu = { entryId, x, y };
  }

  closeMenu() {
    this.menu = null;
  }

  openViewer(entryId: string, usageIndex = 0) {
    this.menu = null;
    this.viewer = { kind: 'entry', entryId, usageIndex };
  }

  /** Viewer for an EXPLICIT fixture file — generated, shadowed, unmapped or
   *  missing. The requested path is never swapped for another entry's file. */
  openFile(path: string) {
    this.menu = null;
    this.viewer = { kind: 'file', path };
  }

  closeViewer() {
    this.viewer = null;
  }

  openBrowser() {
    this.menu = null;
    this.browser = true;
  }

  closeBrowser() {
    this.browser = false;
  }

  openCompare(entryId: string) {
    this.menu = null;
    if (this.scenario.compare[entryId]) this.compare = entryId;
  }

  closeCompare() {
    this.compare = null;
  }

  compareFor(entryId: string): CompareTriple | null {
    return this.scenario.compare[entryId] ?? null;
  }

  /** Entry id mapped to a highlighted node (node→entry navigation). */
  entryForNode(path: string, nodeId: string): string | null {
    const f = this.scenario.files[path];
    if (!f || 'missing' in f) return null;
    const span = f.spans.find((s) => s.nodeId === nodeId);
    return span?.entryId ?? null;
  }

  /** External-change banner — mock transitions only (§12, no watcher). */
  ensureChangeBanner() {
    if (!this.changeInitialized) {
      this.changeInitialized = true;
      this.changeState = this.scenario.externalChange ? 'visible' : 'hidden';
    }
  }

  changeShow() {
    const ec = this.scenario.externalChange;
    if (ec) this.openFile(ec.file);
  }

  changeRescan() {
    // Mock rescan: the demo change is "accepted" — banner clears.
    this.changeState = 'hidden';
    this.noteMockAction('source.changed.rescanDone');
  }

  changeIgnore() {
    this.changeState = 'dismissed';
  }

  changeRearm() {
    this.changeInitialized = true;
    this.changeState = this.scenario.externalChange ? 'visible' : 'hidden';
  }

  // ------------------------------------------------------------ mock honesty
  noteMockAction(labelKey: string, params?: Record<string, string | number>) {
    this.mockSeq += 1;
    this.lastMockAction = { id: this.mockSeq, labelKey, params };
  }

  // ---------------------------------------------------------------- clipboard
  async copyText(id: string, labelKey: string, text: string): Promise<'copied' | 'fallback'> {
    const clipboard = navigator.clipboard;
    if (!clipboard?.writeText) {
      this.clipboardFallback = { id, labelKey, text };
      return 'fallback';
    }
    try {
      await clipboard.writeText(text);
      this.clipboardFallback = null;
      return 'copied';
    } catch {
      this.clipboardFallback = { id, labelKey, text };
      return 'fallback';
    }
  }

  clearClipboardFallback() {
    this.clipboardFallback = null;
  }

  // ------------------------------------------------------------ editor (mock)
  /** Build + validate the structured launch plan for a location (§8).
   *  Accepts the minimal location shape so any usage OR explicit file path
   *  can be planned; returns the argv preview or a typed rejection. */
  planEditorLaunch(
    choice: StoredEditorChoice,
    loc: { displayPath: string; line: number | null; column: number | null }
  ): { ok: true; argv: string[] } | { ok: false; reasonKey: string } {
    const target: EditorTarget = {
      path: loc.displayPath,
      line: loc.line,
      column: loc.column
    };
    const result = buildLaunchPlan(templateFor(choice), target);
    if (!result.ok) return result;
    return { ok: true, argv: [result.plan.executable, ...result.plan.args] };
  }
}

export const source = new SourceStore();
