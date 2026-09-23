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
  editedAt?: string | null;
  note?: string;
  /** TKey multi-contexts; absent for plain Keyed/DefInjected records. */
  contexts?: EntryContext[];
}

export type SaveState = 'idle' | 'dirty' | 'saving' | 'saved';
