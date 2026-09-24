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
  SourceBrowserData,
  SourceEntryData,
  SourceFile,
  SourceScenario,
  SourceUsage
} from './types';

export { SOURCE_SCENARIOS };
export type { StoredEditorChoice };

type ViewerState = { entryId: string; usageIndex: number } | null;
type MenuState = { entryId: string; x: number; y: number } | null;

class SourceStore {
  /** Active demo scenario (stable ids, SOURCE_INSPECTOR §18). */
  scenarioId = $state(DEFAULT_SCENARIO_ID);
  /** Which entry's context menu is open and where. */
  menu = $state<MenuState>(null);
  /** Open read-only viewer (entry + usage). */
  viewer = $state<ViewerState>(null);
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
    if (SOURCE_SCENARIOS.some((s) => s.id === id)) {
      this.scenarioId = id;
      this.menu = null;
      // The banner is a property of the scenario; reset transitions on switch.
      this.changeState = this.scenario.externalChange ? 'visible' : 'hidden';
    }
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
    return this.scenario.files[path] ?? { path, displayPath: path, kind: 'effective', missing: true, reasonKey: 'source.missing.reason' };
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
    this.viewer = { entryId, usageIndex };
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
    if (ec) this.openFileViewer(ec.file);
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
    this.changeState = this.scenario.externalChange ? 'visible' : 'hidden';
  }

  /** Viewer directly for a file path (browser rows, changed file). */
  openFileViewer(path: string, usageIndex = -1) {
    const data = Object.entries(this.scenario.entries).find(([, d]) =>
      d.usages.some((u) => u.location.path === path)
    );
    if (data) {
      const idx = data[1].usages.findIndex((u) => u.location.path === path);
      this.viewer = { entryId: data[0], usageIndex: idx === -1 ? 0 : idx };
      return;
    }
    // No entry mapped (pure file view): fall back to the first entry of the
    // scenario so the viewer has an anchor; the file itself drives content.
    const first = Object.keys(this.scenario.entries)[0];
    if (first) this.viewer = { entryId: first, usageIndex };
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
  /** Build + validate the structured launch plan for a usage (§8). Returns
   *  either the argv preview or a typed rejection; NOTHING is launched. */
  planEditorLaunch(
    choice: StoredEditorChoice,
    usage: { location: { displayPath: string; line: number | null; column: number | null } }
  ): { ok: true; argv: string[] } | { ok: false; reasonKey: string } {
    const target: EditorTarget = {
      path: usage.location.displayPath,
      line: usage.location.line,
      column: usage.location.column
    };
    const result = buildLaunchPlan(templateFor(choice), target);
    if (!result.ok) return result;
    return {
      ok: true,
      argv: [result.plan.executable, ...result.plan.args]
    };
  }
}

export const source = new SourceStore();
