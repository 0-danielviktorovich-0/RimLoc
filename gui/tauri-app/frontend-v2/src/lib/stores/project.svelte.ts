// Project store — the mock stand-in for the backend service boundary (spec §3.1).
// The only write operation mimics `update_translation(entry_id, locale, text)`;
// everything else is local reactive state. No Tauri/IPC calls anywhere.

import { mockEntries } from '../mock/data';
import type { Entry, EntryKind, EntryStatus, SaveState } from '../mock/types';

export type StatusCounts = Record<EntryStatus, number>;

const AUTOSAVE_DEBOUNCE_MS = 800; // spec open question #2: 800ms debounce
const SAVING_INDICATOR_MS = 350; // simulated latency so the "saving…" badge is visible
const SAVED_INDICATOR_MS = 1200;

function cloneInitial(): Entry[] {
  return structuredClone(mockEntries);
}

class ProjectStore {
  projectName = $state('TestMod');
  targetLocale = $state('ru');

  entries = $state<Entry[]>(cloneInitial());

  /** Draft text per entry id — survives leaving the cell, shown italic until commit. */
  drafts = $state<Record<string, string>>({});
  saveStates = $state<Record<string, SaveState>>({});

  selectedId = $state<string | null>(null);
  filters = $state<EntryStatus[]>([]);
  category = $state<'all' | EntryKind>('all');
  search = $state('');

  private timers = new Map<string, ReturnType<typeof setTimeout>>();

  byId(id: string): Entry | undefined {
    return this.entries.find((e) => e.id === id);
  }

  get selected(): Entry | null {
    return this.selectedId ? (this.byId(this.selectedId) ?? null) : null;
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

  /** Mock of the service read: category + status multi-filter + debounced search, OR semantics on filters. */
  filtered(): Entry[] {
    const needle = this.search.trim().toLowerCase();
    return this.entries.filter((e) => {
      if (this.category !== 'all' && e.kind !== this.category) return false;
      if (this.filters.length > 0 && !this.filters.includes(e.status)) return false;
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

  clearFilters() {
    this.filters = [];
    this.search = '';
  }

  /** Stage a draft; schedules the debounced autosave commit. */
  setDraft(id: string, text: string) {
    const entry = this.byId(id);
    if (!entry) return;
    this.drafts[id] = text;
    this.saveStates[id] = 'dirty';
    this.scheduleSave(id);
  }

  /** Flush immediately (blur, Tab navigation, project close). */
  flushDraft(id: string) {
    const pending = this.timers.get(id);
    if (pending !== undefined) {
      clearTimeout(pending);
      this.timers.delete(id);
    }
    if (this.saveStates[id] === 'dirty' && this.drafts[id] !== undefined) {
      this.commit(id, this.drafts[id]);
    }
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

  /** Mock of `update_translation(entry_id, locale, text)` with simulated latency. */
  private commit(id: string, text: string) {
    this.saveStates[id] = 'saving';
    setTimeout(() => {
      const entry = this.byId(id);
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
      }
      delete this.drafts[id];
      this.saveStates[id] = 'saved';
      setTimeout(() => {
        if (this.saveStates[id] === 'saved') this.saveStates[id] = 'idle';
      }, SAVED_INDICATOR_MS);
    }, SAVING_INDICATOR_MS);
  }

  private scheduleSave(id: string) {
    const pending = this.timers.get(id);
    if (pending !== undefined) clearTimeout(pending);
    this.timers.set(
      id,
      setTimeout(() => {
        this.timers.delete(id);
        if (this.drafts[id] !== undefined && this.saveStates[id] === 'dirty') {
          this.commit(id, this.drafts[id]);
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

  /** Context panel actions. */
  setStatus(id: string, status: EntryStatus) {
    const entry = this.byId(id);
    if (entry) entry.status = status;
  }

  setNote(id: string, note: string) {
    const entry = this.byId(id);
    if (entry) entry.note = note;
  }

  /** Reset to the pristine mock dataset (dev panel). */
  reset() {
    this.flushAll();
    this.timers.clear();
    this.entries = cloneInitial();
    this.drafts = {};
    this.saveStates = {};
    this.selectedId = null;
    this.filters = [];
    this.category = 'all';
    this.search = '';
  }
}

export const project = new ProjectStore();
