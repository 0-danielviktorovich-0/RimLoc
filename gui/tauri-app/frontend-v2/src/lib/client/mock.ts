// MockTransport — deterministic representative corpus (UI_SDK mandate):
// first run / normal mod / revision conflict. Driven by the SAME method
// names and DTO shapes as TauriTransport; failures are REAL typed contract
// errors, never fake successes. Session epoch/revision semantics mirror the
// Rust session manager so client code cannot tell the transports apart.
import type {
  ApplyIntentsRequestDto,
  CreateProjectRequestDto,
  ApplyIntentsResponseDto,
  ContractErrorDto,
  ContractErrorCode,
  ContractHandshakeDto,
  ContractMethod,
  ProjectSnapshotDto,
  ProjectSummaryDto,
  TranslationIntentDto
} from './types';
import type { ContractMethodMap, RimLocTransport } from './transport';

export class MockContractError extends Error implements ContractErrorDto {
  readonly code: ContractErrorCode;
  readonly details?: unknown;
  constructor(code: ContractErrorCode, message: string, details?: unknown) {
    super(message);
    this.code = code;
    this.details = details;
  }
}

interface MockTranslation {
  id: { kind: string; key: string; def_type?: string };
  locale: string;
  text: string | null;
  completeness: 'untranslated' | 'todo' | 'translated';
}

interface MockProject {
  project_id: string;
  name: string;
  revision: number;
  session_epoch: number;
  entries: { id: { kind: string; key: string; def_type?: string }, text: string }[];
  translations: MockTranslation[];
}

function entryId(kind: string, key: string) {
  return { kind, key };
}

function mkProject(id: string, name: string): MockProject {
  return {
    project_id: id,
    name,
    revision: 1,
    session_epoch: 1,
    entries: [
      { id: entryId('Keyed', 'MessageLetterArrived'), text: '{0}: A letter has arrived.' },
      { id: entryId('Keyed', 'AncientComplexWarning'), text: 'Warning: the ancient complex is unstable.' },
      // P1 regression tooth: this entry carries a def_type discriminator —
      // intents without the FULL structural identity are skipped, matching
      // the real backend's strict structural resolution.
      { id: { kind: 'DefInjected', key: 'Gun_AssaultRifle.label', def_type: 'Weapons' }, text: 'assault rifle' },
      { id: entryId('DefInjected', 'MeleeWeapon_LongSword.label'), text: 'longsword' }
    ],
    translations: [
      { id: entryId('Keyed', 'MessageLetterArrived'), locale: 'Russian', text: '{0}: Пришло письмо.', completeness: 'translated' },
      { id: entryId('Keyed', 'AncientComplexWarning'), locale: 'Russian', text: null, completeness: 'untranslated' },
      { id: { kind: 'DefInjected', key: 'Gun_AssaultRifle.label', def_type: 'Weapons' }, locale: 'Russian', text: 'штурмовая винтовка', completeness: 'translated' },
      { id: entryId('DefInjected', 'MeleeWeapon_LongSword.label'), locale: 'Russian', text: null, completeness: 'untranslated' }
    ]
  };
}

/** Deterministic corpus (UI_SDK representative slice). */
export function createMockState() {
  const projects: MockProject[] = [
    mkProject('mock-demo-0001', 'Demo TestMod'),
    mkProject('mock-normal-0002', 'My Mod')
  ];
  return { projects };
}

export function createMockTransport(state = createMockState()): RimLocTransport & {
  /** Test hook: bump the persisted revision behind the client's back. */
  forceExternalRevision(projectId: string, revision: number): void;
} {
  const find = (id: string): MockProject => {
    const p = state.projects.find((x) => x.project_id === id);
    if (!p) throw new MockContractError('project_not_found', `no managed project ${id}`);
    return p;
  };

  return {
    mode: 'mock',
    async call<M extends ContractMethod>(
      method: M,
      params: ContractMethodMap[M]['params']
    ): Promise<ContractMethodMap[M]['result']> {
      switch (method) {
        case 'contract_handshake': {
          const result: ContractHandshakeDto = {
            ui_contract_version: 1,
            capabilities: {
              contract_version: 1,
              supported: [
                'project_create',
                'project_open',
                'project_list',
                'project_snapshot',
                'project_apply_intents',
                'project_refresh',
                'job_cancel'
              ],
              unsupported: [
                { capability: 'validate_via_contract', reason: 'next slice' },
                { capability: 'build_via_contract', reason: 'next slice' },
                { capability: 'diagnostics_via_contract', reason: 'next slice' }
              ]
            }
          };
          return result as ContractMethodMap[M]['result'];
        }
        case 'project_create': {
          const req = (params as { request: CreateProjectRequestDto }).request;
          const id = `mock-${String(state.projects.length + 1).padStart(4, '0')}`;
          const p = mkProject(id, req.mod_root.path.split('/').pop() ?? id);
          state.projects.push(p);
          const snap: ProjectSnapshotDto = {
            project_id: p.project_id,
            revision: p.revision,
            session_epoch: p.session_epoch,
            project: { context: { active_dlc: [], active_mods: [], load_order: [], view: 'potential' }, entries: p.entries.map((e) => ({ id: e.id, text: e.text, source_locale: 'en' })), translations: p.translations.map((t) => ({ source_id: t.id, locale: t.locale, text: t.text, completeness: t.completeness, review: 'none', validation: 'unknown', lifecycle: 'active', origin: 'human' })) }
          };
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_open': {
          const p = find((params as { project_id: string }).project_id);
          p.session_epoch += 1;
          const snap: ProjectSnapshotDto = {
            project_id: p.project_id,
            revision: p.revision,
            session_epoch: p.session_epoch,
            project: { context: { active_dlc: [], active_mods: [], load_order: [], view: 'potential' }, entries: p.entries.map((e) => ({ id: e.id, text: e.text, source_locale: 'en' })), translations: p.translations.map((t) => ({ source_id: t.id, locale: t.locale, text: t.text, completeness: t.completeness, review: 'none', validation: 'unknown', lifecycle: 'active', origin: 'human' })) }
          };
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_list': {
          const result: ProjectSummaryDto[] = state.projects.map((p) => ({
            project_id: p.project_id,
            name: p.name,
            revision: p.revision
          }));
          return result as ContractMethodMap[M]['result'];
        }
        case 'project_snapshot': {
          const p = find((params as { project_id: string }).project_id);
          const snap: ProjectSnapshotDto = {
            project_id: p.project_id,
            revision: p.revision,
            session_epoch: p.session_epoch,
            project: { context: { active_dlc: [], active_mods: [], load_order: [], view: 'potential' }, entries: p.entries.map((e) => ({ id: e.id, text: e.text, source_locale: 'en' })), translations: p.translations.map((t) => ({ source_id: t.id, locale: t.locale, text: t.text, completeness: t.completeness, review: 'none', validation: 'unknown', lifecycle: 'active', origin: 'human' })) }
          };
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_apply_intents': {
          const req = (params as { request: ApplyIntentsRequestDto }).request;
          const p = find(req.project_id);
          if (req.session_epoch !== p.session_epoch) {
            throw new MockContractError('stale_epoch', 'a newer session opened this project', { expected: p.session_epoch });
          }
          if (req.expected_revision !== p.revision) {
            throw new MockContractError('stale_revision', 'the project moved on since your edits', { expected: p.revision });
          }
          const skipped: ApplyIntentsResponseDto['skipped'] = [];
          let applied = 0;
          req.intents.forEach((intent: TranslationIntentDto, index: number) => {
            const t = p.translations.find(
              (x) =>
                x.id.kind === intent.entry.kind &&
                x.id.key === intent.entry.key &&
                x.locale === intent.locale &&
                (x.id.def_type ?? undefined) === intent.entry.def_type
            );
            if (!t) {
              skipped.push({ index, code: 'contract_violation', message: `unknown identity ${intent.entry.kind}:${intent.entry.key}` });
              return;
            }
            if (intent.action === 'set_translation') {
              t.text = intent.text ?? '';
              t.completeness = t.text.trim() === '' ? 'untranslated' : 'translated';
            } else if (intent.action === 'mark_todo') {
              t.text = 'TODO';
              t.completeness = 'todo';
            } else {
              t.text = null;
              t.completeness = 'untranslated';
            }
            applied += 1;
          });
          p.revision += 1;
          const result: ApplyIntentsResponseDto = {
            job_id: `mock-job-${p.revision}`,
            revision: p.revision,
            applied,
            skipped,
            cancelled: false
          };
          return result as ContractMethodMap[M]['result'];
        }
        case 'project_refresh': {
          const p = find((params as { project_id: string }).project_id);
          p.session_epoch += 1;
          const snap: ProjectSnapshotDto = {
            project_id: p.project_id,
            revision: p.revision,
            session_epoch: p.session_epoch,
            project: { context: { active_dlc: [], active_mods: [], load_order: [], view: 'potential' }, entries: p.entries.map((e) => ({ id: e.id, text: e.text, source_locale: 'en' })), translations: p.translations.map((t) => ({ source_id: t.id, locale: t.locale, text: t.text, completeness: t.completeness, review: 'none', validation: 'unknown', lifecycle: 'active', origin: 'human' })) }
          };
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_cancel_next': {
          return true as ContractMethodMap[M]['result'];
        }
      }
    },
    forceExternalRevision(projectId: string, revision: number) {
      find(projectId).revision = revision;
    }
  };
}
