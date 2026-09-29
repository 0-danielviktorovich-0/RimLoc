// Project store — the mock stand-in for the backend service boundary (spec §3.1).
// The only write operation mimics `update_translation(entry_id, locale, text)`;
// everything else is local reactive state. No Tauri/IPC calls anywhere.

import { mockEntries } from '../mock/data';
import type { Entry, EntryKind, EntryStatus, Origin, SaveState } from '../mock/types';
// W6/034: a project generation change aborts any in-flight mock build — a
// fresh project must not inherit a done/running build from the old one.
import { buildState } from '../mock/buildState.svelte';
// W-built: in 'tauri' mode the project lifecycle flows through the REAL
// RimLocClient (contract v1); the fixture dataset below is the explicit
// demo/dev mock (devMode) and is never a silent production default.
import { clientInstance } from '../client/instance.svelte';
import { ContractClientError, type RimLocClient } from '../client/client';
import { contractErrorText } from '../client/messages';
import type { ProjectSnapshotDto, ProjectSummaryDto, SourceEntryIdDto } from '../client/types';

export type StatusCounts = Record<EntryStatus, number>;

const AUTOSAVE_DEBOUNCE_MS = 800; // spec open question #2: 800ms debounce
const SAVING_INDICATOR_MS = 350; // simulated latency so the "saving…" badge is visible
const SAVED_INDICATOR_MS = 1200;

/** Audit P1-1: canonical Origin (serde snake_case on the wire) → the UI
 *  provenance vocabulary used by the workspace filter. */
const CONTRACT_ORIGIN_TO_UI: Record<string, Origin | undefined> = {
  human: 'human',
  tm: 'TM',
  llm: 'LLM',
  imported: 'imported',
  unknown: undefined
};

/** v1 decision (night handoff): the GUI pins the RimWorld target version at
 *  create until the version-resolution journey lands. The contract request
 *  already carries it (CreateProjectRequestDto.target_version) and the
 *  backend folds it into the project context via build_project. */
export const DEFAULT_TARGET_VERSION = '1.6';

function cloneInitial(): Entry[] {
  return structuredClone(mockEntries);
}

/** Pristine project name — restored by reset() so the demo/dev reset returns
 * the store to its exact starting identity. */
const DEFAULT_PROJECT_NAME = 'TestMod';

class ProjectStore {
  projectName = $state(DEFAULT_PROJECT_NAME);
  /**
   * M-5 (UI audit 2026-09-29): the HUMAN project name for headers — on a
   * contract project the wire snapshot carries only the service id
   * (`proj-…`), the display name rides `project_list` summaries
   * (ProjectSummaryDto.name ← session display_name in session.rs). Resolved
   * after every snapshot application; null = not (yet) known → the id shows.
   */
  projectDisplayName = $state<string | null>(null);
  targetLocale = $state('ru');
  /** True only while the workspace shows the bundled W6 demo project. */
  isDemo = $state(false);

  entries = $state<Entry[]>(cloneInitial());

  /** Draft text per entry id — survives leaving the cell, shown italic until commit. */
  drafts = $state<Record<string, string>>({});
  saveStates = $state<Record<string, SaveState>>({});

  selectedId = $state<string | null>(null);
  filters = $state<EntryStatus[]>([]);
  category = $state<'all' | EntryKind>('all');
  /** Provenance filter from the filter popover ('any' = no filter). */
  originFilter = $state<Origin | 'any'>('any');
  search = $state('');

  // --- W-built: contract binding (mode 'tauri') ---
  /** 'fixture' = the explicit demo/dev mock dataset; 'contract' = real
   * snapshots through RimLocClient. Chosen by the client mode, never silently. */
  source = $state<'fixture' | 'contract'>('fixture');
  contractProjectId = $state<string | null>(null);
  contractRevision = $state(0);
  contractEpoch = $state(0);
  contractError = $state<string | null>(null);
  /**
   * v2 wire (contract.rs): the last durably ACKED revision. While the
   * backend session is dirty, `apply` compares expected_revision against
   * THIS — revision alone can be ahead of the ack and get rejected as
   * stale_revision. Pass A 1.2.
   */
  contractAckedRevision = $state(0);
  /** True while a disk-adoption refresh is in flight (workspace banner). */
  refreshing = $state(false);
  /**
   * W-built (P1): full STRUCTURAL identities per workspace entry id. The
   * backend resolves intents by exact structural match (kind+key+def_type,
   * no key-fallback — lead 033), so the def_type discriminator must survive
   * the snapshot→workspace mapping and ride every intent.
   */
  private contractIdentities: Record<string, SourceEntryIdDto> = {};
  private rimloc: RimLocClient | null = null;

  private cc(): RimLocClient {
    if (!this.rimloc) this.rimloc = clientInstance.getClient();
    return this.rimloc;
  }

  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  /**
   * W6 (lead 034): staged-but-not-fired commit bookkeeping. A commit captures
   * the project generation and the entry's draft epoch; at fire time a stale
   * commit resolves false WITHOUT writing — so a pending demo save can never
   * land in a replaced project/locale and an older commit can never overwrite
   * a newer draft.
   */
  private generation = 0;
  private draftEpoch: Record<string, number> = {};
  private pendingCommits = new Map<
    string,
    { timer: ReturnType<typeof setTimeout>; resolve: (ok: boolean) => void; gen: number; epoch: number }
  >();

  byId(id: string): Entry | undefined {
    return this.entries.find((e) => e.id === id);
  }

  get selected(): Entry | null {
    return this.selectedId ? (this.byId(this.selectedId) ?? null) : null;
  }

  /** M-5: display name for headers — the human name when resolved, the raw
   * id otherwise (never an invented label). */
  get displayName(): string {
    return this.projectDisplayName ?? this.projectName;
  }

  statusCounts(): StatusCounts {
    const counts: StatusCounts = {
      untranslated: 0,
      translated: 0,
      todo: 0,
      sourceChanged: 0,
      pending_review: 0,
      orphan: 0
    };
    for (const e of this.entries) counts[e.status] += 1;
    return counts;
  }

  /** Mock of the service read: category + status multi-filter + origin + debounced search.
   * Dimensions combine with AND; statuses and kinds combine with OR within their dimension. */
  filtered(): Entry[] {
    const needle = this.search.trim().toLowerCase();
    return this.entries.filter((e) => {
      if (this.category !== 'all' && e.kind !== this.category) return false;
      if (this.filters.length > 0 && !this.filters.includes(e.status)) return false;
      if (this.originFilter !== 'any' && e.origin !== this.originFilter) return false;
      if (needle) {
        const hay = `${e.source}\n${e.target}\n${e.key}`.toLowerCase();
        if (!hay.includes(needle)) return false;
      }
      return true;
    });
  }

  select(id: string | null) {
    this.selectedId = id;
  }

  toggleFilter(status: EntryStatus) {
    this.filters = this.filters.includes(status)
      ? this.filters.filter((s) => s !== status)
      : [...this.filters, status];
  }

  /** Left-navigator single-select status view (user-oriented navigation, mandate §9). */
  setStatusFilter(status: EntryStatus | null) {
    this.filters = status ? [status] : [];
  }

  /** Number of active combined filters (statuses + kind + origin) for the badge. */
  activeFilterCount(): number {
    return this.filters.length + (this.category !== 'all' ? 1 : 0) + (this.originFilter !== 'any' ? 1 : 0);
  }

  clearFilters() {
    this.filters = [];
    this.category = 'all';
    this.originFilter = 'any';
    this.search = '';
  }

  /** Stage a draft; schedules the debounced autosave commit. Each new draft
   * bumps the entry's epoch, so an older staged commit becomes stale and can
   * never overwrite the newer text when its timer fires. */
  setDraft(id: string, text: string) {
    const entry = this.byId(id);
    if (!entry) return;
    this.draftEpoch[id] = (this.draftEpoch[id] ?? 0) + 1;
    this.drafts[id] = text;
    this.saveStates[id] = 'dirty';
    this.scheduleSave(id);
  }

  /** Flush immediately (blur, Tab navigation, project close). Resolves true
   * when a pending draft was actually committed (W6: guided actions await the
   * real save instead of assuming it). */
  flushDraft(id: string): Promise<boolean> {
    const pending = this.timers.get(id);
    if (pending !== undefined) {
      clearTimeout(pending);
      this.timers.delete(id);
    }
    if (this.saveStates[id] === 'dirty' && this.drafts[id] !== undefined) {
      return this.commit(id, this.drafts[id], this.draftEpoch[id] ?? 0);
    }
    return Promise.resolve(false);
  }

  /** Cancel an uncommitted draft (Esc in the editor) and stop its autosave timer. */
  discardDraft(id: string) {
    const pending = this.timers.get(id);
    if (pending !== undefined) {
      clearTimeout(pending);
      this.timers.delete(id);
    }
    delete this.drafts[id];
    this.saveStates[id] = 'idle';
  }

  /** Mock of `update_translation(entry_id, locale, text)` with simulated
   * latency. Resolves true when the write actually landed on the entry.
   * W6 (lead 034): the commit is bound to the project generation it was
   * staged in and to the entry's draft epoch — a stale completion resolves
   * false and writes NOTHING (no target, no draft, no save-state churn). */
  private commit(id: string, text: string, epoch: number): Promise<boolean> {
    const gen = this.generation;
    this.saveStates[id] = 'saving';
    if (this.source === 'contract') {
      // Real client path (027 semantics still hold: the guided action
      // resolves only when the save verifiably landed).
      return this.commitContract(id, text, gen, epoch);
    }
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        this.pendingCommits.delete(id);
        if (gen !== this.generation || this.draftEpoch[id] !== epoch) {
          resolve(false); // stale: project replaced or a newer draft exists
          return;
        }
        const entry = this.byId(id);
        let ok = false;
        if (entry) {
          entry.target = text;
          const trimmed = text.trim();
          if (trimmed && (entry.status === 'untranslated' || entry.status === 'todo')) {
            entry.status = 'translated';
          } else if (!trimmed && entry.status === 'translated') {
            entry.status = 'untranslated';
          }
          entry.editedAt = new Date().toISOString();
          if (entry.origin && entry.origin !== 'human') entry.origin = 'human';
          ok = true;
        }
        delete this.drafts[id];
        this.saveStates[id] = 'saved';
        setTimeout(() => {
          if (this.saveStates[id] === 'saved') this.saveStates[id] = 'idle';
        }, SAVED_INDICATOR_MS);
        resolve(ok);
      }, SAVING_INDICATOR_MS);
      this.pendingCommits.set(id, { timer, resolve, gen, epoch });
    });
  }

  /** Cancel staged-but-not-fired commits: they resolve false and write
   * nothing (W6/034). Used on locale switches — a commit staged for one
   * locale must never land after the active dataset changed. */
  cancelPendingCommits(): void {
    for (const [id, pending] of this.pendingCommits) {
      clearTimeout(pending.timer);
      this.saveStates[id] = 'idle';
      pending.resolve(false);
    }
    this.pendingCommits.clear();
  }

  private scheduleSave(id: string) {
    const pending = this.timers.get(id);
    if (pending !== undefined) clearTimeout(pending);
    this.timers.set(
      id,
      setTimeout(() => {
        this.timers.delete(id);
        if (this.drafts[id] !== undefined && this.saveStates[id] === 'dirty') {
          this.commit(id, this.drafts[id], this.draftEpoch[id] ?? 0);
        }
      }, AUTOSAVE_DEBOUNCE_MS)
    );
  }

  /** Flush every pending draft (used when leaving the workspace). */
  flushAll() {
    for (const id of Object.keys(this.drafts)) this.flushDraft(id);
  }

  hasPending(): boolean {
    return Object.values(this.saveStates).some((s) => s === 'dirty' || s === 'saving');
  }

  // ---------------------------------------------------------------- contract (W-built)

  /** Map a canonical contract snapshot onto the workspace Entry shape.
   * Source fields are read-only by contract; the target comes from the
   * translation rows of the active target locale (folder contract "Russian").
   *
   * `keepDrafts` (Pass A P1-1): the disk-adoption refresh maps the DISK
   * truth but keeps the caller's unsaved drafts/draft epochs/save states —
   * the backend discards ITS dirty state (disk wins), the client's local
   * draft survives as dirty and the user can retry the save on top of the
   * adopted revision. */
  private applyContractSnapshot(snap: ProjectSnapshotDto, opts: { keepDrafts?: boolean } = {}) {
    const mapped: Entry[] = [];
    const byId = new Map<string, Entry>();
    this.contractIdentities = {};
    const structuralId = (id: { kind: string; key: string; def_type?: string }): string => {
      // P1: the def_type discriminator is part of the structural identity —
      // dropping it makes any typed DefInjected/TKey intent unresolvable.
      return id.def_type ? `${id.kind}:${id.key}:${id.def_type}` : `${id.kind}:${id.key}`;
    };
    for (const e of snap.project.entries) {
      const id = structuralId(e.id);
      this.contractIdentities[id] = { kind: e.id.kind, key: e.id.key };
      if (e.id.def_type) this.contractIdentities[id].def_type = e.id.def_type;
      const kind: EntryKind = e.id.kind === 'DefInjected' || e.id.kind === 'TKey' ? e.id.kind : 'Keyed';
      const entry: Entry = {
        id,
        kind,
        key: e.id.key,
        source: e.text,
        target: '',
        status: 'untranslated',
        file: '',
        line: 0,
        // Live Source Inspector projection (wave 12): present on contract
        // snapshots that can honestly project it; the SOURCE tab renders it
        // verbatim and never fixture data in this mode.
        sourceRef: e.source_ref
          ? { file: e.source_ref.file, line: e.source_ref.line ?? null, selected_by: e.source_ref.selected_by }
          : null
      };
      mapped.push(entry);
      byId.set(id, entry);
    }
    for (const t of snap.project.translations) {
      const id = structuralId(t.source_id);
      const entry = byId.get(id);
      if (!entry) continue;
      if (!(t.locale ?? '').toLowerCase().startsWith('ru')) continue; // active target slice
      entry.target = t.text ?? '';
      if (t.completeness === 'todo') entry.status = 'todo';
      else if (t.text !== null && t.text !== '') entry.status = 'translated';
      if (t.review === 'needs_review') entry.status = 'pending_review';
      // Audit P1-1: provenance and validation ride the snapshot — the DTO
      // already carries them; dropping them left the provenance filter and
      // validation badges dead in live mode.
      entry.origin = CONTRACT_ORIGIN_TO_UI[t.origin] ?? null;
      if (t.validation === 'ok') {
        entry.validation = 'ok';
        entry.validationIssues = undefined;
      } else if (typeof t.validation === 'object' && t.validation !== null) {
        entry.validation = 'issues';
        entry.validationIssues = t.validation.issues;
      } else {
        entry.validation = 'unknown';
        entry.validationIssues = undefined;
      }
    }
    if (opts.keepDrafts) {
      // Recovery mapping: kill in-flight autosave timers (they abort on the
      // generation bump anyway) and swap the entries, but KEEP the drafts,
      // draft epochs and dirty save states.
      this.timers.clear();
      this.generation += 1;
      this.entries = mapped;
    } else {
      this.flushAll();
      this.cancelPendingCommits();
      this.timers.clear();
      this.generation += 1;
      this.draftEpoch = {};
      this.entries = mapped;
      this.drafts = {};
      this.saveStates = {};
      this.selectedId = null;
      this.filters = [];
      this.category = 'all';
      this.originFilter = 'any';
      this.search = '';
    }
    this.isDemo = false;
    this.contractProjectId = snap.project_id;
    this.contractRevision = snap.revision;
    // v2 wire: the apply base is the last ACKED revision, not the (possibly
    // ahead) in-memory revision.
    this.contractAckedRevision = snap.acked_revision ?? snap.revision;
    this.contractEpoch = snap.session_epoch;
    this.contractError = null;
    this.projectName = snap.project_id;
    this.projectDisplayName = null;
    this.source = 'contract';
    // M-5: the snapshot DTO carries no display name — resolve it from the
    // project list summaries (best-effort; the id remains the fallback and a
    // dead transport only costs the nicer label, never a failure).
    void this.resolveDisplayName(snap.project_id);
  }

  /** Fire-and-forget display-name resolution (M-5). Guarded against a project
   * switch mid-flight: only the CURRENT project's name may land. */
  private async resolveDisplayName(projectId: string): Promise<void> {
    try {
      const list = await this.listContractProjects();
      if (this.contractProjectId !== projectId) return;
      const hit = list.find((p) => p.project_id === projectId);
      if (hit?.name) this.projectDisplayName = hit.name;
    } catch {
      // Cosmetic resolution — the id fallback stays.
    }
  }

  /** Pass A P1-1: adopt the DISK state after a typed failure
   *  (`project_changed_on_disk` / `save_failed`). Contract doctrine: refresh
   *  discards the backend's dirty state (disk wins) — so it is always an
   *  EXPLICIT user action (the workspace banner button), never a silent
   *  auto-call. The caller's local draft survives as dirty and can be
   *  re-saved on top of the adopted revision. */
  async refreshContract(): Promise<boolean> {
    const projectId = this.contractProjectId;
    if (!projectId || this.refreshing) return false;
    this.refreshing = true;
    try {
      const snap = await this.cc().refresh(projectId);
      this.applyContractSnapshot(snap, { keepDrafts: true });
      this.contractError = null;
      return true;
    } catch (e) {
      this.contractError =
        e instanceof ContractClientError ? contractErrorText(e.code, e.message) : String(e);
      return false;
    } finally {
      this.refreshing = false;
    }
  }

  /** Create a project from a real mod folder (mode 'tauri'). */
  async createContractProject(
    modRoot: string,
    targetVersion: string = DEFAULT_TARGET_VERSION
  ): Promise<boolean> {
    try {
      const snap = await this.cc().createProject(modRoot, targetVersion);
      this.applyContractSnapshot(snap);
      return true;
    } catch (e) {
      this.contractError =
        e instanceof ContractClientError ? contractErrorText(e.code, e.message) : String(e);
      return false;
    }
  }

  /** Reopen a project by id (restart journey: fresh launch → open). */
  async openContractProject(projectId: string): Promise<boolean> {
    try {
      const snap = await this.cc().openProject(projectId);
      this.applyContractSnapshot(snap);
      return true;
    } catch (e) {
      this.contractError =
        e instanceof ContractClientError ? contractErrorText(e.code, e.message) : String(e);
      return false;
    }
  }

  /** Recent projects for the contract-mode Home (real list). */
  async listContractProjects(): Promise<ProjectSummaryDto[]> {
    try {
      return await this.cc().listProjects();
    } catch (e) {
      // Built-GUI lesson (2026-09-26): this catch used to be silent, so a
      // dead transport was indistinguishable from "no projects yet". Log it.
      console.error(
        '[rimloc] project_list failed:',
        e instanceof ContractClientError ? `${e.code}: ${e.message}` : e
      );
      return [];
    }
  }

  /** Contract save: one typed set_translation intent with lost-update guards.
   * On failure the dirty draft is KEPT (persist-before-ack) and false returns. */
  private async commitContract(id: string, text: string, gen: number, epoch: number): Promise<boolean> {
    if (gen !== this.generation || this.draftEpoch[id] !== epoch) return false;
    const identity = this.contractIdentities[id];
    const projectId = this.contractProjectId;
    if (!identity || !projectId) return false;
    try {
      const entryId: { kind: string; key: string; def_type?: string } = {
        kind: identity.kind,
        key: identity.key
      };
      if (identity.def_type) entryId.def_type = identity.def_type;
      const resp = await this.cc().applyIntents({
        projectId,
        // Pass A 1.2: while the backend session is dirty, the apply base is
        // the last ACKED revision, not the (possibly ahead) in-memory one.
        expectedRevision: this.contractAckedRevision || this.contractRevision,
        sessionEpoch: this.contractEpoch,
        intents: [{ entry: entryId, locale: 'Russian', action: 'set_translation', text }]
      });
      // Verifier P2: the acked base moves ONLY on an applied response. On a
      // refusal (applied: 0) a dirty backend echoes its own AHEAD revision
      // (session.rs: "no revision bump, no save, no dirty state" — yet
      // st.revision may already lead acked after a save_failed); adopting
      // it as the base would deadlock every later apply on stale_revision.
      if (resp.applied > 0) {
        this.contractRevision = resp.revision; // apply acks to disk
        this.contractAckedRevision = resp.revision;
        const entry = this.byId(id);
        if (entry) {
          entry.target = text;
          if (entry.status === 'untranslated' || entry.status === 'todo') entry.status = 'translated';
          entry.editedAt = new Date().toISOString();
          // Audit P1-1 mirror of session.rs: an applied edit re-stamps
          // provenance as Human so the provenance filter stays truthful.
          entry.origin = 'human';
          // Placeholder suspicion classifies as Issues on the backend; the
          // message mirrors session.rs and the authoritative state arrives
          // with the next snapshot/refresh.
          if (/%(?!%|[a-zA-Z]|\{|\d+\$)/.test(text)) {
            entry.validation = 'issues';
            entry.validationIssues = ['suspicious placeholder (single % not part of a known token)'];
          } else {
            entry.validation = 'ok';
            entry.validationIssues = undefined;
          }
        }
      }
      if (resp.applied === 0) {
        // Nothing stored: keep the draft staged with the same 'dirty'
        // semantics as the typed failure path — otherwise the row sticks
        // on "saving…" with no backend write behind it.
        this.saveStates[id] = 'dirty';
        // Pass A 2.2: a refused intent is DATA, not a silent dirty — surface
        // the typed refusal where the user works.
        const sk = resp.skipped[0];
        this.contractError = sk
          ? contractErrorText(sk.code, sk.message)
          : 'contract_violation: the intent was skipped by the backend';
      }
      return resp.applied > 0;
    } catch (e) {
      // persist-before-ack: the draft stays staged for retry; the typed
      // error is surfaced without ever clearing the caller's text.
      this.contractError =
        e instanceof ContractClientError ? contractErrorText(e.code, e.message) : String(e);
      this.saveStates[id] = 'dirty';
      return false;
    }
  }

  /** Context panel actions. W-built (P2-b): in contract mode only statuses
   * expressible as v1 intents are proxied (and reverted if the backend
   * rejects); anything else stays untouched instead of being silently lost
   * on reopen. */
  setStatus(id: string, status: EntryStatus) {
    const entry = this.byId(id);
    if (!entry) return;
    if (this.source !== 'contract') {
      entry.status = status;
      return;
    }
    if (status !== 'translated' && status !== 'todo') return; // not in v1 slice
    const prevStatus = entry.status;
    const intentText = status === 'translated' ? entry.target : 'TODO';
    const action = status === 'translated' ? 'set_translation' : 'mark_todo';
    void this.applyIntentAsync(id, action, intentText, () => {
      entry.status = prevStatus; // rollback on typed rejection
    });
    entry.status = status; // optimistic; rolled back on rejection
  }

  /** Fire one v1 intent; on success bumps the revision, on typed rejection
   * surfaces the error and runs the caller's rollback. */
  private async applyIntentAsync(
    id: string,
    action: 'set_translation' | 'mark_todo' | 'clear_translation',
    text: string | undefined,
    rollback?: () => void
  ): Promise<boolean> {
    const identity = this.contractIdentities[id];
    const projectId = this.contractProjectId;
    if (!identity || !projectId) return false;
    const entryId: { kind: string; key: string; def_type?: string } = {
      kind: identity.kind,
      key: identity.key
    };
    if (identity.def_type) entryId.def_type = identity.def_type;
    try {
      const resp = await this.cc().applyIntents({
        projectId,
        // Pass A 1.2: acked revision is the apply base (see commitContract).
        expectedRevision: this.contractAckedRevision || this.contractRevision,
        sessionEpoch: this.contractEpoch,
        intents: [{ entry: entryId, locale: 'Russian', action, text }]
      });
      // Verifier P2: the acked base moves ONLY on an applied response (see
      // commitContract) — a refusal must never adopt the backend's ahead
      // revision as the base.
      if (resp.applied > 0) {
        this.contractRevision = resp.revision;
        this.contractAckedRevision = resp.revision;
        const entry = this.byId(id);
        if (entry) entry.origin = 'human';
      } else {
        // Pass A 2.2: a refused intent is DATA — surface the typed refusal.
        const sk = resp.skipped[0];
        this.contractError = sk
          ? contractErrorText(sk.code, sk.message)
          : 'contract_violation: the intent was skipped by the backend';
      }
      return resp.applied > 0;
    } catch (e) {
      rollback?.();
      this.contractError =
        e instanceof ContractClientError ? contractErrorText(e.code, e.message) : String(e);
      return false;
    }
  }

  /** Apply a SUGGESTIONS-tab pick. AI output never lands as translated (spec §8):
   * it becomes pending_review; TM/import glossary picks count as accepted drafts.
   * W-built (P2-b): in contract mode this is a REAL set_translation intent —
   * the text applies only after the backend accepts it. */
  applySuggestion(id: string, text: string, origin: Origin) {
    const entry = this.byId(id);
    if (!entry) return;
    if (this.source === 'contract') {
      const prevTarget = entry.target;
      void this.applyIntentAsync(id, 'set_translation', text).then((ok) => {
        if (!ok) {
          entry.target = prevTarget; // rejection: nothing silently lost
        }
      });
      entry.target = text; // optimistic draft display until the response
      return;
    }
    entry.target = text;
    entry.origin = origin;
    entry.editedAt = new Date().toISOString();
    if (origin === 'LLM') {
      if (entry.status === 'untranslated' || entry.status === 'todo') entry.status = 'pending_review';
    } else if (entry.status === 'untranslated' || entry.status === 'todo') {
      entry.status = 'translated';
    }
  }

  setNote(id: string, note: string) {
    const entry = this.byId(id);
    if (!entry) return;
    if (this.source === 'contract') {
      // P2-b: notes are not in the v1 contract slice — refused loudly instead
      // of being silently dropped on reopen.
      this.contractError = 'notes: local only in this slice (not persisted to the project)';
      return;
    }
    entry.note = note;
  }

  /** Reset to the pristine mock dataset (dev panel). W6/034: pending commits
   * are CANCELLED, not flushed — a staged demo save must never land in the
   * replaced project. Bumping the generation also invalidates them. */
  reset() {
    this.cancelPendingCommits();
    this.timers.clear();
    this.generation += 1;
    buildState.reset();
    this.draftEpoch = {};
    this.entries = cloneInitial();
    this.projectName = DEFAULT_PROJECT_NAME;
    this.projectDisplayName = null;
    this.drafts = {};
    this.saveStates = {};
    this.selectedId = null;
    this.filters = [];
    this.category = 'all';
    this.originFilter = 'any';
    this.search = '';
    this.isDemo = false;
    this.source = 'fixture';
    this.contractProjectId = null;
    this.contractRevision = 0;
    this.contractAckedRevision = 0;
    this.contractEpoch = 0;
    this.contractError = null;
    this.refreshing = false;
  }
}

export const project = new ProjectStore();
