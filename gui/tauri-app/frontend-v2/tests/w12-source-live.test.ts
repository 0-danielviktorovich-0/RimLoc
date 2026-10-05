// Wave 12: the Source Inspector LIVE bridge. The SOURCE tab has two modes
// that must never mix:
//  - tauri/contract: the snapshot projected the entry's real source as
//    `source_ref` (project-root-relative file + parser-guaranteed line +
//    winner reason). The tab renders THAT and never fixture data —
//    fixture usages/compare/browser stay hidden, unavailability is honest.
//  - mock: the fixture seam behaves exactly as before (w7 regressions hold).
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import { exists, mountCmp, q } from './helpers';
import { project } from '../src/lib/stores/project.svelte';
import { source } from '../src/lib/source/store.svelte';
import { i18n } from '../src/i18n/store.svelte';
import SourceTab from '../src/lib/components/source/SourceTab.svelte';

type LiveRef = NonNullable<
  import('../src/lib/mock/types').Entry['sourceRef']
>;

function liveEntry(ref: LiveRef | null): import('../src/lib/mock/types').Entry {
  return {
    id: 'keyed:live.entry',
    kind: 'Keyed',
    key: 'live.entry',
    source: 'Live source text',
    target: '',
    status: 'untranslated',
    file: '',
    line: 0,
    sourceRef: ref
  };
}

const REF: LiveRef = {
  file: 'Languages/English/Keyed/Misc_Gameplay.xml',
  line: 12,
  selected_by: 'keyed-first-in-file'
};

describe('w12 #1: tauri mode renders the live source_ref, never fixtures', () => {
  beforeEach(() => {
    i18n.locale = 'en';
    project.source = 'contract';
    source.setScenario('source/simple');
    source.closeViewer();
    source.closeBrowser();
  });

  it('shows the projected file, the guaranteed line and the human winner reason', () => {
    mountCmp(SourceTab, { entry: liveEntry(REF) });
    expect(exists('source.tab.live')).toBe(true);
    // the live path, not the fixture one (fixture would be the same file
    // here — the decisive part is that the value comes from source_ref)
    expect(q('source.tab.live.path').textContent).toBe(REF.file);
    expect(q('source.tab.live.path').textContent).not.toBe(
      'Languages/English/Keyed/Ancient_Complex.xml'
    );
    expect(q('source.tab.live.line').textContent).toContain('12');
    expect(q('source.tab.live.why').textContent).toContain('first occurrence in the file wins');
  });

  it('hides fixture usages/compare/browser and says unavailability honestly', () => {
    mountCmp(SourceTab, { entry: liveEntry(REF) });
    // the unavailable note replaces fixture usages
    expect(q('source.tab.live.usagesUnavailable').textContent).toContain(
      'not available for this project'
    );
    // no fixture actions: viewer/browser/compare/reveal stay mock-only
    expect(exists('source.action.open')).toBe(false);
    expect(exists('source.action.candidates')).toBe(false);
    expect(exists('source.action.compare')).toBe(false);
    // the fixture advanced file-refs block is not rendered either
    expect(exists('source.tab.advanced')).toBe(false);
  });

  it('an unknown future winner reason renders as the raw token, not a guessed phrase', () => {
    mountCmp(SourceTab, {
      entry: liveEntry({ ...REF, selected_by: 'some-future-reason' })
    });
    expect(q('source.tab.live.why').textContent).toContain('some-future-reason');
  });

  it('a null line is honest ("line unknown"), never a fabricated number', () => {
    mountCmp(SourceTab, { entry: liveEntry({ ...REF, line: null }) });
    expect(q('source.tab.live.line').textContent).toContain('line unknown');
  });

  it('a contract entry without a projection shows the live empty state', () => {
    mountCmp(SourceTab, { entry: liveEntry(null) });
    expect(exists('source.tab.live.empty')).toBe(true);
    expect(exists('source.tab.live')).toBe(false);
    // ...and never falls back to fixture data
    expect(exists('source.tab')).toBe(false);
  });

  it('editor action plans from the live path and signs the preview as live-honest', () => {
    localStorage.setItem(
      'rimloc.source.editor',
      JSON.stringify({ preset: 'zed', custom: { executable: '', argsText: '{path}' } })
    );
    try {
      mountCmp(SourceTab, { entry: liveEntry(REF) });
      q('source.tab.live.editor').click();
      flushSync();
      // the argv preview substitutes the LIVE projected path + line
      expect(q('source.tab.live.editorPreview').textContent).toBe(
        JSON.stringify(['zed', REF.file, String(REF.line)])
      );
      // the honest live signature replaces the demo wording
      expect(document.body.textContent).toContain(
        'Live project: this is an argv preview — nothing was launched.'
      );
    } finally {
      localStorage.removeItem('rimloc.source.editor');
    }
  });

  it('a null line rejects an editor template that needs one (no invented numbers)', () => {
    localStorage.setItem(
      'rimloc.source.editor',
      JSON.stringify({ preset: 'zed', custom: { executable: '', argsText: '{path}' } })
    );
    try {
      mountCmp(SourceTab, { entry: liveEntry({ ...REF, line: null }) });
      q('source.tab.live.editor').click();
      flushSync();
      expect(exists('source.tab.live.editorPreview')).toBe(false);
      expect(document.body.textContent).toContain('Line number is not guaranteed');
    } finally {
      localStorage.removeItem('rimloc.source.editor');
    }
  });
});

describe('w12 #2: mock mode keeps the fixture behavior intact', () => {
  beforeEach(() => {
    i18n.locale = 'en';
    project.source = 'fixture';
    source.setScenario('source/simple');
  });

  it('fixture entries render fixture data and keep the full action set', () => {
    const entry = {
      id: 'keyed-01',
      kind: 'Keyed',
      key: 'MessageLetterArrived',
      source: 'text',
      target: '',
      status: 'untranslated' as const,
      file: '',
      line: 0,
      sourceRef: null
    };
    mountCmp(SourceTab, { entry });
    expect(exists('source.tab.live')).toBe(false);
    expect(exists('source.tab')).toBe(true);
    expect(q('source.tab.path').textContent).toContain('Misc_Gameplay.xml');
    // fixture usages/actions are alive in mock
    expect(exists('source.action.open')).toBe(true);
    expect(exists('source.action.editor')).toBe(true);
    expect(exists('source.action.copy')).toBe(true);
  });

  it('a fixture-mode entry without fixture coverage keeps the w7 empty state', () => {
    mountCmp(SourceTab, { entry: liveEntry(null) });
    expect(exists('source.tab.empty')).toBe(true);
    expect(exists('source.tab.live.empty')).toBe(false);
  });
});
