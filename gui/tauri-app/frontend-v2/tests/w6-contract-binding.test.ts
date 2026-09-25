// W-built regression: the project store bound to the REAL RimLocClient
// (mock transport under test — same DTOs, same typed errors, same
// lost-update semantics as the Tauri bridge).
//   - contract create maps the canonical snapshot onto workspace entries;
//   - an edit flows through apply_intents and bumps the revision;
//   - a stale revision keeps the dirty draft and mutates nothing;
//   - reopen refreshes the epoch and works again;
//   - the fixture multi-target switcher is locked in contract mode.
import { beforeEach, describe, expect, it } from 'vitest';
import { project } from '../src/lib/stores/project.svelte';
import { languages } from '../src/lib/languages/store.svelte';
import { review } from '../src/lib/stores/review.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';

describe('project store on the contract client', () => {
  beforeEach(() => {
    project.reset();
    review.resetSession();
    buildState.reset();
  });

  it('createContractProject maps the snapshot onto workspace entries', async () => {
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');
    expect(project.projectName).toMatch(/^mock-\d{4}$/);
    expect(project.projectName).toBe(project.contractProjectId);
    expect(project.entries.length).toBe(4);
    const translated = project.byId('Keyed:MessageLetterArrived');
    expect(translated?.source).toBe('{0}: A letter has arrived.');
    expect(translated?.target).toBe('{0}: Пришло письмо.');
    expect(translated?.status).toBe('translated');
    const untranslated = project.byId('Keyed:AncientComplexWarning');
    expect(untranslated?.status).toBe('untranslated');
    expect(project.contractProjectId).toMatch(/^mock-\d{4}$/);
  });

  it('an edit flows through apply_intents and bumps the revision', async () => {
    await project.createContractProject('/mods/Demo');
    const revBefore = project.contractRevision;
    project.setDraft('Keyed:AncientComplexWarning', 'Предупреждение: древний комплекс нестабилен.');
    const saved = await project.flushDraft('keyed-04-placeholder'); // wrong id → false guard
    expect(saved).toBe(false);
    const ok = await project.flushDraft('Keyed:AncientComplexWarning');
    expect(ok).toBe(true);
    expect(project.byId('Keyed:AncientComplexWarning')?.target).toBe('Предупреждение: древний комплекс нестабилен.');
    expect(project.contractRevision).toBe(revBefore + 1);
  });

  it('a stale revision keeps the dirty draft and mutates nothing', async () => {
    await project.createContractProject('/mods/Demo');
    // External writer bumps the revision behind our back.
    const client = (project as unknown as { cc: () => { transport: { forceExternalRevision: (id: string, r: number) => void } } }).cc().transport as unknown as { forceExternalRevision: (id: string, r: number) => void };
    client.forceExternalRevision(project.contractProjectId!, 99);
    project.setDraft('Keyed:AncientComplexWarning', 'черновик-который-не-пролезет');
    const ok = await project.flushDraft('Keyed:AncientComplexWarning');
    expect(ok).toBe(false); // typed stale_revision → persist-before-ack
    expect(project.byId('Keyed:AncientComplexWarning')?.target).toBe(''); // nothing written
    expect(project.saveStates['Keyed:AncientComplexWarning']).toBe('dirty'); // draft kept
  });

  it('reopen refreshes the session epoch and works again', async () => {
    await project.createContractProject('/mods/Demo');
    const epochBefore = project.contractEpoch;
    const ok = await project.openContractProject(project.contractProjectId!);
    expect(ok).toBe(true);
    expect(project.contractEpoch).toBeGreaterThan(epochBefore);
    project.setDraft('Keyed:AncientComplexWarning', 'Снова правка');
    expect(await project.flushDraft('Keyed:AncientComplexWarning')).toBe(true);
  });

  it('listContractProjects exposes the real recent list', async () => {
    await project.createContractProject('/mods/Demo');
    const list = await project.listContractProjects();
    expect(list.length).toBeGreaterThanOrEqual(2);
  });

  it('typed DefInjected edit carries the FULL structural id (P1)', async () => {
    await project.createContractProject('/mods/Demo');
    const id = 'DefInjected:Gun_AssaultRifle.label:Weapons';
    const typed = project.byId(id);
    expect(typed).toBeDefined(); // def_type survived the snapshot mapping
    expect(typed?.source).toBe('assault rifle');
    expect(project.contractIdentities[id]?.def_type).toBe('Weapons');
    project.setDraft(id, 'штурмовая винтовка (правка)');
    // The mock skips intents whose identity lacks def_type — success proves
    // the discriminator rode along.
    expect(await project.flushDraft(id)).toBe(true);
    expect(project.byId(id)?.target).toBe('штурмовая винтовка (правка)');
  });

  it('untouched Keyed ids keep working without def_type', async () => {
    await project.createContractProject('/mods/Demo');
    project.setDraft('Keyed:MessageLetterArrived', '{0}: Пришло письмо (правка).');
    expect(await project.flushDraft('Keyed:MessageLetterArrived')).toBe(true);
    expect(project.contractIdentities['Keyed:MessageLetterArrived']?.def_type).toBeUndefined();
  });

  it('the fixture locale switcher is locked in contract mode', async () => {
    await project.createContractProject('/mods/Demo');
    languages.setActive('uk');
    expect(languages.activeLocale).toBe('ru');
  });
});
