// W4.5 integration regressions (AUTONOMOUS_STATUS §W4.5 п.1-4):
//   1. Provider credential semantics — duplicate copies non-secret config
//      only with an explicit credential decision; export serializes without
//      any credential fields.
//   2. Glossary/TM mutability provenance — reference-read-only rows refuse
//      edit/delete (UI disabled + store-level guard), batch import marks
//      rows `imported`.
//   3. Shortcut safety classes — rimloc conflicts block, system-reserved and
//      convention combos save with warnings/info.
//   4. About version comes from the single version module, not a literal.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import pkg from '../package.json';
import About from '../src/lib/components/screens/About.svelte';
import GlossaryEditor from '../src/lib/components/screens/GlossaryEditor.svelte';
import ProviderManager from '../src/lib/components/screens/ProviderManager.svelte';
import ShortcutsEditor from '../src/lib/components/ShortcutsEditor.svelte';
import TMEditor from '../src/lib/components/screens/TMEditor.svelte';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { i18n } from '../src/i18n/store.svelte';
import { APP_VERSION } from '../src/lib/version';
import { isReadOnly, MUTABILITY_VALUES } from '../src/lib/mutability';
import { glossary } from '../src/lib/stores/glossary.svelte';
import { tm } from '../src/lib/stores/tm.svelte';
import { shortcuts, classifyBinding, SHORTCUT_DEFS } from '../src/lib/stores/shortcuts.svelte';
import { click, exists, mountCmp, q } from './helpers';

const NEW_KEYS = [
  'providers.inst.export',
  'providers.export.done',
  'providers.inst.cred.shared',
  'providers.inst.cred.sharedHint',
  'providers.inst.cred.missing',
  'providers.duplicate.title',
  'providers.duplicate.note',
  'providers.duplicate.reuse',
  'providers.duplicate.needs',
  'providers.duplicate.sharedFlash',
  'providers.duplicate.missingFlash',
  'providers.duplicate.reuseUnavailable',
  'providers.duplicate.local',
  'providers.duplicate.localNote',
  'providers.duplicate.localFlash',
  'providers.export.unavailable',
  'providers.export.denied',
  'providers.export.fallbackNote',
  'shortcuts.issue.rimlocConflict',
  'shortcuts.issue.systemReserved',
  'shortcuts.issue.convention',
  'shortcuts.blocked',
  'shortcuts.warnReserved',
  'shortcuts.infoConvention',
  'mutability.label',
  'mutability.user-created',
  'mutability.project-created',
  'mutability.imported',
  'mutability.reference-read-only',
  'mutability.readOnlyReason'
];

describe('w45 #4: about version from the single version module', () => {
  it('About renders APP_VERSION read from package.json metadata, not a literal', () => {
    i18n.setLocale('en');
    mountCmp(About);
    const text = q('help.about.version').textContent ?? '';
    expect(text).toContain(`RimLoc ${APP_VERSION}`);
    // Compare against the REAL package metadata so a legitimate version bump
    // never requires editing this test (behavior-review 008 #4).
    expect(APP_VERSION).toBe(pkg.version);
  });
});

describe('w45 #1: provider credential semantics', () => {
  beforeEach(() => {
    i18n.setLocale('en');
  });

  it('duplicate asks for an explicit credential decision', () => {
    mountCmp(ProviderManager);
    expect(exists('providers.inst.duplicateChoice.inst-1')).toBe(false);
    click('providers.inst.duplicate.inst-1');
    expect(exists('providers.inst.duplicateChoice.inst-1')).toBe(true);
    // Until the user decides, no copy exists.
    expect(exists('providers.inst.credShared.inst-4')).toBe(false);
  });

  it('reuse path: shared keychain reference, non-secret config copied', () => {
    mountCmp(ProviderManager);
    click('providers.inst.duplicate.inst-1');
    click('providers.inst.duplicateReuse.inst-1');
    // The copy resolves to a SHARED reference, visibly marked.
    expect(exists('providers.inst.credShared.inst-4')).toBe(true);
    expect(q('providers.inst.key.inst-4').textContent).toContain('shared keychain reference');
    // Non-secret configuration travelled: base URL and model are present.
    const card = q('providers.inst.inst-4').textContent ?? '';
    expect(card).toContain('https://api.z.ai/api/anthropic');
    expect(card).toContain('glm-4.6');
    // The copy is never the default, and only non-secret state moved.
    expect(exists('providers.inst.default.inst-4')).toBe(false);
  });

  it('shared reference is one identity: copy of a copy and keychain untouched after source removal', () => {
    mountCmp(ProviderManager);
    click('providers.inst.duplicate.inst-1');
    click('providers.inst.duplicateReuse.inst-1'); // inst-4 shares kc-1
    // Opaque handle travels as an identity (attribute, never rendered text).
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toBe(
      q('providers.inst.inst-1').getAttribute('data-cred-ref')
    );
    // Removing the source keeps the keychain entry (per removeWarn), so the
    // shared copy keeps resolving — no silent "connected" lie, no downgrade.
    click('providers.inst.remove.inst-1');
    click('providers.inst.remove.inst-1');
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toBe('kc-1');
    expect(q('providers.inst.key.inst-4').textContent).toContain('shared keychain reference');
  });

  it('replacing the key on a shared copy detaches it into its own keychain entry', () => {
    mountCmp(ProviderManager);
    click('providers.inst.duplicate.inst-1');
    click('providers.inst.duplicateReuse.inst-1'); // inst-4 shared
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toBe('kc-1');
    click('providers.inst.replaceKey.inst-4');
    // Detached: own badge gone, fresh own handle, original keeps its handle.
    expect(exists('providers.inst.credShared.inst-4')).toBe(false);
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).not.toBe('kc-1');
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toMatch(/^kc-\d+$/);
    expect(q('providers.inst.inst-1').getAttribute('data-cred-ref')).toBe('kc-1');
  });

  it('needs-credential path: copy starts without a key and unconfigured', () => {
    mountCmp(ProviderManager);
    click('providers.inst.duplicate.inst-1');
    click('providers.inst.duplicateMissing.inst-1');
    expect(exists('providers.inst.credMissing.inst-4')).toBe(true);
    expect(q('providers.inst.key.inst-4').textContent).toContain('No key yet');
    expect(q('providers.inst.status.inst-4').textContent).toContain('Not configured');
    // No resolvable identity is manufactured for a keyless copy.
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toBe('');
  });

  it('keyless keychain provider (My VPS) cannot manufacture a credential on duplicate', () => {
    mountCmp(ProviderManager);
    click('providers.inst.duplicate.inst-3');
    // UI guard: reuse disabled with an explanatory tooltip…
    const reuse = q('providers.inst.duplicateReuse.inst-3') as HTMLButtonElement;
    expect(reuse.disabled).toBe(true);
    expect(reuse.title).toBe(en['providers.duplicate.reuseUnavailable']);
    // …and the handler cannot be tricked into a credential either: the only
    // path forward marks the copy "needs credential", not connected.
    click('providers.inst.duplicateMissing.inst-3');
    expect(exists('providers.inst.credMissing.inst-4')).toBe(true);
    expect(q('providers.inst.status.inst-4').textContent).toContain('Not configured');
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toBe('');
  });

  it('local no-key family duplicates plainly and honestly (no credential claims)', () => {
    mountCmp(ProviderManager);
    click('providers.inst.duplicate.inst-2');
    // No credential decision is offered for a keyless local service.
    expect(exists('providers.inst.duplicateReuse.inst-2')).toBe(false);
    expect(exists('providers.inst.duplicateMissing.inst-2')).toBe(false);
    click('providers.inst.duplicateLocal.inst-2');
    const card = q('providers.inst.inst-4').textContent ?? '';
    expect(card).toContain('Local Ollama — copy');
    expect(card).toContain('Offline'); // honest state, not "needs credential"
    expect(exists('providers.inst.credMissing.inst-4')).toBe(false);
    expect(q('providers.inst.inst-4').getAttribute('data-cred-ref')).toBe('');
  });

  it('export JSON contains NO credential fields at all (absent, not empty)', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    mountCmp(ProviderManager);
    await click('providers.inst.export.inst-1');
    expect(writeText).toHaveBeenCalledTimes(1);
    const exported = JSON.parse(writeText.mock.calls[0][0]) as Record<string, unknown>;
    expect(Object.keys(exported).sort()).toEqual(['baseUrl', 'discovery', 'family', 'model', 'name']);
    expect(exported.name).toBe('Z.AI Personal');
    for (const key of ['auth', 'hasKey', 'credential', 'credRef']) {
      expect(key in exported).toBe(false);
    }
  });

  it('export reports success only after the clipboard write fulfilled', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    mountCmp(ProviderManager);
    q('providers.inst.export.inst-1').click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(q('providers.inst.flash.inst-1').textContent ?? '').toContain('Exported');
    expect(exists('providers.inst.exportFallback.inst-1')).toBe(false);
  });

  it('export on clipboard denial falls back to a selectable JSON, no false success', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockRejectedValue(new Error('denied'));
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    mountCmp(ProviderManager);
    q('providers.inst.export.inst-1').click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(q('providers.inst.flash.inst-1').textContent ?? '').toContain('denied');
    const pre = q('providers.inst.exportJson.inst-1').textContent ?? '';
    expect(JSON.parse(pre)).toEqual({ name: 'Z.AI Personal', family: 'zai', baseUrl: 'https://api.z.ai/api/anthropic', model: 'glm-4.6', discovery: 'auto' });
  });

  it('export without a clipboard API falls back to a selectable JSON, no false success', async () => {
    Object.defineProperty(navigator, 'clipboard', { value: undefined, configurable: true });
    mountCmp(ProviderManager);
    q('providers.inst.export.inst-1').click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(q('providers.inst.flash.inst-1').textContent ?? '').toContain('Clipboard unavailable');
    expect(exists('providers.inst.exportJson.inst-1')).toBe(true);
  });
});

describe('w45 #2: glossary mutability provenance', () => {
  beforeEach(() => {
    i18n.setLocale('en');
  });

  it('mock corpus marks reference terms read-only and they cannot be edited or deleted', () => {
    // glossary-002 "Hydroponics" and glossary-009 "caravan" are reference.
    expect(glossary.list.filter((t) => t.mutability === 'reference-read-only').map((t) => t.id).sort())
      .toEqual(['glossary-002', 'glossary-009']);
    // Store refuses mutation regardless of UI state.
    glossary.update('glossary-002', { target: 'HACKED' });
    expect(glossary.list.find((t) => t.id === 'glossary-002')?.target).toBe('Гидропоника');
    glossary.remove('glossary-002');
    expect(glossary.list.some((t) => t.id === 'glossary-002')).toBe(true);
    expect(isReadOnly('reference-read-only')).toBe(true);
    expect(isReadOnly('imported')).toBe(false);
  });

  it('editor disables edit/delete on read-only rows with a reason tooltip', () => {
    mountCmp(GlossaryEditor);
    const edit = q('glossary.edit.glossary-002') as HTMLButtonElement;
    const del = q('glossary.delete.glossary-002') as HTMLButtonElement;
    expect(edit.disabled).toBe(true);
    expect(del.disabled).toBe(true);
    expect(edit.title).toBe(en['mutability.readOnlyReason']);
    // A mutable row stays editable.
    const editOk = q('glossary.edit.glossary-001') as HTMLButtonElement;
    expect(editOk.disabled).toBe(false);
    // Provenance chip renders the read-only label.
    expect(q('glossary.mut.glossary-002').textContent).toContain('Reference (read-only)');
  });

  it('batch import marks the new term imported; manual add follows the scope', () => {
    mountCmp(GlossaryEditor);
    const before = glossary.list.length;
    click('glossary.import');
    expect(glossary.list.length).toBe(before + 1);
    expect(glossary.list[glossary.list.length - 1].mutability).toBe('imported');
    // Manual add derives mutability from the scope (behavior-review 008 #5):
    // project scope → project-created, personal library → user-created.
    glossary.add({
      source: 'x-test-term',
      target: 'тест',
      note: '',
      sourceLang: 'en',
      targetLang: 'ru',
      scope: 'project',
      variants: [],
      caseSensitive: false
    });
    expect(glossary.list[glossary.list.length - 1].mutability).toBe('project-created');
    glossary.add({
      source: 'x-test-term-2',
      target: 'тест 2',
      note: '',
      sourceLang: 'en',
      targetLang: 'ru',
      scope: 'user',
      variants: [],
      caseSensitive: false
    });
    expect(glossary.list[glossary.list.length - 1].mutability).toBe('user-created');
  });
});

describe('w45 #2: TM mutability provenance', () => {
  beforeEach(() => {
    i18n.setLocale('en');
  });

  it('reference TM rows are read-only in store and editor', () => {
    expect(tm.list.filter((e) => e.mutability === 'reference-read-only').map((e) => e.id).sort())
      .toEqual(['tm-003', 'tm-009']);
    const before = tm.list.find((e) => e.id === 'tm-003')?.target;
    tm.update('tm-003', { target: 'HACKED' });
    expect(tm.list.find((e) => e.id === 'tm-003')?.target).toBe(before);
    tm.remove('tm-003');
    expect(tm.list.some((e) => e.id === 'tm-003')).toBe(true);

    mountCmp(TMEditor);
    expect((q('tm.edit.tm-003') as HTMLButtonElement).disabled).toBe(true);
    expect((q('tm.delete.tm-003') as HTMLButtonElement).disabled).toBe(true);
    expect(q('tm.mut.tm-003').textContent).toContain('Reference (read-only)');
  });

  it('batch import appends a row marked imported', () => {
    mountCmp(TMEditor);
    const before = tm.list.length;
    click('tm.import');
    expect(tm.list.length).toBe(before + 1);
    expect(tm.list[tm.list.length - 1].mutability).toBe('imported');
  });
});

describe('w45 #3: shortcut safety classes', () => {
  beforeEach(() => {
    i18n.setLocale('en');
    shortcuts.resetAll();
  });

  it('classification: rimloc conflict is an error', () => {
    const issue = classifyBinding('save', { key: 'f', mod: true }, shortcuts.bindings);
    expect(issue?.cls).toBe('error');
    expect(issue?.kind).toBe('rimloc-conflict');
  });

  it('classification: system-reserved combo (Ctrl/Cmd+Q class) is a warning', () => {
    const issue = classifyBinding('save', { key: 'q', mod: true }, shortcuts.bindings);
    expect(issue?.cls).toBe('warning');
    expect(issue?.kind).toBe('system-reserved');
  });

  it('classification: convention combo moved to another command is info; matching own default is clean', () => {
    // Cmd/Ctrl+S on "Save edit" IS the convention — no issue.
    expect(classifyBinding('save', { key: 's', mod: true }, shortcuts.bindings)).toBeNull();
    // With "save" moved off Ctrl+S, the palette taking Ctrl+S shadows the
    // save convention — info only, no RimLoc conflict.
    const moved = { ...shortcuts.bindings, save: { key: 'j', mod: true } };
    const issue = classifyBinding('palette', { key: 's', mod: true }, moved);
    expect(issue?.cls).toBe('info');
    expect(issue?.kind).toBe('convention');
    // Defaults are a clean baseline: no warnings at all.
    expect(shortcuts.issues()).toEqual([]);
  });

  it('editor: assigning a rimloc-conflicting combo is BLOCKED, binding unchanged', () => {
    mountCmp(ShortcutsEditor);
    // Move "save" away from its default first, then try to take search's combo.
    shortcuts.assign('save', { key: 'j', mod: true });
    click('shortcuts.remap.save');
    expect(exists('shortcuts.listening.save')).toBe(true);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', ctrlKey: true }));
    flushSync();
    expect(exists('shortcuts.blocked.save')).toBe(true);
    expect(shortcuts.binding('save').key).toBe('j'); // NOT saved
  });

  it('editor: system-reserved combo is saved with a warning, never blocked', () => {
    mountCmp(ShortcutsEditor);
    click('shortcuts.remap.palette');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'q', ctrlKey: true }));
    flushSync();
    expect(shortcuts.binding('palette').key).toBe('q'); // saved
    expect(exists('shortcuts.blocked.palette')).toBe(false);
    expect(exists('shortcuts.reserved.palette')).toBe(true);
    expect(q('shortcuts.warnReserved').textContent).toContain('system-reserved');
  });

  it('editor: convention-shadowing combo is saved with an info flag', () => {
    mountCmp(ShortcutsEditor);
    // Move "save" off Ctrl+S so taking it on nextIssue is a convention
    // shadow, not a RimLoc conflict.
    shortcuts.assign('save', { key: 'j', mod: true });
    click('shortcuts.remap.nextIssue');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }));
    flushSync();
    expect(shortcuts.binding('nextIssue').key).toBe('s'); // saved
    expect(exists('shortcuts.convention.nextIssue')).toBe(true);
    expect(q('shortcuts.infoConvention').textContent).toContain('shadow common app conventions');
  });

  it('every RimLoc command has a default and every default is clean', () => {
    for (const def of SHORTCUT_DEFS) {
      // W7: source commands ship UNBOUND ({ key: '' }) — no new fixed
      // shortcuts; classification of an unbound binding is clean by design.
      if (def.defaultBinding.key !== '') {
        expect(def.defaultBinding.key).toBeTruthy();
      }
      expect(classifyBinding(def.id, { ...def.defaultBinding }, shortcuts.bindings)).toBeNull();
    }
  });
});

describe('w45: i18n ru/en parity for the W4.5 additions', () => {
  it('every W4.5 key exists in both dictionaries', () => {
    for (const key of NEW_KEYS) {
      expect(en[key], `en missing: ${key}`).toBeTruthy();
      expect(ru[key], `ru missing: ${key}`).toBeTruthy();
    }
  });

  it('mutability vocabulary is fully covered by i18n', () => {
    for (const m of MUTABILITY_VALUES) {
      expect(en[`mutability.${m}`]).toBeTruthy();
      expect(ru[`mutability.${m}`]).toBeTruthy();
    }
  });
});
