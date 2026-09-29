// MockTransport — deterministic representative corpus (UI_SDK mandate):
// first run / normal mod / revision conflict. Driven by the SAME method
// names and DTO shapes as TauriTransport; failures are REAL typed contract
// errors, never fake successes. Session epoch/revision semantics mirror the
// Rust session manager so client code cannot tell the transports apart.
import type {
  ApplyExistingRequestDto,
  ApplyIntentsRequestDto,
  CreateProjectRequestDto,
  ApplyIntentsResponseDto,
  ContractErrorDto,
  ContractErrorCode,
  ContractHandshakeDto,
  ContractMethod,
  ExistingAmbiguousItemDto,
  ExistingMatchItemDto,
  ImportExistingRequestDto,
  ProjectSnapshotDto,
  ProjectSummaryDto,
  TranslationIntentDto,
  ValidateProjectResponseDto
} from './types';
import type { ContractMethodMap, RimLocTransport } from './transport';
import { looksAbsolutePath } from '../paths';

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
  /** Wire values mirroring the canonical Origin (serde snake_case). */
  origin?: 'unknown' | 'human' | 'tm' | 'llm' | 'imported';
  /** Wire values mirroring the canonical ValidationState. */
  validation?: 'unknown' | 'ok' | { issues: string[] };
}

interface MockProject {
  project_id: string;
  name: string;
  revision: number;
  session_epoch: number;
  /** Source root the project was created from (export guard tooth). */
  mod_root?: string;
  /** Test hook state: the managed file changed behind our back — the next
   *  apply refuses with `project_changed_on_disk` until refresh adopts the
   *  disk (mirrors the Rust disk-hash gate, session.rs). */
  disk_dirty?: boolean;
  entries: { id: { kind: string; key: string; def_type?: string }, text: string }[];
  translations: MockTranslation[];
}

function entryId(kind: string, key: string) {
  return { kind, key };
}

/** Mirror of the Rust apply_intent severity rule (session.rs): a suspicious
 *  placeholder classifies the stored validation as Issues, else Ok. */
function validationFor(text: string | null): 'unknown' | 'ok' | { issues: string[] } {
  if (text === null) return 'ok';
  // single % not part of a known token ({n} / %% / %letter)
  const bad = /%(?!%|[a-zA-Z]|\{|\d+\$)/.test(text);
  return bad
    ? { issues: ['suspicious placeholder (single % not part of a known token)'] }
    : 'ok';
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
      // Audit P1-1 teeth: the corpus carries REAL provenance/validation so
      // the snapshot mapping (origin → filter, validation → badges) is
      // exercised against the same wire values the Rust bridge emits.
      { id: entryId('Keyed', 'MessageLetterArrived'), locale: 'Russian', text: '{0}: Пришло письмо.', completeness: 'translated', origin: 'imported', validation: 'ok' },
      { id: entryId('Keyed', 'AncientComplexWarning'), locale: 'Russian', text: null, completeness: 'untranslated', origin: 'unknown', validation: 'unknown' },
      // Deliberate tooth: issues classification on a stored translation.
      { id: { kind: 'DefInjected', key: 'Gun_AssaultRifle.label', def_type: 'Weapons' }, locale: 'Russian', text: 'штурмовая винтовка', completeness: 'translated', origin: 'human', validation: { issues: ['suspicious placeholder (single % not part of a known token)'] } },
      { id: entryId('DefInjected', 'MeleeWeapon_LongSword.label'), locale: 'Russian', text: null, completeness: 'untranslated', origin: 'unknown', validation: 'unknown' }
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

/** ONE snapshot builder for every read method — mirrors snapshot_of on the
 *  Rust side: provenance/validation wire values + the additive v2 fields
 *  (dirty / acked_revision). */
function snapshotOf(p: MockProject): ProjectSnapshotDto {
  return {
    project_id: p.project_id,
    revision: p.revision,
    session_epoch: p.session_epoch,
    // v2 wire: an external disk change leaves the session dirty until
    // refresh adopts the disk (Rust: applied-but-unacked semantics).
    dirty: p.disk_dirty === true,
    acked_revision: p.revision,
    project: {
      context: { active_dlc: [], active_mods: [], load_order: [], view: 'potential' },
      entries: p.entries.map((e) => ({ id: e.id, text: e.text, source_locale: 'en' })),
      translations: p.translations.map((t) => ({
        source_id: t.id,
        locale: t.locale,
        text: t.text,
        completeness: t.completeness,
        review: 'none',
        validation: t.validation ?? 'unknown',
        lifecycle: 'active',
        origin: t.origin ?? 'human'
      }))
    }
  };
}

export function createMockTransport(state = createMockState()): RimLocTransport & {
  /** Test hook: bump the persisted revision behind the client's back. */
  forceExternalRevision(projectId: string, revision: number): void;
  /** Test hook: simulate an external disk change — the next apply refuses
   *  with `project_changed_on_disk` until refresh adopts the disk. */
  forceExternalDiskChange(projectId: string): void;
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
                'job_cancel',
                // Final night wave: mirrors the Rust capability_report
                // (contract.rs) — validate/build/diagnostics are supported.
                'project_validate',
                'project_build_export',
                'project_build_mod',
                'project_diagnostics_bundle',
                // Selfloc entry (mandate D): mirrors contract.rs
                // Capability::SelflocCatalog — the shell-level resolver IS a
                // supported slice op, not an unsupported approximation.
                'selfloc_catalog',
                // W2 (existing-pack flow): mirrors contract.rs
                // Capability::ProjectImportExisting / ProjectApplyExisting.
                'project_import_existing',
                'project_apply_existing'
              ],
              unsupported: [
                // Audit P2-1: names mirror the Rust capability_report
                // (crates/rimloc-services/src/contract.rs) EXACTLY — the
                // stale *_via_contract aliases are gone, and `import_pack`
                // moved to the two live W2 entries above.
                { capability: 'source_inspector_actions', reason: 'next slice: identity-based source actions with size-limited reads' },
                { capability: 'providers_settings', reason: 'later slice: provider/settings parity' },
                { capability: 'entry_create_delete', reason: 'intents cover translation edits only; identities come from rescan' }
              ]
            }
          };
          return result as ContractMethodMap[M]['result'];
        }
        case 'project_create': {
          const req = (params as { request: CreateProjectRequestDto }).request;
          const id = `mock-${String(state.projects.length + 1).padStart(4, '0')}`;
          const p = mkProject(id, req.mod_root.path.split('/').pop() ?? id);
          p.mod_root = req.mod_root.path; // export-guard tooth
          state.projects.push(p);
          const snap: ProjectSnapshotDto = snapshotOf(p);
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_open': {
          const p = find((params as { project_id: string }).project_id);
          p.session_epoch += 1;
          const snap: ProjectSnapshotDto = snapshotOf(p);
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
          const snap: ProjectSnapshotDto = snapshotOf(p);
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_apply_intents': {
          const req = (params as { request: ApplyIntentsRequestDto }).request;
          const p = find(req.project_id);
          // Mirror of the Rust disk-hash gate: an external change refuses
          // the apply until refresh adopts the disk (typed, not silent).
          if (p.disk_dirty) {
            throw new MockContractError(
              'project_changed_on_disk',
              'the managed file changed on disk since your last read'
            );
          }
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
            // Audit P1-1 mirror of session.rs apply_intent: EVERY applied
            // intent re-stamps provenance as Human and recomputes the
            // stored validation from the resulting text.
            t.origin = 'human';
            t.validation = validationFor(t.text);
            applied += 1;
          });
          // Rust invariant (session.rs applied == 0): NOTHING acked changes
          // when every intent was refused — no revision bump, no save, no
          // dirty state; the response echoes the CURRENT revision.
          if (applied === 0) {
            return {
              job_id: `mock-job-${p.revision}`,
              revision: p.revision,
              applied: 0,
              skipped,
              cancelled: false
            } as ContractMethodMap[M]['result'];
          }
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
          // Refresh adopts the DISK: the external change is accepted and the
          // session is no longer dirty (Rust: disk wins, dirty discarded).
          p.disk_dirty = false;
          p.session_epoch += 1;
          const snap: ProjectSnapshotDto = snapshotOf(p);
          return snap as ContractMethodMap[M]['result'];
        }
        case 'project_cancel_next': {
          return true as ContractMethodMap[M]['result'];
        }
        // --- final night wave: validate/build/diagnostics -------------------
        // Scenario rules mirror the real semantics: findings derive from the
        // STORED state (never invented), refusals are typed errors.
        case 'project_validate': {
          const { project_id, session_epoch, locale } = params as {
            project_id: string;
            session_epoch: number;
            locale?: string;
          };
          const p = find(project_id);
          if (p.session_epoch !== session_epoch) {
            throw new MockContractError('stale_epoch', 'a newer session opened this project', {
              expected: p.session_epoch
            });
          }
          const findings: ValidateProjectResponseDto['findings'] = [];
          for (const t of p.translations) {
            if (!(t.locale ?? '').toLowerCase().startsWith('ru')) continue;
            const entry = p.entries.find(
              (e) => e.id.kind === t.id.kind && e.id.key === t.id.key && (e.id.def_type ?? undefined) === (t.id.def_type ?? undefined)
            );
            const identity = { id: t.id, path: entry ? `Defs/${t.id.kind}.xml` : '' };
            if (t.validation && typeof t.validation === 'object') {
              findings.push({
                ...identity,
                severity: 'error',
                kind: 'placeholder',
                key: t.id.key,
                message: t.validation.issues[0] ?? 'validation issue'
              });
            } else if (t.completeness === 'untranslated') {
              findings.push({
                ...identity,
                severity: 'warning',
                kind: 'untranslated',
                key: t.id.key,
                message: `not translated yet (${locale ?? 'all locales'})`
              });
            } else if (t.completeness === 'todo') {
              findings.push({
                ...identity,
                severity: 'warning',
                kind: 'todo',
                key: t.id.key,
                message: 'literal TODO placeholder counts as missing'
              });
            }
          }
          const error_count = findings.filter((f) => f.severity === 'error').length;
          const warning_count = findings.filter((f) => f.severity === 'warning').length;
          const info_count = findings.filter((f) => f.severity === 'info').length;
          p.revision += 0; // validate NEVER mutates (read-only)
          return {
            job_id: `mock-validate-${p.revision}`,
            status: error_count > 0 ? 'failed' : 'succeeded',
            findings,
            error_count,
            warning_count,
            info_count,
            ...(locale ? { locale } : {})
          } as ContractMethodMap[M]['result'];
        }
        case 'project_export': {
          const { project_id, session_epoch, out_dir, locale } = params as {
            project_id: string;
            session_epoch: number;
            out_dir: string;
            locale: string;
          };
          const p = find(project_id);
          if (p.session_epoch !== session_epoch) {
            throw new MockContractError('stale_epoch', 'a newer session opened this project', {
              expected: p.session_epoch
            });
          }
          // Mirror of the strict language-folder form guard (session P1-2):
          // the locale joins output paths, so 'ru' must be refused — the
          // caller sends the FOLDER form ('Russian').
          if (!/^[A-Z][A-Za-z]*$/.test(locale)) {
            throw new MockContractError(
              'contract_violation',
              `locale \`${locale}\` is not the strict language-folder form (e.g. Russian)`
            );
          }
          // Mirror of the path-FORM guard (invalid_output_path): a relative
          // out dir would land wherever the app was launched from — refused
          // BEFORE any containment check or write.
          if (!looksAbsolutePath(out_dir)) {
            throw new MockContractError(
              'invalid_output_path',
              `output path \`${out_dir}\` is not absolute; specify an absolute output directory`
            );
          }
          // Mirror of the source-tree guard: an out dir inside the project's
          // own mod root is a denied self-overwrite (guard_output_denied).
          if (p.mod_root && (out_dir === p.mod_root || out_dir.startsWith(p.mod_root + '/'))) {
            throw new MockContractError(
              'guard_output_denied',
              'the output directory sits inside the read-only source tree'
            );
          }
          const written = p.translations.filter(
            (t) => t.text !== null && t.text !== '' && (t.locale ?? '').toLowerCase().startsWith('ru')
          );
          // Writer tooth: DefInjected entries with an UNKNOWN def type are
          // skipped and surfaced for review (delta risk #5).
          const skipped_unknown_type = p.entries
            .filter((e) => e.id.kind === 'DefInjected' && !e.id.def_type)
            .map((e) => e.id.key);
          return {
            job_id: `mock-export-${p.revision}`,
            out_dir: { path: out_dir },
            files_written: written.length,
            reparsed_keys: written.length, // reparse parity before ack
            skipped_unknown_type
          } as ContractMethodMap[M]['result'];
        }
        case 'project_build_mod': {
          const { project_id, session_epoch, out_dir, locale } = params as {
            project_id: string;
            session_epoch: number;
            out_dir: string;
            locale: string;
          };
          const p = find(project_id);
          // Guard partition MIRRORS the Rust build_mod_project (a copy of
          // the export partition): stale epoch → locale form → absolute
          // form → source-tree deny. Same typed codes, same order.
          if (p.session_epoch !== session_epoch) {
            throw new MockContractError('stale_epoch', 'a newer session opened this project', {
              expected: p.session_epoch
            });
          }
          if (!/^[A-Z][A-Za-z]*$/.test(locale)) {
            throw new MockContractError(
              'contract_violation',
              `locale \`${locale}\` is not the strict language-folder form (e.g. Russian)`
            );
          }
          if (!looksAbsolutePath(out_dir)) {
            throw new MockContractError(
              'invalid_output_path',
              `output path \`${out_dir}\` is not absolute; specify an absolute output directory`
            );
          }
          if (p.mod_root && (out_dir === p.mod_root || out_dir.startsWith(p.mod_root + '/'))) {
            throw new MockContractError(
              'guard_output_denied',
              'the output directory sits inside the read-only source tree'
            );
          }
          // Same acceptance accounting as the export: the translated rows
          // are the keys; the About/About.xml (<ModMetaData>) rides on top.
          const written = p.translations.filter(
            (t) => t.text !== null && t.text !== '' && (t.locale ?? '').toLowerCase().startsWith('ru')
          );
          const skipped_unknown_type = p.entries
            .filter((e) => e.id.kind === 'DefInjected' && !e.id.def_type)
            .map((e) => e.id.key);
          return {
            job_id: `mock-buildmod-${p.revision}`,
            out_dir: { path: out_dir },
            files_written: written.length + 1, // + About/About.xml
            reparsed_keys: written.length, // reparse parity before ack
            skipped_unknown_type
          } as ContractMethodMap[M]['result'];
        }
        case 'project_diagnose': {
          const { project_id, out_dir } = params as { project_id: string; out_dir: string };
          const p = find(project_id);
          // Mirror of the path-FORM guard (invalid_output_path), first on
          // the entry exactly like the Rust diagnose.
          if (!looksAbsolutePath(out_dir)) {
            throw new MockContractError(
              'invalid_output_path',
              `output path \`${out_dir}\` is not absolute; specify an absolute output directory`
            );
          }
          return {
            job_id: `mock-diagnose-${p.revision}`,
            bundle_dir: { path: `${out_dir}/rimloc-bundle-${p.revision}` },
            operation_id: `mock-op-${p.revision}`,
            files: ['operation.json', 'affected.json', 'environment.json'],
            redacted_count: 3,
            excluded_count: 1
          } as ContractMethodMap[M]['result'];
        }
        // --- W2: existing translation pack (analyze + apply) ---------------
        // Mock rules: mock mode cannot read the user's filesystem, so the
        // "pack" is a DETERMINISTIC synthetic line set derived from the
        // corpus itself — untranslated entries are reusable, translated ones
        // conflicts — plus fixed obsolete / invalid / ambiguous lines. The
        // guards (epoch, revision, path form, locale form) mirror the Rust
        // session so client code cannot tell the transports apart.
        case 'project_import_existing':
        case 'project_apply_existing': {
          const isApply = method === 'project_apply_existing';
          const req = (
            params as {
              request: ImportExistingRequestDto & Partial<ApplyExistingRequestDto>;
            }
          ).request;
          const p = find(req.project_id);
          if (p.session_epoch !== req.session_epoch) {
            throw new MockContractError('stale_epoch', 'a newer session opened this project', {
              expected: p.session_epoch
            });
          }
          if (isApply && req.expected_revision !== undefined && req.expected_revision !== p.revision) {
            throw new MockContractError('stale_revision', 'the project moved on since your edits', {
              expected: p.revision
            });
          }
          // Mirror of the path-FORM guard (invalid_output_path), first on
          // the entry exactly like the Rust session.
          if (!looksAbsolutePath(req.existing_dir.path)) {
            throw new MockContractError(
              'invalid_output_path',
              `existing translation directory \`${req.existing_dir.path}\` is not absolute; specify an absolute directory`
            );
          }
          // Mirror of the strict language-folder form guard (locale joins
          // durable translation records on apply) — same charset as the
          // Rust util::lang_dir_form_ok: letters, digits, `_`, `-`.
          if (!/^[A-Za-z0-9_-]+$/.test(req.locale)) {
            throw new MockContractError(
              'contract_violation',
              `locale \`${req.locale}\` is not the strict language-folder form (letters, digits, \`_\`, \`-\`)`
            );
          }
          // Mirror of the pack↔locale cross-check: the scan walks ANY
          // */Languages/<Any> under the given root, so the chosen folder
          // must BE the language folder (leaf == locale, case-insensitive)
          // — otherwise another language's source text would classify as
          // reusable.
          const packLeaf =
            req.existing_dir.path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? '';
          if (packLeaf.toLowerCase() !== req.locale.toLowerCase()) {
            throw new MockContractError(
              'contract_violation',
              `existing translation directory \`${req.existing_dir.path}\` does not match the locale \`${req.locale}\`: the pack folder must be the language folder itself (…/Languages/${req.locale})`
            );
          }
          // Deterministic synthetic pack derived from the corpus.
          const reusable: ExistingMatchItemDto[] = [];
          const conflicts: ExistingMatchItemDto[] = [];
          for (const tr of p.translations) {
            if ((tr.locale ?? '') !== req.locale) continue;
            const item: ExistingMatchItemDto = {
              key: tr.id.key,
              entry: { kind: tr.id.kind, key: tr.id.key, ...(tr.id.def_type ? { def_type: tr.id.def_type } : {}) }
            };
            if (tr.text !== null && tr.text.trim() !== '') {
              conflicts.push(item);
            } else {
              reusable.push(item);
            }
          }
          const obsolete: ExistingMatchItemDto[] = [
            { key: 'OldGearLabel' },
            { key: 'RemovedLegacyTip' }
          ];
          const invalid: ExistingMatchItemDto[] = [{ key: 'EmptyLegacyLine' }];
          const ambiguous: ExistingAmbiguousItemDto[] = [
            { key: 'DualScopeKey', candidates: ['DualScopeKey', 'DualScopeKey.base'] }
          ];

          if (!isApply) {
            return {
              job_id: `mock-import-${p.revision}`,
              scanned_files: 2,
              scanned_keys: reusable.length + conflicts.length + obsolete.length + invalid.length + ambiguous.length,
              reusable_count: reusable.length,
              conflict_count: conflicts.length,
              obsolete_count: obsolete.length,
              ambiguous_count: ambiguous.length,
              invalid_count: invalid.length,
              // The synthetic pack covers the whole corpus — nothing stays
              // uncovered (the Rust analyzer reports real gaps).
              new_count: 0,
              reusable,
              conflicts,
              obsolete,
              ambiguous,
              invalid
            } as ContractMethodMap[M]['result'];
          }

          // Apply moves ONLY the reusable set; conflicts keep their text.
          const applied = reusable.length;
          if (applied === 0) {
            return {
              job_id: `mock-apply-${p.revision}`,
              revision: p.revision,
              applied: 0,
              conflicts: conflicts.length,
              unmatched: obsolete.length,
              ambiguous: ambiguous.length
            } as ContractMethodMap[M]['result'];
          }
          for (const item of reusable) {
            const tr = p.translations.find(
              (x) =>
                x.id.kind === item.entry!.kind &&
                x.id.key === item.entry!.key &&
                x.locale === req.locale &&
                (x.id.def_type ?? undefined) === item.entry!.def_type
            );
            if (!tr) continue;
            tr.text = `импорт: ${tr.id.key}`;
            tr.completeness = 'translated';
            tr.origin = 'imported';
            tr.validation = 'ok';
          }
          p.revision += 1;
          return {
            job_id: `mock-apply-${p.revision}`,
            revision: p.revision,
            applied,
            conflicts: conflicts.length,
            unmatched: obsolete.length,
            ambiguous: ambiguous.length
          } as ContractMethodMap[M]['result'];
        }
        case 'pick_directory': {
          // Honest refusal (never a fake dialog): there is no OS folder
          // dialog without the desktop bridge. The desktop app answers the
          // REAL native picker (main.rs pick_directory → blocking_pick_folder).
          throw new MockContractError(
            'unsupported_capability',
            'pick_directory: the native folder dialog is unavailable in mock — run the desktop app'
          );
        }
        case 'selfloc_catalog_dir': {
          // Honest refusal (never a fabricated catalog dir): there is no
          // bundled RimLoc UI catalog without the desktop bridge, and a fake
          // path would dead-end project_create — the entry point must not
          // pretend success in dev/demo mode.
          throw new MockContractError(
            'unsupported_capability',
            'selfloc_catalog_dir: the bundled RimLoc UI catalog is unavailable in mock — run the desktop app'
          );
        }
        case 'selfloc_build_contribution': {
          // Honest refusal (never a fake bundle): building the contribution
          // runs the §6 gate over a REAL backend session and WRITES a file —
          // neither exists in mock, and a fabricated READY result would lie
          // about a bundle that was never produced.
          throw new MockContractError(
            'unsupported_capability',
            'selfloc_build_contribution: the contribution builder is unavailable in mock — run the desktop app'
          );
        }
      }
    },
    forceExternalRevision(projectId: string, revision: number) {
      find(projectId).revision = revision;
    },
    forceExternalDiskChange(projectId: string) {
      find(projectId).disk_dirty = true;
    }
  };
}
