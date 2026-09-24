// W7 SOURCE INSPECTOR regressions (SOURCE_INSPECTOR_MANDATE §18 scenarios,
// §1-§14 behaviors). Mock/pre-freeze: everything below exercises the typed
// fixture seam — no backend, no FS, no real editor launches.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { click, exists, mountCmp, q } from './helpers';
import { mockEntries } from '../src/lib/mock/data';
import { project } from '../src/lib/stores/project.svelte';
import { source } from '../src/lib/source/store.svelte';
import {
  buildLaunchPlan,
  loadEditorChoice,
  splitArgsText,
  splitArgsTextStrict,
  templateFor
} from '../src/lib/source/editor';
import { shortcuts } from '../src/lib/stores/shortcuts.svelte';
import { installSourceShortcutDispatcher } from '../src/lib/source/dispatch.svelte';
import { i18n } from '../src/i18n/store.svelte';
import DetailPanel from '../src/lib/components/workspace/DetailPanel.svelte';
import SourceOverlays from '../src/lib/components/source/SourceOverlays.svelte';
import SourceTab from '../src/lib/components/source/SourceTab.svelte';
import SourceChangeBanner from '../src/lib/components/source/SourceChangeBanner.svelte';

function reset() {
  i18n.locale = 'en'; // CI assertions pin English accessible names (spec §5)
  project.select(null);
  source.setScenario('source/simple');
  source.closeViewer();
  source.closeBrowser();
  source.closeMenu();
  source.closeCompare();
  source.clearClipboardFallback();
  source.changeRearm();
}

function entryById(id: string) {
  const e = mockEntries.find((m) => m.id === id);
  if (!e) throw new Error(`fixture entry missing: ${id}`);
  return e;
}

describe('w7 #1: SOURCE tab renders fixture facts', () => {
  beforeEach(reset);

  it('keyed-01 shows effective location, honest nullable column and provenance facts', () => {
    project.select('keyed-01');
    mountCmp(DetailPanel, { entry: entryById('keyed-01') });
    click('tabs.source');
    flushSync();
    expect(exists('source.tab')).toBe(true);
    expect(q('source.tab.path').textContent).toContain('Misc_Gameplay.xml');
    expect(q('source.tab.lineCol').textContent).toContain('12');
    expect(q('source.tab.lineCol').textContent).toContain('—'); // column not guaranteed
    expect(q('source.tab.why').textContent).toContain('Keyed');
    expect(q('source.tab.why').textContent).toContain('last file');
  });

  it('entry without fixture data shows the honest empty state with covered ids', () => {
    mountCmp(SourceTab, { entry: { id: 'keyed-03', kind: 'Keyed', key: 'ResearchTabTitle' } });
    flushSync();
    expect(exists('source.tab.empty')).toBe(true);
    expect(q('source.tab.empty').textContent).toContain('keyed-03');
  });
});

describe('w7 #2: TKey multi-context primary + other usages', () => {
  beforeEach(reset);

  it('tk-01 lists primary + one other usage and the viewer opens the chosen node', () => {
    source.setScenario('source/tkey-multi-context');
    project.select('tk-01');
    mountCmp(SourceTab, { entry: entryById('tk-01') });
    mountCmp(SourceOverlays); // overlays host the viewer
    expect(exists('source.tab')).toBe(true);
    expect(q('source.tab').textContent).toContain('Other usages (1)');
    // open viewer on the OTHER usage (index 1)
    click('source.tab.other.0');
    flushSync();
    expect(exists('source.viewer')).toBe(true);
    // current node highlight is line 18 for the other usage
    expect(q('source.viewer.code').textContent).toContain('<li>Your three colonists');
  });

  it('viewer node→entry navigation selects the mapped entry', () => {
    source.setScenario('source/generated-output');
    project.select('di-02');
    source.openViewer('di-02', 0);
    mountCmp(SourceOverlays);
    const node = q('source.viewer.node.di-02');
    node.click();
    flushSync();
    expect(project.selectedId).toBe('di-02');
    expect(exists('source.viewer')).toBe(false);
  });
});

describe('w7 #3: missing file soft state', () => {
  beforeEach(reset);

  it('missing-file scenario renders the viewer warning, not fake content', () => {
    source.setScenario('source/missing-file');
    source.openViewer('keyed-02', 0);
    mountCmp(SourceOverlays);
    expect(exists('source.viewer.missing')).toBe(true);
    expect(q('source.viewer.missing').textContent).toContain('Ancient_Complex.xml');
    expect(exists('source.viewer.code')).toBe(false);
  });
});

describe('w7 #4: safe text rendering and search', () => {
  beforeEach(reset);

  it('XML fixture lines render as literal text (no element injection)', () => {
    source.setScenario('source/tkey-multi-context');
    source.openViewer('tk-01', 0);
    mountCmp(SourceOverlays);
    const row = q('source.viewer.node.tk-01.primary');
    // the raw <li> tag text must survive as text, not become an element
    expect(row.textContent).toContain('<li>');
    expect(row.querySelector('li')).toBeNull();
  });

  it('search counts matches and marks the active one', () => {
    source.setScenario('source/tkey-multi-context');
    source.openViewer('tk-01', 0);
    mountCmp(SourceOverlays);
    const input = q('source.viewer.search') as HTMLInputElement;
    input.value = 'colonists';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(q('source.viewer.matchCount').textContent).toContain('1/3');
  });
});

describe('w7 #5: clipboard fulfilled-or-fallback in the viewer', () => {
  beforeEach(reset);

  it('copy line reports success only after the write fulfilled', async () => {
    const writeText = vi.fn<(t: string) => Promise<void>>().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    source.setScenario('source/simple');
    source.openViewer('keyed-01', 0);
    mountCmp(SourceOverlays);
    click('source.viewer.copyLine');
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain('<MessageLetterArrived>');
    expect(exists('source.viewer.fallback')).toBe(false);
  });

  it('denied write shows the selectable fallback, never a false success', async () => {
    const writeText = vi.fn<(t: string) => Promise<void>>().mockRejectedValue(new Error('denied'));
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    source.setScenario('source/simple');
    source.openViewer('keyed-01', 0);
    mountCmp(SourceOverlays);
    click('source.viewer.copyLine');
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(exists('source.viewer.fallback')).toBe(true);
    expect(q('source.viewer.fallback').textContent).toContain('<MessageLetterArrived>');
  });
});

describe('w7 #6: provenance states are data-driven', () => {
  beforeEach(reset);

  it('version-override: primary is effective, other usage is the shadowed twin', () => {
    source.setScenario('source/version-override');
    mountCmp(SourceTab, { entry: entryById('di-03') });
    expect(q('source.tab').textContent).toContain('Other usages (1)');
    expect(q('source.tab').textContent).toContain('1.5/Defs');
  });

  it('browser renders ACTIVE/shadowed candidates from fixture reasons', () => {
    source.setScenario('source/version-override');
    source.openBrowser();
    mountCmp(SourceOverlays);
    const cands = q('source.browser.candidates').textContent ?? '';
    expect(cands).toContain('Common');
    expect(cands).toContain('1.5');
  });
});

describe('w7 #7: external-change banner mock transitions', () => {
  beforeEach(reset);

  it('visible in external-change scenario; Ignore dismisses; Rescan clears with a demo note', () => {
    source.setScenario('source/external-change');
    mountCmp(SourceChangeBanner);
    expect(exists('source.changed.banner')).toBe(true);
    click('source.changed.ignore');
    flushSync();
    expect(exists('source.changed.banner')).toBe(false);
    source.changeRearm();
    flushSync();
    expect(exists('source.changed.banner')).toBe(true);
    click('source.changed.rescan');
    flushSync();
    expect(exists('source.changed.banner')).toBe(false);
  });

  it('no banner in scenarios without a change fixture', () => {
    source.setScenario('source/simple');
    mountCmp(SourceChangeBanner);
    expect(exists('source.changed.banner')).toBe(false);
  });
});

describe('w7 #8: editor launch plans are structural', () => {
  it('unknown placeholders are rejected', () => {
    const r = buildLaunchPlan(
      { executable: 'edit', argsTemplate: ['{file}'] },
      { path: 'a.xml', line: 1, column: 1 }
    );
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reasonKey).toBe('source.editor.error.unknownPlaceholder');
  });

  it('missing line cannot be substituted (no invented numbers)', () => {
    const r = buildLaunchPlan(
      { executable: 'zed', argsTemplate: ['{path}', '{line}'] },
      { path: 'a.xml', line: null, column: null }
    );
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reasonKey).toBe('source.editor.error.noLine');
  });

  it('spaces and Unicode stay literal argv items; quotes group', () => {
    expect(
      splitArgsText('"/Applications/Мой Редактор.app" --wait "{path}.xml"')
    ).toEqual(['/Applications/Мой Редактор.app', '--wait', '{path}.xml']);
    // lead review 029 #2: an unbalanced quote is a typed rejection
    expect(splitArgsTextStrict('"/Applications/Мой')).toEqual({
      ok: false,
      reasonKey: 'source.editor.error.unbalancedQuote'
    });
    const r = buildLaunchPlan(
      { executable: '/usr/local/bin/右クリック', argsTemplate: ['--goto', '{path}:{line}:{column}'] },
      { path: 'Docs/файл пробел.xml', line: 12, column: 4 }
    );
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.plan.args).toEqual(['--goto', 'Docs/файл пробел.xml:12:4']);
  });
});

describe('w7 #8b: literal substitution safety (lead 029 #2)', () => {
  const target = { path: 'Мой $& {line}.xml', line: 12, column: 4 };

  it('replacement text is never reinterpreted ($&, ${}, braces stay literal)', () => {
    const r = buildLaunchPlan({ executable: 'ed', argsTemplate: ['{path}'] }, target);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.plan.args).toEqual(['Мой $& {line}.xml']);
  });

  it('repeated tokens all resolve in one pass', () => {
    const r = buildLaunchPlan(
      { executable: 'ed', argsTemplate: ['{path}', '{path}:{line}:{column}', '{path}'] },
      { path: 'a b.xml', line: 3, column: 7 }
    );
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.plan.args).toEqual(['a b.xml', 'a b.xml:3:7', 'a b.xml']);
  });

  it('standalone {column} substitutes ONLY the column', () => {
    const r = buildLaunchPlan({ executable: 'ed', argsTemplate: ['{path}', '{column}'] }, { path: 'a.xml', line: 12, column: 4 });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.plan.args).toEqual(['a.xml', '4']);
  });

  it('unknown or malformed placeholders are rejected', () => {
    for (const tok of ['{bogus_1}', '{}', '{LINE }']) {
      const r = buildLaunchPlan({ executable: 'ed', argsTemplate: [tok] }, { path: 'a.xml', line: 1, column: 1 });
      expect(r.ok).toBe(false);
      if (!r.ok) expect(r.reasonKey).toBe('source.editor.error.unknownPlaceholder');
    }
  });

  it('line/column must be positive integers when required', () => {
    for (const line of [null, 0, -3, 1.5]) {
      const r = buildLaunchPlan({ executable: 'ed', argsTemplate: ['{line}'] }, { path: 'a.xml', line, column: null });
      expect(r.ok).toBe(false);
      if (!r.ok) expect(r.reasonKey).toBe('source.editor.error.noLine');
    }
    const r = buildLaunchPlan({ executable: 'ed', argsTemplate: ['{column}'] }, { path: 'a.xml', line: 1, column: 0 });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reasonKey).toBe('source.editor.error.noColumn');
  });

  it('corrupt persisted editor choice falls back to defaults', () => {
    localStorage.setItem('rimloc.source.editor', '{"preset":{"evil":1},"custom":null}');
    const c = loadEditorChoice();
    expect(c.preset).toBe('system');
    localStorage.setItem('rimloc.source.editor', 'not json at all');
    expect(loadEditorChoice().preset).toBe('system');
    localStorage.removeItem('rimloc.source.editor');
  });

  it('templateFor rejects an unbalanced custom template', () => {
    const r = templateFor({ preset: 'custom', custom: { executable: 'ed', argsText: '"unclosed' } });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reasonKey).toBe('source.editor.error.unbalancedQuote');
  });
});

describe('w7 #9: unbound source shortcuts coexist with W4.5 registry', () => {
  beforeEach(() => {
    shortcuts.resetAll();
  });

  it('source commands default unbound and never create conflicts', () => {
    for (const id of ['sourceOpen', 'sourceBrowser', 'sourceCompare'] as const) {
      const def = shortcuts.bindings[id];
      expect(def.key).toBe('');
      expect(shortcuts.isConflicting(id)).toBe(false);
    }
    expect(shortcuts.conflictMap().size).toBe(0);
  });

  it('unbound bindings classify clean; existing W4.5 classifications survive', () => {
    expect(shortcuts.issueFor('sourceOpen')).toBeNull();
    // existing reserved warning still classifies (Cmd+Q default is absent in
    // defaults, so force one binding onto a reserved combo)
    shortcuts.assign('save', { key: 'q', mod: true });
    expect(shortcuts.issueFor('save')?.kind).toBe('system-reserved');
    shortcuts.resetAll();
  });
});

describe('w7 #10: runtime shortcut dispatcher (lead 029 #3)', () => {
  beforeEach(() => {
    reset();
    shortcuts.resetAll();
    installSourceShortcutDispatcher();
  });

  it('assigned binding triggers the browser action', () => {
    shortcuts.assign('sourceBrowser', { key: 'b', mod: true });
    project.select('di-02');
    mountCmp(SourceOverlays);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', ctrlKey: true }));
    flushSync();
    expect(source.browser).toBe(true);
    source.closeBrowser();
    shortcuts.resetAll();
  });

  it('unbound commands stay inert', () => {
    project.select('di-02');
    mountCmp(SourceOverlays);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'v', ctrlKey: true }));
    flushSync();
    expect(source.viewer).toBeNull();
  });

  it('typing in an editable target never fires the dispatcher', () => {
    shortcuts.assign('sourceBrowser', { key: 'b', mod: true });
    mountCmp(SourceOverlays);
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'b', ctrlKey: true, bubbles: true }));
    flushSync();
    expect(source.browser).toBe(false);
    input.remove();
    shortcuts.resetAll();
  });

  it('entry actions without a selection report honestly (palette parity)', () => {
    shortcuts.assign('sourceOpen', { key: 'o', mod: true });
    mountCmp(SourceOverlays);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'o', ctrlKey: true }));
    flushSync();
    expect(source.viewer).toBeNull();
    expect(source.lastMockAction?.labelKey).toBe('source.palette.noSelection');
    shortcuts.resetAll();
  });
});

describe('w7 #11: search index clamp and Escape close (lead 029 #4)', () => {
  beforeEach(reset);

  it('advancing then narrowing the query shows a valid index (never 3/1)', async () => {
    source.setScenario('source/tkey-multi-context');
    source.openViewer('tk-01', 0);
    mountCmp(SourceOverlays);
    const input = q('source.viewer.search') as HTMLInputElement;
    const set = (v: string) => {
      const proto = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      proto.call(input, v);
      input.dispatchEvent(new Event('input', { bubbles: true }));
      flushSync();
    };
    set('o'); // several matches
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    flushSync();
    set('winter is close'); // exactly one match
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(q('source.viewer.matchCount').textContent).toContain('1/1');
  });

  it('Escape closes the viewer', () => {
    source.setScenario('source/simple');
    source.openViewer('keyed-01', 0);
    mountCmp(SourceOverlays);
    expect(exists('source.viewer')).toBe(true);
    document
      .querySelector('[data-testid="source.viewer"]')
      ?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    flushSync();
    expect(exists('source.viewer')).toBe(false);
  });
});
