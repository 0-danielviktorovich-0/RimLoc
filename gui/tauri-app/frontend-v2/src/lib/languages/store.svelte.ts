// Multi-target store (W2, MULTI_TARGET_MANDATE): the mock adapter over the
// canonical project model (rimloc-domain/canonical — Project {entries,
// translations per locale}):
//
//   - ONE source inventory: entry identity (id/key/kind/source/file/line) is
//     target-locale independent and is never touched after init.
//   - N targets: per-locale translation state ({target,status,origin,editedAt,
//     issues} per entry + per-entry revision) — the frontend mirror of
//     Translation rows keyed by locale.
//   - Switching the active target is a dataset SUBSTITUTION over the same
//     source entries: no rescan, same entry objects, editor selection and
//     scroll position survive.
//   - Stale-AI protection per target locale (mandate §12): an AI result
//     exported at revision N is rejected when the entry moved to revision > N
//     or when aimed at a different locale — it becomes a suggestion, human
//     work is never overwritten.
//
// The substitution writes into the existing project store's entries (the mock
// stand-in for the service boundary). Everything else in the workspace —
// navigator, table, detail panel, review queue — derives from those entries
// and therefore follows the active target without any changes of its own.

import { project } from '../stores/project.svelte';
import { mockEntries } from '../mock/data';
import type { Entry, EntryIssue, EntryStatus, Origin, Suggestion } from '../mock/types';
import { registry, SOURCE_LOCALE, type LanguageDefinition } from './registry';
import { corpusEditedAt, corpusForLocale, corpusOrigin, type AddFlow } from './corpus';

/** Per-translation state for one entry inside one target locale. */
export interface TargetEntryState {
  target: string;
  status: EntryStatus;
  origin: Origin | null;
  editedAt: string | null;
  issues: EntryIssue[];
  /** Suggestions parked on this translation (e.g. stale AI drafts); never auto-applied. */
  suggestions?: Suggestion[];
}

/** One target locale's dataset: the locale-scoped slice of canonical translations. */
export interface TargetState {
  locale: string;
  addedVia: AddFlow | 'wizard';
  entries: Record<string, TargetEntryState>;
  /** Translation revision per entry — bumped on every human/AI write. */
  revs: Record<string, number>;
  lastModified: string;
}

export interface TargetSummary {
  locale: string;
  definition: LanguageDefinition;
  progress: number;
  issueCount: number;
  lastModified: string;
  addedVia: AddFlow | 'wizard';
}

/** Result of an AI-result application attempt (mandate §12 semantics). */
export type AiApplyResult =
  | { status: 'applied' }
  | { status: 'stale'; currentRev: number }
  | { status: 'unknown-entry' }
  | { status: 'unknown-batch' }
  | { status: 'no-target' };

/** A batch exported for external AI: captured per-entry revisions. */
export interface AiBatch {
  id: string;
  locale: string;
  exportedAt: string;
  /** entryId → revision captured at export time. */
  revs: Record<string, number>;
}

const PERSIST_KEY = 'rimloc.project.testmod.multitarget.v1';

interface PersistedShape {
  active: string;
  pinned: string[];
  targets: Array<{ locale: string; addedVia: AddFlow | 'wizard' }>;
}

function nowIso(): string {
  return new Date().toISOString();
}

class MultiTargetStore {
  /** Active target locale — what the editor shows right now. */
  activeLocale = $state<string>('ru');
  /** Locales pinned to the quick tabs. */
  pinned = $state<string[]>(['ru', 'uk']);
  /** Per-locale datasets (insertion order = target order in the manager). */
  targets = $state<Record<string, TargetState>>({});
  /** Batches exported by THIS store: the trusted side of the AI round-trip. */
  private exportedBatches = new Map<string, AiBatch>();
  /** Language manager dialog open state (workspace toolbar + palette entry). */
  managerOpen = $state(false);
  /** What the manager should focus when opened ('add' pre-opens the add flow). */
  managerIntent = $state<'manage' | 'add'>('manage');
  /** Mirror of registry.custom so components get reactivity for free. */
  customLanguages = $state(registry.custom);

  constructor() {
    this.initFromPristine();
  }

  // ---------------------------------------------------------------- invariants

  /** Source locale of this mock project — never a target, never detachable. */
  get sourceLocale(): string {
    return SOURCE_LOCALE;
  }

  get sourceLanguage(): LanguageDefinition {
    return registry.resolve(SOURCE_LOCALE);
  }

  /** Ordered summaries for the switcher/manager: progress, issues, modified. */
  summaries(): TargetSummary[] {
    return Object.values(this.targets).map((t) => {
      const countable = this.countableIds();
      let translated = 0;
      let issues = 0;
      for (const id of countable) {
        const st = t.entries[id];
        if (st && st.status === 'translated') translated += 1;
        issues += st ? st.issues.length : 0;
      }
      return {
        locale: t.locale,
        definition: registry.resolve(t.locale),
        progress: countable.length ? Math.round((translated / countable.length) * 100) : 0,
        issueCount: issues,
        lastModified: t.lastModified,
        addedVia: t.addedVia
      };
    });
  }

  summary(locale: string): TargetSummary | undefined {
    return this.summaries().find((s) => s.locale === locale);
  }

  get activeDefinition(): LanguageDefinition {
    return registry.resolve(this.activeLocale);
  }

  get activeTarget(): TargetState | undefined {
    return this.targets[this.activeLocale];
  }

  /** Entries that count towards progress (orphans are excluded diagnostics). */
  private countableIds(): string[] {
    return project.entries.filter((e) => e.status !== 'orphan').map((e) => e.id);
  }

  // ---------------------------------------------------------------- init / reset

  /**
   * Build target datasets from the pristine mock inventory: the base dataset
   * (rich statuses/issues) becomes the RU translation layer; other locales
   * come from the deterministic corpus. Re-applies the persisted active target.
   *
   * The RU layer is built from the INVARIANT mock source, never from the live
   * `project.entries`: if this store is ever constructed twice over one page
   * (HMR swap, duplicate module instance), the live entries may already hold
   * another locale's dataset — copying them would silently corrupt the RU
   * layer. Canonical identity of the source inventory does not depend on the
   * currently rendered locale.
   */
  initFromPristine() {
    const ru = this.buildTargetState('ru', 'wizard');
    for (const e of mockEntries) {
      ru.entries[e.id] = {
        target: e.target,
        status: e.status,
        origin: e.origin ?? null,
        editedAt: e.editedAt ?? null,
        issues: (e.issues ?? []).map((i) => ({ ...i }))
      };
      ru.revs[e.id] = 1;
    }

    const restored = this.readPersisted();
    const targetList: Array<{ locale: string; addedVia: AddFlow | 'wizard' }> =
      restored?.targets?.length
        ? restored.targets
        : [
            { locale: 'ru', addedVia: 'wizard' },
            { locale: 'uk', addedVia: 'import' },
            { locale: 'ja', addedVia: 'pack' }
          ];

    const map: Record<string, TargetState> = { ru };
    for (const item of targetList) {
      if (item.locale === SOURCE_LOCALE) continue;
      if (map[item.locale]) continue;
      map[item.locale] =
        item.locale === 'ru'
          ? ru
          : this.buildTargetState(item.locale, (item.addedVia ?? 'empty') as AddFlow);
    }

    this.targets = map;
    this.pinned = (restored?.pinned ?? ['ru', 'uk']).filter((l) => map[l] || l === SOURCE_LOCALE);
    const wanted = restored?.active && map[restored.active] ? restored.active : 'ru';
    this.activeLocale = wanted;
    this.applyDataset(wanted);
    this.persist();
  }

  private buildTargetState(locale: string, via: AddFlow | 'wizard'): TargetState {
    const state: TargetState = {
      locale,
      addedVia: via,
      entries: {},
      revs: {},
      lastModified: nowIso()
    };
    const corpus = via === 'wizard' ? null : corpusForLocale(locale, via);
    const origin = via === 'empty' || via === 'wizard' ? null : corpusOrigin(via);
    const ids = project.entries.map((e) => e.id);
    ids.forEach((id, i) => {
      const text = corpus?.[id];
      state.entries[id] = {
        target: text ?? '',
        status: text ? 'translated' : 'untranslated',
        origin: text ? origin : null,
        editedAt: text ? corpusEditedAt(locale, i) : null,
        issues: []
      };
      state.revs[id] = 1;
    });
    return state;
  }

  // ---------------------------------------------------------------- switching

  /**
   * Switch the active target: capture the old locale's live edits back into
   * its dataset, then substitute the new dataset over the SAME entry objects.
   * No rescan happens; selection, filters and scroll position survive.
   */
  setActive(locale: string) {
    if (!this.targets[locale] || locale === this.activeLocale) return;
    this.flushDraftsSync();
    this.captureBack(this.activeLocale);
    this.applyDataset(locale);
    this.activeLocale = locale;
    this.persist();
  }

  /**
   * Commit pending drafts into the entries synchronously (mock save without
   * the 350 ms latency) and mark them 'saved' so the project store's pending
   * timers no-op later instead of writing into the NEXT locale's data.
   */
  private flushDraftsSync() {
    for (const [id, text] of Object.entries({ ...project.drafts })) {
      const entry = project.byId(id);
      if (!entry) continue;
      entry.target = text;
      const trimmed = text.trim();
      if (trimmed && (entry.status === 'untranslated' || entry.status === 'todo')) {
        entry.status = 'translated';
      } else if (!trimmed && entry.status === 'translated') {
        entry.status = 'untranslated';
      }
      entry.editedAt = nowIso();
      if (entry.origin && entry.origin !== 'human') entry.origin = 'human';
    }
    for (const id of Object.keys(project.drafts)) delete project.drafts[id];
    for (const [id, s] of Object.entries(project.saveStates)) {
      if (s === 'dirty' || s === 'saving') project.saveStates[id] = 'saved';
    }
  }

  /** Write the active entries' live translation state back into its dataset. */
  private captureBack(locale: string) {
    const target = this.targets[locale];
    if (!target) return;
    let touched = false;
    for (const entry of project.entries) {
      const stored = target.entries[entry.id];
      if (!stored) continue;
      const liveIssues = entry.issues ?? [];
      const changed =
        stored.target !== entry.target ||
        stored.status !== entry.status ||
        stored.origin !== (entry.origin ?? null) ||
        stored.editedAt !== (entry.editedAt ?? null) ||
        stored.issues.length !== liveIssues.length ||
        stored.issues.some((i, idx) => i !== liveIssues[idx]);
      if (!changed) continue;
      stored.target = entry.target;
      stored.status = entry.status;
      stored.origin = entry.origin ?? null;
      stored.editedAt = entry.editedAt ?? null;
      stored.issues = liveIssues.map((i) => ({ ...i }));
      target.revs[entry.id] = (target.revs[entry.id] ?? 1) + 1;
      touched = true;
    }
    if (touched) target.lastModified = nowIso();
  }

  /** Substitute a locale's dataset over the source entries (same objects). */
  private applyDataset(locale: string) {
    const target = this.targets[locale];
    if (!target) return;
    for (const entry of project.entries) {
      const st = target.entries[entry.id];
      if (!st) continue;
      entry.target = st.target;
      entry.status = st.status;
      entry.origin = st.origin;
      entry.editedAt = st.editedAt;
      entry.issues = st.issues.length ? [...st.issues] : undefined;
    }
  }

  // ---------------------------------------------------------------- targets lifecycle

  /** Add a target locale with a given fill flow; becomes active? No — stays. */
  addTarget(locale: string, via: AddFlow) {
    if (locale === SOURCE_LOCALE || this.targets[locale]) return false;
    this.targets = {
      ...this.targets,
      [locale]: this.buildTargetState(locale, via)
    };
    this.persist();
    return true;
  }

  /**
   * Detach a target locale. Never the source; never the last remaining target.
   * Returns the volume of work being removed (translated entries) so the UI
   * can show what the user is about to lose, or null when refused.
   */
  detachVolume(locale: string): number | null {
    const target = this.targets[locale];
    if (!target) return null;
    let translated = 0;
    for (const st of Object.values(target.entries)) if (st.status === 'translated') translated += 1;
    return translated;
  }

  detach(locale: string): { ok: true } | { ok: false; reason: 'source' | 'last' | 'missing' } {
    if (locale === SOURCE_LOCALE) return { ok: false, reason: 'source' };
    if (!this.targets[locale]) return { ok: false, reason: 'missing' };
    if (Object.keys(this.targets).length <= 1) return { ok: false, reason: 'last' };
    const { [locale]: _removed, ...rest } = this.targets;
    this.targets = rest;
    this.pinned = this.pinned.filter((l) => l !== locale);
    if (this.activeLocale === locale) {
      const fallback = Object.keys(rest)[0] ?? SOURCE_LOCALE;
      this.activeLocale = fallback;
      this.applyDataset(fallback);
    }
    this.persist();
    return { ok: true };
  }

  /** Remove a custom language: drops the language and any target using it. */
  removeCustomLanguage(localeId: string): boolean {
    if (!registry.isCustom(localeId)) return false;
    const hadTarget = Boolean(this.targets[localeId]);
    if (hadTarget) this.detach(localeId);
    const removed = registry.removeCustom(localeId);
    if (removed) this.customLanguages = registry.custom;
    return removed;
  }

  // ---------------------------------------------------------------- pinning

  isPinned(locale: string): boolean {
    return this.pinned.includes(locale);
  }

  togglePin(locale: string) {
    if (locale === SOURCE_LOCALE) return;
    this.pinned = this.isPinned(locale)
      ? this.pinned.filter((l) => l !== locale)
      : [...this.pinned, locale];
    this.persist();
  }

  // ---------------------------------------------------------------- stale-AI protection

  /**
   * Export a batch: capture per-entry revisions of the target locale. The
   * batch is recorded here — applying later resolves the batch by id against
   * THIS record, so locale and revisions are store-side truth, not
   * caller-side claims.
   */
  exportAiBatch(locale: string, entryIds: string[]): AiBatch | null {
    const target = this.targets[locale];
    if (!target) return null;
    const revs: Record<string, number> = {};
    for (const id of entryIds) {
      if (!target.entries[id]) continue;
      revs[id] = target.revs[id] ?? 1;
    }
    const batch: AiBatch = {
      id: `batch-${locale}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      locale,
      exportedAt: nowIso(),
      revs
    };
    this.exportedBatches.set(batch.id, batch);
    return batch;
  }

  /**
   * Apply an imported AI result for ONE entry. Mandate §12: the batch holds
   * the revision at export time; if the entry moved since (human edit) the
   * result is stale — it never writes, the human text survives. The batch id
   * is resolved against the store's own export history: the result always
   * writes into the locale the batch was exported FOR (a tampered copy cannot
   * re-aim it at another locale — mandate §11, wrong target locale). AI
   * output never lands as 'translated' — it becomes pending_review.
   */
  applyAiResult(batchRef: AiBatch, entryId: string, text: string): AiApplyResult {
    const batch = this.exportedBatches.get(batchRef.id);
    if (!batch) return { status: 'unknown-batch' };
    const target = this.targets[batch.locale];
    if (!target) return { status: 'no-target' };
    const stored = target.entries[entryId];
    if (!stored) return { status: 'unknown-entry' };
    const exportedRev = batch.revs[entryId];
    if (exportedRev === undefined) return { status: 'unknown-entry' };
    const currentRev = target.revs[entryId] ?? 1;
    if (currentRev !== exportedRev) {
      // Stale (mandate §12): the AI draft NEVER writes — it is parked as an
      // LLM suggestion for the reviewer (W3 hybrid acceptance), while the
      // human text and its revision stay untouched.
      stored.suggestions = [...(stored.suggestions ?? []), { source: 'LLM', text }];
      if (batch.locale === this.activeLocale) {
        const entry = project.byId(entryId);
        if (entry) entry.suggestions = [...(entry.suggestions ?? []), { source: 'LLM', text }];
      }
      return { status: 'stale', currentRev };
    }
    stored.target = text;
    stored.status = 'pending_review';
    stored.origin = 'LLM';
    stored.editedAt = nowIso();
    target.revs[entryId] = currentRev + 1;
    target.lastModified = nowIso();
    if (batch.locale === this.activeLocale) {
      const entry = project.byId(entryId);
      if (entry) {
        entry.target = text;
        entry.status = 'pending_review';
        entry.origin = 'LLM';
        entry.editedAt = stored.editedAt;
      }
    }
    return { status: 'applied' };
  }

  /** Current revision of an entry in a locale (diagnostics / advanced UI). */
  revisionOf(locale: string, entryId: string): number | undefined {
    return this.targets[locale]?.revs[entryId];
  }

  // ---------------------------------------------------------------- batch fills (W3 hybrid)

  /**
   * Fill empty targets from translation-memory hits (controlled hybrid, W3).
   * TM NEVER overwrites existing work: only entries whose target is still
   * empty are filled. Writes are origin 'TM', status 'translated', revision
   * bumped; the active locale's live entries mirror the write immediately.
   */
  applyTm(locale: string, updates: Record<string, string>): { applied: number; skipped: number } {
    const target = this.targets[locale];
    if (!target) return { applied: 0, skipped: 0 };
    let applied = 0;
    let skipped = 0;
    for (const [id, text] of Object.entries(updates)) {
      const stored = target.entries[id];
      if (!stored || stored.target.trim() !== '') {
        skipped += 1;
        continue;
      }
      stored.target = text;
      stored.status = 'translated';
      stored.origin = 'TM';
      stored.editedAt = nowIso();
      target.revs[id] = (target.revs[id] ?? 1) + 1;
      applied += 1;
      if (locale === this.activeLocale) {
        const entry = project.byId(id);
        if (entry) {
          entry.target = text;
          entry.status = 'translated';
          entry.origin = 'TM';
          entry.editedAt = stored.editedAt;
        }
      }
    }
    if (applied > 0) target.lastModified = nowIso();
    return { applied, skipped };
  }

  /**
   * A human editor commit at the store boundary (W3 hybrid): overwrite the
   * target text, mark it human-owned and bump the revision — the anchor the
   * stale-AI protection compares exported revisions against.
   */
  applyHumanEdit(locale: string, entryId: string, text: string): boolean {
    const target = this.targets[locale];
    const stored = target?.entries[entryId];
    if (!target || !stored) return false;
    stored.target = text;
    stored.status = 'translated';
    stored.origin = 'human';
    stored.editedAt = nowIso();
    target.revs[entryId] = (target.revs[entryId] ?? 1) + 1;
    target.lastModified = nowIso();
    if (locale === this.activeLocale) {
      const entry = project.byId(entryId);
      if (entry) {
        entry.target = text;
        entry.status = 'translated';
        entry.origin = 'human';
        entry.editedAt = stored.editedAt;
      }
    }
    return true;
  }

  // ---------------------------------------------------------------- manager dialog

  openManager(intent: 'manage' | 'add' = 'manage') {
    this.managerIntent = intent;
    this.managerOpen = true;
  }

  closeManager() {
    this.managerOpen = false;
  }

  /** Re-mirror the custom-language list after registry mutations. */
  refreshCustom() {
    this.customLanguages = [...registry.custom];
  }

  // ---------------------------------------------------------------- persistence

  private readPersisted(): PersistedShape | null {
    try {
      const raw = localStorage.getItem(PERSIST_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as PersistedShape;
      if (!parsed || typeof parsed !== 'object') return null;
      return parsed;
    } catch {
      return null;
    }
  }

  private persist() {
    try {
      const shape: PersistedShape = {
        active: this.activeLocale,
        pinned: [...this.pinned],
        targets: Object.values(this.targets).map((t) => ({ locale: t.locale, addedVia: t.addedVia }))
      };
      localStorage.setItem(PERSIST_KEY, JSON.stringify(shape));
    } catch {
      /* storage unavailable — session-only state */
    }
  }
}

export const languages = new MultiTargetStore();
