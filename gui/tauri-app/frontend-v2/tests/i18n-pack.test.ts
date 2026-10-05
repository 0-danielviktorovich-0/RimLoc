// Language pack loading (self-localization wave B2, mandate §10/§15).
//   - strict whole-pack validation: ANY structural problem rejects the pack
//     whole, the runtime state stays untouched;
//   - incomplete packs are valid: missing ids fall back to the built-in
//     dictionaries (locale -> en -> id-as-diagnostic, never an empty render);
//   - preview is explicit: previewPack() installs, clearPreview() removes,
//     nothing persists (not localStorage, not the project target locales —
//     the §15 invariant: UI preview locale ≠ project source/target locales);
//   - data-only trust boundary: pack values render as text; markup/handlers
//     in values cannot become DOM (<script>, onerror) — Svelte text
//     interpolation escapes them.
import { beforeEach, afterEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import {
  PACK_VALUE_MAX_LENGTH,
  extractPlaceholders,
  parseAndValidatePack,
  validatePackObject
} from '../src/i18n/pack-schema';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { i18n } from '../src/i18n/store.svelte';
import { languages } from '../src/lib/languages/store.svelte';
import App from '../src/App.svelte';
import DevPanel from '../src/lib/components/DevPanel.svelte';
import { click, cleanupMounted, exists, goto, mountCmp, q } from './helpers';

/** A pack built over the real en catalog: every value prefixed so any
 * rendered string is provably from the pack. */
function fullPack(locale = 'ja'): unknown {
  return {
    schema_version: '1',
    locale,
    base_catalog_revision: 'a709064be8ae995997559915d3da3e5fb9a4de95',
    messages: Object.entries(en).map(([id, value]) => ({ id, value: `⟨${locale}⟩ ${value}` }))
  };
}

function partialPack(locale = 'ja'): unknown {
  return {
    schema_version: '1',
    locale,
    base_catalog_revision: 'a709064be8ae995997559915d3da3e5fb9a4de95',
    messages: [
      { id: 'common.appName', value: 'リムロック' },
      { id: 'common.close', value: '閉じる' }
    ]
  };
}

beforeEach(() => {
  i18n.setLocale('ru');
  i18n.clearPreview();
});

afterEach(() => {
  i18n.clearPreview();
  cleanupMounted();
});

describe('pack schema: whole-pack validation', () => {
  it('accepts a valid partial pack', () => {
    const result = validatePackObject(partialPack(), en);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.pack.locale).toBe('ja');
      expect(result.pack.messages).toHaveLength(2);
    }
  });

  it('accepts an incomplete pack (missing ids are a fallback case, not an error)', () => {
    const result = validatePackObject(
      {
        schema_version: '1',
        locale: 'ja',
        base_catalog_revision: 'a709064',
        messages: [{ id: 'common.appName', value: 'リムロック' }]
      },
      en
    );
    expect(result.ok).toBe(true);
  });

  it('rejects an unknown schema_version', () => {
    const pack = { ...fullPack(), schema_version: '2' };
    const result = validatePackObject(pack, en);
    expect(result).toMatchObject({ ok: false, reason: 'unknown_schema_version' });
  });

  it('rejects a pack with an id that does not exist in the base catalog', () => {
    const pack = {
      ...partialPack(),
      messages: [{ id: 'totally.foreign.id', value: 'whatever' }]
    };
    const result = validatePackObject(pack, en);
    expect(result).toMatchObject({ ok: false, reason: 'unknown_id' });
  });

  it('rejects a value whose placeholder set differs from the base contract', () => {
    const pack = {
      ...partialPack(),
      messages: [{ id: 'wizard.stepOf', value: 'Шаг {step} из {total} (ещё {extra})' }]
    };
    const result = validatePackObject(pack, en);
    expect(result).toMatchObject({ ok: false, reason: 'placeholder_mismatch' });
  });

  it('placeholder sets are compared as sets: reordered base tokens still match', () => {
    expect(extractPlaceholders('{total} потом {step}')).toEqual(
      extractPlaceholders('{step} из {total}')
    );
  });

  it('rejects an empty value, an oversized value and duplicate ids whole', () => {
    const empty = { ...partialPack(), messages: [{ id: 'common.appName', value: '  ' }] };
    expect(validatePackObject(empty, en)).toMatchObject({ ok: false, reason: 'empty_value' });

    const long = {
      ...partialPack(),
      messages: [{ id: 'common.appName', value: 'x'.repeat(PACK_VALUE_MAX_LENGTH + 1) }]
    };
    expect(validatePackObject(long, en)).toMatchObject({ ok: false, reason: 'value_too_long' });

    const dup = {
      ...partialPack(),
      messages: [
        { id: 'common.appName', value: 'a' },
        { id: 'common.appName', value: 'b' }
      ]
    };
    expect(validatePackObject(dup, en)).toMatchObject({ ok: false, reason: 'duplicate_id' });
  });

  it('rejects malformed roots, bad locale/revision shapes and broken JSON', () => {
    expect(validatePackObject('not an object', en)).toMatchObject({ ok: false, reason: 'malformed' });
    expect(validatePackObject([1, 2], en)).toMatchObject({ ok: false, reason: 'malformed' });
    expect(validatePackObject({ ...partialPack(), locale: 'not a locale!' }, en)).toMatchObject({
      ok: false,
      reason: 'bad_locale'
    });
    expect(validatePackObject({ ...partialPack(), base_catalog_revision: 'r2d2' }, en)).toMatchObject({
      ok: false,
      reason: 'bad_revision'
    });
    expect(parseAndValidatePack('{nope', en)).toMatchObject({ ok: false, reason: 'malformed' });
  });

  it('subtag forms like zh-Hans / pt-BR are valid pack locales', () => {
    for (const locale of ['zh-Hans', 'pt-BR', 'uk']) {
      const result = validatePackObject(partialPack(locale), en);
      expect(result.ok, locale).toBe(true);
    }
  });
});

describe('pack runtime: preview overlay + safe fallback', () => {
  it('a valid full pack overrides every message', () => {
    const result = i18n.previewPack(fullPack('ja'));
    expect(result.ok).toBe(true);
    for (const [id, value] of Object.entries(en)) {
      expect(i18n.t(id)).toBe(`⟨ja⟩ ${value}`);
    }
  });

  it('an incomplete pack falls back to the built-in dictionary for missing ids', () => {
    expect(i18n.previewPack(partialPack()).ok).toBe(true);
    expect(i18n.t('common.appName')).toBe('リムロック');
    expect(i18n.t('common.close')).toBe('閉じる');
    // missing in the pack -> ru dictionary (current locale chain)
    expect(i18n.t('common.retry')).toBe(ru['common.retry']);
  });

  it('a rejected pack changes nothing: UI stays on the built-in dictionary', () => {
    const broken = {
      schema_version: '1',
      locale: 'ja',
      base_catalog_revision: 'a709064',
      messages: [{ id: 'wizard.stepOf', value: 'Step {step} of {total} plus {oops}' }]
    };
    const before = i18n.t('wizard.stepOf', { step: 1, total: 2 });
    const result = i18n.previewPack(broken);
    expect(result).toMatchObject({ ok: false, reason: 'placeholder_mismatch' });
    expect(i18n.previewActive).toBe(false);
    expect(i18n.t('wizard.stepOf', { step: 1, total: 2 })).toBe(before);
  });

  it('preview -> clearPreview returns rendering to the built-in dictionaries', () => {
    i18n.previewPack(partialPack());
    expect(i18n.t('common.appName')).toBe('リムロック');
    i18n.clearPreview();
    expect(i18n.previewActive).toBe(false);
    expect(i18n.preview).toBeNull();
    expect(i18n.t('common.appName')).toBe(ru['common.appName']);
  });

  it('fallback never renders an empty string: the last resort is the id as diagnostic', () => {
    i18n.previewPack(partialPack());
    expect(i18n.t('no.such.key.anywhere')).toBe('no.such.key.anywhere');
    // and a pack can never introduce an empty render: empty values are
    // rejected whole at load time (validated above).
  });

  it('the §15 invariant: a UI pack preview never touches project source/target locales', () => {
    goto('#/');
    const targetsBefore = JSON.stringify(languages.targets);
    const pinnedBefore = JSON.stringify(languages.pinned);

    i18n.setLocale('en');
    const storageBefore = window.localStorage.getItem('rimloc.locale');

    const result = i18n.previewPack(fullPack('ja'));
    expect(result.ok).toBe(true);

    // UI locale setting untouched...
    expect(i18n.locale).toBe('en');
    expect(window.localStorage.getItem('rimloc.locale')).toBe(storageBefore);
    // ...and the project target-locale dataset untouched.
    expect(languages.activeLocale).toBe('ru');
    expect(JSON.stringify(languages.targets)).toBe(targetsBefore);
    expect(JSON.stringify(languages.pinned)).toBe(pinnedBefore);
  });

  // SF-08 (preview metadata): the document lang/dir must reflect the
  // language ACTUALLY RENDERED. The earlier version of the §15 test pinned
  // the opposite (lang left stale) — that stale-metadata behavior is
  // exactly what the review flagged: a Japanese preview under
  // `html lang="ru"` lies about the shown language, and an RTL preview
  // without `dir="rtl"` renders mirrored. The §15 INVARIANT itself (UI
  // locale preference, storage, project targets untouched) is asserted
  // above and stays intact.
  describe('SF-08: preview document lang/dir metadata', () => {
    it('preview ja sets html lang=ja; clearPreview restores the previous lang', () => {
      const langBefore = document.documentElement.lang;
      i18n.previewPack(fullPack('ja'));
      expect(i18n.previewActive).toBe(true);
      expect(document.documentElement.lang).toBe('ja');
      i18n.clearPreview();
      expect(document.documentElement.lang).toBe(langBefore);
      expect(document.documentElement.hasAttribute('dir')).toBe(false);
    });

    it('an RTL preview locale (he) sets dir=rtl; clearPreview restores it', () => {
      expect(document.documentElement.getAttribute('dir')).toBeNull();
      i18n.previewPack(fullPack('he'));
      expect(document.documentElement.dir).toBe('rtl');
      expect(document.documentElement.lang).toBe('he');
      i18n.clearPreview();
      expect(document.documentElement.getAttribute('dir')).toBeNull();
    });

    it('a nested preview does not overwrite the original snapshot', () => {
      const langBefore = document.documentElement.lang;
      i18n.previewPack(fullPack('ja'));
      i18n.previewPack(fullPack('ar-EG')); // second load without clearing
      expect(document.documentElement.lang).toBe('ar-EG');
      expect(document.documentElement.dir).toBe('rtl');
      i18n.clearPreview();
      expect(document.documentElement.lang).toBe(langBefore);
      expect(document.documentElement.getAttribute('dir')).toBeNull();
    });

    it('switching the UI locale during a preview keeps the preview metadata', () => {
      i18n.previewPack(fullPack('ja'));
      i18n.setLocale('en');
      // The rendered language is still the preview's — the preference
      // change must not lie in the document metadata.
      expect(i18n.locale).toBe('en');
      expect(document.documentElement.lang).toBe('ja');
      i18n.clearPreview();
      // ...and the restore goes to the CURRENT preference, not the
      // snapshotted one, because the user changed it mid-preview.
      expect(document.documentElement.lang).toBe('en');
    });
  });
});

describe('pack UI: explicit load + explicit reset in the dev panel', () => {
  function pickFile(testid: string, text: string) {
    const input = q(testid) as HTMLInputElement;
    const file = new File([text], 'pack.json', { type: 'application/json' });
    Object.defineProperty(input, 'files', { value: [file] });
    input.dispatchEvent(new Event('change', { bubbles: true }));
    flushSync();
  }

  async function settleReader() {
    for (let i = 0; i < 10; i++) {
      await Promise.resolve();
      await new Promise((r) => setTimeout(r, 5));
      flushSync();
      if (i18n.previewActive) return;
    }
    flushSync();
  }

  it('loads a pack file client-side, shows active state, resets back to idle', async () => {
    mountCmp(DevPanel);
    expect(q('dev.pack.status').textContent).toContain(i18n.t('dev.pack.idle'));

    pickFile('dev.pack.file', JSON.stringify(partialPack()));
    click('dev.pack.load');
    await settleReader();

    expect(i18n.previewActive).toBe(true);
    expect(q('dev.pack.status').textContent).toContain('ja');
    expect(exists('dev.pack.reset')).toBe(true);

    click('dev.pack.reset');
    expect(i18n.previewActive).toBe(false);
    expect(q('dev.pack.status').textContent).toContain(i18n.t('dev.pack.idle'));
  });

  it('shows the machine reject reason for a broken pack file', async () => {
    mountCmp(DevPanel);
    pickFile('dev.pack.file', '{"schema_version": "9", "locale": "ja"}');
    click('dev.pack.load');
    await settleReader();

    expect(i18n.previewActive).toBe(false);
    expect(q('dev.pack.status').textContent).toContain('unknown_schema_version');
  });
});

describe('SF-07: interpolation keeps parameter values literal', () => {
  // The store reads the pack overlay FIRST, so a preview pack over a
  // placeholder-bearing catalog id is the precise harness: the value passes
  // the placeholder-set contract (same tokens as the base) and t() renders it.
  function overlayWithValue(id: string, value: string) {
    const result = i18n.previewPack({
      schema_version: '1',
      locale: 'ja',
      base_catalog_revision: 'a709064',
      messages: [{ id, value }]
    });
    expect(result.ok, `${id}: ${JSON.stringify(result)}`).toBe(true);
  }

  // `wizard.w2.selected.summary` = "Selected: {name}" in en — the {name}
  // token the $-family probes hang on.
  it.each(['$&', '$`', '$\'', '$$'])('a %s sequence in the VALUE stays literal', (value) => {
    overlayWithValue('wizard.w2.selected.summary', 'Hello {name}!');
    expect(i18n.t('wizard.w2.selected.summary', { name: value })).toBe(`Hello ${value}!`);
  });

  it('a value that looks like a token is NOT re-interpolated (single pass)', () => {
    // review counterexample: A={a}; B={b} with a="{b}", b="B_VALUE" used to
    // render "A=B_VALUE; B=B_VALUE" — the second pass re-ran on the inserted
    // text. The value must surface verbatim.
    overlayWithValue('wizard.stepOf', 'A={step}; B={total}');
    expect(i18n.t('wizard.stepOf', { step: '{total}', total: 'TOTAL_VALUE' })).toBe(
      'A={total}; B=TOTAL_VALUE'
    );
  });

  it('Unicode parameter values pass through unchanged', () => {
    overlayWithValue('wizard.w2.selected.summary', 'Selected: {name}');
    expect(i18n.t('wizard.w2.selected.summary', { name: 'Привет 🌍' })).toBe('Selected: Привет 🌍');
  });

  it('a key without tokens is untouched even when params are given', () => {
    expect(i18n.t('common.appName', { name: '$&', x: '{y}' })).toBe('RimLoc');
  });

  it('a parameter name absent from params keeps its literal token', () => {
    overlayWithValue('wizard.stepOf', 'Step {step} of {total}');
    const rendered = i18n.t('wizard.stepOf', { step: 1 });
    expect(rendered).toContain('{total}');
    expect(rendered).toBe('Step 1 of {total}');
  });
});

describe('pack trust boundary: values render as text, never as markup', () => {
  it('<script> and onerror= in pack values appear literally, no executable DOM', () => {
    const evil = {
      schema_version: '1',
      locale: 'ja',
      base_catalog_revision: 'a709064',
      // both ids render as visible text on the default route
      messages: [
        { id: 'common.appName', value: '<script>alert(1)</script>' },
        { id: 'home.title', value: '<img src=x onerror=alert(1)>' }
      ]
    };
    expect(i18n.previewPack(evil).ok).toBe(true);

    goto('#/');
    mountCmp(App);
    flushSync();

    const body = document.body;
    // no script element, no onerror attribute anywhere in the mounted tree
    expect(body.querySelector('script')).toBeNull();
    expect(body.querySelector('[onerror]')).toBeNull();
    // the values are present as escaped TEXT
    expect(body.textContent).toContain('<script>alert(1)</script>');
    expect(body.textContent).toContain('<img src=x onerror=alert(1)>');
  });
});
