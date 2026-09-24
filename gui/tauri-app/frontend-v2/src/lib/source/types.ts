// Typed source seam (SOURCE_INSPECTOR_MANDATE §1, UI_SDK mandate). These are
// the shapes the future framework-neutral RimLocClient `source.*` namespace
// must return; today they are produced by RimLoc-owned synthetic fixtures
// (mock/pre-freeze), tomorrow by the real backend — components never know
// which. Source FACTS (file identity, line guarantees, ACTIVE/shadowed
// candidates, provenance reasons) always arrive as DATA; the GUI computes no
// precedence and runs no scanner (real resolver ownership: backend/J).
//
// Provenance vocabulary mirrors the canonical domain model
// (rimloc-domain canonical.rs SourceProvenance.selected_by) — same strings,
// same semantics, no GUI-local second vocabulary.
export type SourceProvenanceKind =
  | 'version-selected'
  | 'loadfolders'
  | 'first-file-wins'
  | 'keyed-last-wins'
  | 'patch-applied'
  | 'definition';

/** One provenance fact, already resolved by the backend/fixture. */
export interface ProvenanceFact {
  kind: SourceProvenanceKind;
  /** Human-readable detail in data language (demo fixtures ship en text). */
  detail: string;
}

export type SourceFileKind = 'effective' | 'original' | 'generated';

export interface SourceFileRef {
  path: string;
  /** Human path for headers/breadcrumbs (demo: relative to the demo mod). */
  displayPath: string;
  kind: SourceFileKind;
  versionLabel?: string;
  loadFolder?: string;
}

/** A highlighted node region inside a file. 1-based inclusive; null spans
 *  mean the position is NOT guaranteed and must never be faked (§1). */
export interface SourceNodeSpan {
  /** Stable node id; entry-mapped ids enable node→entry navigation. */
  nodeId: string;
  entryId?: string;
  start: number | null;
  end: number | null;
}

export interface SourceFileContent extends SourceFileRef {
  /** File body as literal lines (no parsing in the GUI). */
  lines: string[];
  /** true only when the fixture/backend GUARANTEES numbering (§1, §2). */
  lineNumbersGuaranteed: boolean;
  spans: SourceNodeSpan[];
  /** Breadcrumb chain: mod → version/load folder → … → file. */
  breadcrumb: string[];
}

/** Soft missing state (§2): recorded location does not resolve. */
export interface SourceFileMissing {
  path: string;
  displayPath: string;
  kind: SourceFileKind;
  missing: true;
  reasonKey: string;
}

export type SourceFile = SourceFileContent | SourceFileMissing;

export function isMissingFile(f: SourceFile): f is SourceFileMissing {
  return 'missing' in f && f.missing === true;
}

export type SourceUsageRole = 'primary' | 'other';

/** One resolved usage of an identity (mandate §14: primary + other usages —
 *  TKey/multi-context is never flattened into one fake source). */
export interface SourceUsage {
  role: SourceUsageRole;
  effective: boolean;
  location: {
    path: string;
    displayPath: string;
    line: number | null;
    column: number | null;
    nodePath: string;
  };
  provenance: ProvenanceFact[];
}

/** Structured inline excerpt (§4): identity-anchored, not "N raw lines". */
export interface SourceExcerpt {
  defName: string;
  field: string;
  related: { labelKey: string; value: string }[];
}

export interface SourceEntryData {
  entryId: string;
  usages: SourceUsage[];
  excerpt: SourceExcerpt;
}

/** Browser candidate (§6): ACTIVE vs shadowed, reason is DATA. */
export interface SourceCandidate {
  path: string;
  displayPath: string;
  status: 'active' | 'shadowed';
  reasonKey: string;
  versionLabel?: string;
  loadFolder?: string;
}

export interface SourceBrowserData {
  modId: string;
  modName: string;
  packageId: string;
  versions: { label: string; selected: boolean }[];
  loadFolders: { label: string; selected: boolean }[];
  categories: { name: string; files: { path: string; displayPath: string }[] }[];
  candidates: SourceCandidate[];
}

/** Compare triple (§10): original source ↔ canonical target ↔ generated
 *  output. The generated output is NEVER a second source of truth. */
export interface CompareTriple {
  entryId: string;
  sourceText: string;
  targetText: string | null;
  generated: SourceFileContent;
  generatedSpan: SourceNodeSpan | null;
}

/** Structured external-editor launch plan (§8): executable + argument array;
 *  NO shell strings, paths/substitutions are literal array items. */
export interface EditorLaunchPlan {
  executable: string;
  args: string[];
}

export type EditorLaunchPlanResult =
  | { ok: true; plan: EditorLaunchPlan }
  | { ok: false; reasonKey: string };

/** Mock OS-action honesty (§9): demo states never claim a real launch. */
export interface MockActionFeedback {
  id: number;
  labelKey: string;
  params?: Record<string, string | number>;
}

/** External change (§12) — mock transitions, no watcher/FS mutation. */
export interface ExternalChangeFixture {
  file: string;
  displayPath: string;
  summaryKey: string;
  oldExcerpt: string[];
  newExcerpt: string[];
}

export interface SourceScenario {
  id: string;
  /** Stable machine id used by the DevPanel/scenario browser (W6 reads this
   *  list via SOURCE_SCENARIOS; ownership of the panel stays with W6). */
  entries: Record<string, SourceEntryData>;
  files: Record<string, SourceFile>;
  browser: SourceBrowserData;
  compare: Record<string, CompareTriple>;
  externalChange?: ExternalChangeFixture;
}
