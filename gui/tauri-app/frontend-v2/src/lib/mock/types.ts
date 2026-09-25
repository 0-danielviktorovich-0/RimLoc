// Mock domain model for the Translator Workspace.
// These types stand in for the backend service contract (spec §3.1) while the
// backend stabilises; the GUI layer must stay isolated behind these shapes.

export type EntryStatus =
  | 'untranslated'
  | 'translated'
  | 'todo'
  | 'sourceChanged'
  | 'pending_review'
  | 'orphan';

export type EntryKind = 'Keyed' | 'DefInjected' | 'TKey';

/** RimWorld TKey serialization strategy (spec: bare node / .slateRef / .value.slateRef). */
export type TkeyStrategy = 'bare' | '.slateRef' | '.value.slateRef';

export type Origin = 'human' | 'TM' | 'LLM' | 'imported';

/** One TKey multi-context: where else this string appears in the XML. */
export interface EntryContext {
  node: string;
  file: string;
  line: number;
  sourceValue: string;
  sibling: string;
}

/** Review-queue issue categories (mandate §12). */
export type IssueKind =
  | 'placeholder_mismatch'
  | 'glossary'
  | 'untranslated_suspect'
  | 'wordinfo'
  | 'ambiguity'
  | 'ai_concern';

export type IssueSeverity = 'error' | 'warning';

export interface EntryIssue {
  kind: IssueKind;
  severity: IssueSeverity;
  /** Human-readable mock message (data language, not localized). */
  message: string;
}

/** Provenance history events for the HISTORY tab (mandate §11). */
export type HistoryAction =
  | 'created'
  | 'edited'
  | 'tm_match'
  | 'ai_draft'
  | 'imported'
  | 'source_changed'
  | 'marked_review';

export interface HistoryEvent {
  at: string;
  action: HistoryAction;
  origin?: Origin;
  /** Model id for AI events (mock). */
  model?: string;
  detail?: string;
}

/** SUGGESTIONS tab content: TM matches, glossary terms, AI drafts. */
export interface Suggestion {
  source: 'TM' | 'glossary' | 'LLM';
  /** TM similarity percent (0-100). */
  similarity?: number;
  /** Glossary term this suggestion is based on. */
  term?: string;
  text: string;
}

export interface Entry {
  id: string;
  kind: EntryKind;
  /** defName + path for DefInjected, key name for Keyed, TKey path otherwise. */
  key: string;
  source: string;
  target: string;
  status: EntryStatus;
  file: string;
  line: number;
  /** TKey only: which serialization strategy produced this record. */
  strategy?: TkeyStrategy;
  origin?: Origin | null;
  /** Contract validation dimension (audit P1-1): 'ok'/'issues' come from the
   *  canonical Translation; absent = fixture records without contract state
   *  (the DetailPanel falls back to the status proxy for those). */
  validation?: 'unknown' | 'ok' | 'issues';
  /** Issue strings when validation === 'issues' (canonical ValidationState). */
  validationIssues?: string[];
  editedAt?: string | null;
  note?: string;
  /** TKey multi-contexts; absent for plain Keyed/DefInjected records. */
  contexts?: EntryContext[];
  /** Where else this string surfaces in the product (user-oriented context). */
  usages?: string[];
  /** Previous source text for sourceChanged entries. */
  sourcePrev?: string;
  /** Review-queue problems attached to this record. */
  issues?: EntryIssue[];
  /** Provenance trail for the HISTORY tab. */
  history?: HistoryEvent[];
  /** TM / glossary / AI suggestions for the SUGGESTIONS tab. */
  suggestions?: Suggestion[];
}

export type SaveState = 'idle' | 'dirty' | 'saving' | 'saved';
