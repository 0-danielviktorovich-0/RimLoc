// Chat-batch manager store (GUI_DESIGN_SPEC §10 external AI, wave W3):
// batches for the copy → paste-into-chat → import-back loop without an API
// key. Everything is a local mock — nothing calls a backend. Batch ids follow
// the stable `batch-3-of-8` format so replies map back by id, not by row
// order; retry children keep the same shape (`batch-3a-of-8`).

export type BatchStatus =
  | 'not_started'
  | 'exported'
  | 'waiting'
  | 'imported'
  | 'partial'
  | 'needs_review'
  | 'done';

export type SizingPreset = 'auto' | 'small' | 'medium' | 'large';
export type ChatProfileId = 'generic' | 'chatgpt' | 'claude' | 'gemini' | 'glm' | 'custom';

export interface AdvancedSizing {
  entries: number;
  tokenBudget: number;
  outputTokens: number;
  characters: number;
}

export interface RetrySplit {
  aId: string;
  bId: string;
  aSize: number;
  bSize: number;
  /** Entries already accepted before the split — they are preserved. */
  preserved: number;
}

export interface ChatBatch {
  id: string;
  index: number;
  /** Total batches in the run — part of the stable id. */
  total: number;
  size: number;
  status: BatchStatus;
  accepted: number;
  problems: number;
  /** Source changed since the draft — stale results are never applied. */
  stale: number;
  skipped: number;
  tokens: number;
  split: RetrySplit | null;
  /** Non-null for retry children of a split parent. */
  parent: string | null;
}

export interface ApplyPreview {
  ready: number;
  problems: number;
  stale: number;
  skipped: number;
  review: number;
}

/** Mock project context shown in the manager header and transparency summary. */
export const CHAT_PROJECT = {
  name: 'Vanilla Furniture Expanded',
  totalEntries: 76,
  sourceLocale: 'en',
  targetLocale: 'ru'
};

/** Rough per-entry token estimate for the transparency summary (mock). */
const TOKENS_PER_ENTRY = 110;

/** Auto sizing: derived for this mock project (76 entries in 8 batches). */
export const AUTO_SIZING: AdvancedSizing = {
  entries: 10,
  tokenBudget: 1200,
  outputTokens: 4000,
  characters: 16000
};

/** Named presets; "large" is what the 80 → 40+40 retry-split example assumes. */
export const SIZING_PRESETS: Record<Exclude<SizingPreset, 'auto'>, AdvancedSizing> = {
  small: { entries: 20, tokenBudget: 2400, outputTokens: 8000, characters: 32000 },
  medium: { entries: 40, tokenBudget: 4800, outputTokens: 16000, characters: 64000 },
  large: { entries: 80, tokenBudget: 9600, outputTokens: 32000, characters: 128000 }
};

/** Chat profiles are behavior defaults, not secrets: how a given chat likes
 * its prompts and replies. "Recommended" is derived from the target locale. */
export const CHAT_PROFILES: ChatProfileId[] = [
  'generic',
  'chatgpt',
  'claude',
  'gemini',
  'glm',
  'custom'
];

function estimateTokens(entries: number): number {
  return entries * TOKENS_PER_ENTRY;
}

function initialBatches(): ChatBatch[] {
  // Sizes sum to 76. The spread shows every status of the lifecycle at once
  // and leaves work for each manager action: copy, wait, import, apply,
  // review, split-retry.
  const rows: Array<
    Pick<ChatBatch, 'index' | 'size' | 'status'> &
      Partial<Pick<ChatBatch, 'accepted' | 'problems' | 'stale' | 'skipped'>>
  > = [
    { index: 1, size: 10, status: 'done', accepted: 10 },
    { index: 2, size: 10, status: 'done', accepted: 10 },
    { index: 3, size: 10, status: 'needs_review', accepted: 6, problems: 3, stale: 1 },
    { index: 4, size: 10, status: 'imported' },
    { index: 5, size: 10, status: 'partial', accepted: 7 },
    { index: 6, size: 10, status: 'waiting' },
    { index: 7, size: 10, status: 'exported' },
    { index: 8, size: 6, status: 'not_started' }
  ];
  const total = rows.length;
  return rows.map((r) => ({
    id: `batch-${r.index}-of-${total}`,
    index: r.index,
    total,
    size: r.size,
    status: r.status,
    accepted: r.accepted ?? 0,
    problems: r.problems ?? 0,
    stale: r.stale ?? 0,
    skipped: r.skipped ?? 0,
    tokens: estimateTokens(r.size),
    split: null,
    parent: null
  }));
}

class ChatBatchStore {
  batches = $state<ChatBatch[]>(initialBatches());

  /** Batch sizing: Auto by default; presets fill the advanced budgets. */
  sizing = $state<SizingPreset>('auto');
  advanced = $state<AdvancedSizing>({ ...AUTO_SIZING });

  /** Chat profile: behavior defaults for the target chat. */
  profile = $state<ChatProfileId>('glm');

  /** Id of the batch whose prompt was last copied (mock clipboard receipt). */
  copiedId = $state<string | null>(null);

  readonly totalEntries = CHAT_PROJECT.totalEntries;
  readonly targetLocale = CHAT_PROJECT.targetLocale;

  /** Estimated token volume of the whole external-AI run — the transparency
   * summary number (estimates, not billing: there is no API in this loop). */
  readonly runTokensEstimate = TOKENS_PER_ENTRY * CHAT_PROJECT.totalEntries;

  rootBatches = $derived(this.batches.filter((b) => b.parent === null));

  /** Globally accepted entries across all batches. */
  acceptedTotal = $derived(this.batches.reduce((s, b) => s + b.accepted, 0));

  /** Entries waiting for a human look: problems plus stale sources. */
  reviewTotal = $derived(this.batches.reduce((s, b) => s + b.problems + b.stale, 0));

  /** Next batch ready to be copied: not sent yet or partially sent. */
  nextToCopy = $derived(
    this.rootBatches.find((b) => b.status === 'not_started' || b.status === 'partial') ?? null
  );

  /** Chat profile suggested for the current target locale. */
  recommendedProfile = $derived<ChatProfileId>(this.targetLocale === 'ru' ? 'glm' : 'generic');

  byId(id: string): ChatBatch | undefined {
    return this.batches.find((b) => b.id === id);
  }

  childrenOf(id: string): ChatBatch[] {
    return this.batches.filter((b) => b.parent === id);
  }

  /** Entries of the batch not yet sent or still copyable (partial remainder). */
  copyableCount(b: ChatBatch): number {
    return Math.max(0, b.size - b.accepted - b.problems - b.stale - b.skipped);
  }

  /** What applying the import would do, before anything is written. */
  preview(b: ChatBatch): ApplyPreview {
    const ready = Math.max(0, b.size - b.accepted - b.problems - b.stale - b.skipped);
    return {
      ready,
      problems: b.problems,
      stale: b.stale,
      skipped: b.skipped,
      review: b.problems + b.stale + b.skipped
    };
  }

  setPreset(p: SizingPreset) {
    this.sizing = p;
    if (p !== 'auto') this.advanced = { ...SIZING_PRESETS[p] };
  }

  setProfile(id: ChatProfileId) {
    this.profile = id;
  }

  /** Copy the prompt of a batch (mock): mark it exported, remember the receipt. */
  copy(id: string) {
    const b = this.byId(id);
    if (!b || (b.status !== 'not_started' && b.status !== 'partial')) return;
    b.status = 'exported';
    this.copiedId = id;
  }

  copyAgain(id: string) {
    if (this.byId(id)) this.copiedId = id;
  }

  /** The prompt is pasted into the chat — now we wait for the reply. */
  markWaiting(id: string) {
    const b = this.byId(id);
    if (b && b.status === 'exported') b.status = 'waiting';
  }

  /** Paste the reply back: it gets mapped by stable ids and previewed. */
  importReply(id: string) {
    const b = this.byId(id);
    if (b && b.status === 'waiting') b.status = 'imported';
  }

  /** Apply the ready entries of an import; problems stay for human review. */
  apply(id: string) {
    const b = this.byId(id);
    if (!b) return;
    const pv = this.preview(b);
    b.accepted += pv.ready;
    if (pv.problems + pv.stale + pv.skipped === 0) {
      b.status = 'done';
    } else if (b.status !== 'needs_review') {
      b.status = 'needs_review';
    }
  }

  /** Human looked at the problem entries: problems accepted, stale preserved. */
  review(id: string) {
    const b = this.byId(id);
    if (!b) return;
    b.accepted += b.problems;
    b.skipped += b.stale;
    b.problems = 0;
    b.stale = 0;
    if (b.status === 'needs_review' || b.status === 'imported') b.status = 'done';
  }

  /** Retry a stuck batch by splitting it in two halves; accepted stay put. */
  split(id: string) {
    const b = this.byId(id);
    if (!b || b.split !== null) return;
    if (b.status !== 'needs_review' && b.status !== 'partial') return;
    const aSize = Math.ceil(b.size / 2);
    const bSize = b.size - aSize;
    b.split = {
      aId: `batch-${b.index}a-of-${b.total}`,
      bId: `batch-${b.index}b-of-${b.total}`,
      aSize,
      bSize,
      preserved: b.accepted
    };
    const child = (childId: string, size: number): ChatBatch => ({
      id: childId,
      index: b.index,
      total: b.total,
      size,
      status: 'not_started',
      accepted: 0,
      problems: 0,
      stale: 0,
      skipped: 0,
      tokens: estimateTokens(size),
      split: null,
      parent: b.id
    });
    this.batches.push(child(b.split.aId, aSize), child(b.split.bId, bSize));
  }
}

export const chat = new ChatBatchStore();
