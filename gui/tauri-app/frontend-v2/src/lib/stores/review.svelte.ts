// Review store (mandate §12): the queue of issues derived from the live mock
// corpus plus the session-local resolutions (fixed / ignored / reviewed).
// Mock actions only — nothing here calls a backend; it edits the same local
// project store the editor uses, so fixes made here are visible there.

import { project } from './project.svelte';
import type { Entry, IssueKind, IssueSeverity } from '../mock/types';

export interface ReviewIssue {
  /** Stable id: `${entryId}:${kind}` — explicit and derived issues dedupe on it. */
  id: string;
  entryId: string;
  kind: IssueKind;
  severity: IssueSeverity;
  /** Message from the entry's explicit issue, if it has one. */
  message: string | null;
}

export type Resolution = 'fixed' | 'ignored' | 'reviewed';

/** Review-queue order by severity, then by kind (spec §11 ordering). */
const KIND_ORDER: IssueKind[] = [
  'placeholder_mismatch',
  'glossary',
  'untranslated_suspect',
  'ambiguity',
  'wordinfo',
  'ai_concern'
];

/** Sources at least this long that stay untranslated are "suspicious" (mock heuristic). */
const SUSPECT_SOURCE_LEN = 60;

function placeholders(text: string): Set<string> {
  return new Set(text.match(/\{[^}]+\}/g) ?? []);
}

function collectIssues(entries: Entry[]): ReviewIssue[] {
  const out: ReviewIssue[] = [];
  const seen = new Set<string>();
  const push = (e: Entry, kind: IssueKind, severity: IssueSeverity, message: string | null) => {
    const id = `${e.id}:${kind}`;
    if (seen.has(id)) return;
    seen.add(id);
    out.push({ id, entryId: e.id, kind, severity, message });
  };
  for (const e of entries) {
    if (e.issues) for (const i of e.issues) push(e, i.kind, i.severity, i.message);
    const srcPh = placeholders(e.source);
    if (e.status === 'translated' && srcPh.size > 0) {
      const tgtPh = placeholders(e.target);
      if ([...srcPh].some((p) => !tgtPh.has(p))) {
        push(e, 'placeholder_mismatch', 'error', null);
      }
    }
    if (e.status === 'untranslated' && e.source.length >= SUSPECT_SOURCE_LEN) {
      push(e, 'untranslated_suspect', 'warning', null);
    }
  }
  out.sort((a, b) => {
    if (a.severity !== b.severity) return a.severity === 'error' ? -1 : 1;
    return KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind);
  });
  return out;
}

class ReviewStore {
  /** Currently opened issue in the context pane. */
  selectedId = $state<string | null>(null);
  /** Active category filter ('all' = no filter). */
  activeKind = $state<IssueKind | 'all'>('all');

  /** Session resolutions: issue id -> how it was closed. */
  resolutions = $state<Record<string, Resolution>>({});
  /** Reason captured for ignored issues. */
  reasons = $state<Record<string, string>>({});
  /** Inline fix editor state. */
  editingId = $state<string | null>(null);
  fixText = $state<Record<string, string>>({});
  /** Inline ignore form state. */
  ignoringId = $state<string | null>(null);
  ignoreDraft = $state('');

  issues = $derived.by(() => collectIssues(project.entries));

  /** Issues not yet resolved this session, honoring the category filter. */
  active = $derived(
    this.issues.filter((i) => !this.resolutions[i.id] && (this.activeKind === 'all' || i.kind === this.activeKind))
  );

  /** Per-kind counts over the still-open issues (chip badges). */
  countsByKind = $derived.by(() => {
    const acc: Record<string, number> = {};
    for (const i of this.issues) {
      if (this.resolutions[i.id]) continue;
      acc[i.kind] = (acc[i.kind] ?? 0) + 1;
    }
    return acc;
  });

  get selected(): ReviewIssue | null {
    return this.selectedId ? (this.issues.find((i) => i.id === this.selectedId) ?? null) : null;
  }

  resolution(id: string): Resolution | undefined {
    return this.resolutions[id];
  }

  open(issue: ReviewIssue) {
    this.selectedId = issue.id;
    this.editingId = null;
    this.ignoringId = null;
    // Keep the editor in sync: navigating there opens the same entry.
    project.select(issue.entryId);
  }

  closeContext() {
    this.selectedId = null;
    this.editingId = null;
    this.ignoringId = null;
  }

  setKind(kind: IssueKind | 'all') {
    this.activeKind = kind;
    const selected = this.selected;
    if (selected && kind !== 'all' && selected.kind !== kind) this.closeContext();
  }

  startFix(issue: ReviewIssue) {
    const entry = project.byId(issue.entryId);
    this.fixText[issue.id] = entry?.target ?? '';
    this.editingId = issue.id;
    this.ignoringId = null;
  }

  cancelFix() {
    this.editingId = null;
  }

  /** Mock of "fix and save": writes through the shared project store. */
  saveFix(issue: ReviewIssue) {
    const text = (this.fixText[issue.id] ?? '').trim();
    if (text) project.applySuggestion(issue.entryId, text, 'human');
    this.resolutions[issue.id] = 'fixed';
    this.editingId = null;
  }

  startIgnore(issue: ReviewIssue) {
    this.ignoringId = issue.id;
    this.ignoreDraft = '';
    this.editingId = null;
  }

  cancelIgnore() {
    this.ignoringId = null;
    this.ignoreDraft = '';
  }

  confirmIgnore(issue: ReviewIssue) {
    const reason = this.ignoreDraft.trim();
    if (!reason) return;
    this.reasons[issue.id] = reason;
    this.resolutions[issue.id] = 'ignored';
    this.ignoringId = null;
    this.ignoreDraft = '';
  }

  /** "I looked at it, it is fine": entry becomes translated, issue closes. */
  markReviewed(issue: ReviewIssue) {
    project.setStatus(issue.entryId, 'translated');
    this.resolutions[issue.id] = 'reviewed';
    this.editingId = null;
    this.ignoringId = null;
  }

  /** Drop resolutions that no longer match any live issue (dev-panel reset). */
  pruneStale(valid: Set<string>) {
    const stale = Object.keys(this.resolutions).filter((id) => !valid.has(id));
    if (stale.length === 0) return;
    for (const id of stale) {
      delete this.resolutions[id];
      delete this.reasons[id];
      delete this.fixText[id];
    }
    if (this.selectedId && !valid.has(this.selectedId)) this.selectedId = null;
  }

  /** W6 demo isolation: a fresh demo pass starts with a clean review session
   * (no stale resolutions/selection carried over from a previous run). */
  resetSession() {
    this.resolutions = {};
    this.reasons = {};
    this.fixText = {};
    this.selectedId = null;
    this.editingId = null;
    this.ignoringId = null;
    this.ignoreDraft = '';
    this.activeKind = 'all';
  }
}

export const review = new ReviewStore();
