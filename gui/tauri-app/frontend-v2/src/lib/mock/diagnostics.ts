// Diagnostics mock layer (W5, GUI_QA_MANDATE §16-§17 + AUTONOMOUS_STATUS
// "W4.5/W5 integration requirements" items 5-6):
//
//   1. CONTROLLED KNOWN FAILURE — a scripted validation failure ("placeholder
//      mismatch in keyed-04") that Diagnose replays on demand. The acceptance
//      bar is NOT "a bundle was generated": the causal context produced here
//      must be small, structured and sufficient for an independent reviewer
//      (human or AI) to name the probable cause. A short relevant trace beats
//      raw logs.
//   2. SUPPORT BUNDLE REDACTION PREVIEW — the raw bundle lines are passed
//      through a real rule engine before display, so the Included / Redacted /
//      Excluded sections demonstrate automatic protection (credentials, auth
//      headers, tokens, keychain, signing, non-essential env values, home
//      path normalization, third-party game content) while keeping the
//      diagnostic payload useful.
//
// Pure data + pure functions: no Svelte runes, no DOM. The store
// (stores/diagnostics.svelte.ts) owns the reactive phases.

/** Canonical app/package metadata (W4 requirement 4: one source, not a hand string). */
export const APP_VERSION = '0.1.0';

/** RimWorld version of the mock project (matches workspace.meta.version). */
export const RW_VERSION = 'RimWorld 1.6';

/** Validator identity surfaced in the causal context. */
export const VALIDATOR_ID = 'rimloc-structural-validator';
export const VALIDATOR_VERSION = '1.6';

/** The controlled-failure fixture: the exact entry the validator choked on. */
export const SCENARIO_ENTRY = {
  id: 'keyed-04',
  key: 'ConfirmBanish',
  file: 'Languages/English/Keyed/Dialogs.xml',
  line: 41,
  /** Source placeholders (game content values themselves are NOT shipped). */
  placeholders: ['{PAWN_nameDef}'],
  /** What the pipeline produced for this entry (the human/AI draft). */
  draftTarget: 'Изгнать Ивана?'
} as const;

export type TraceLevel = 'info' | 'warn' | 'error';

/** One structured trace event — milliseconds offset from operation start. */
export interface TraceEvent {
  atMs: number;
  level: TraceLevel;
  phase: string;
  message: string;
}

/** Structured causal context: the "why" a reviewer actually needs. */
export interface CausalContext {
  /** Operation id in the op-xxxxxx format (one per diagnostics run). */
  operationId: string;
  stage: 'validate';
  startedAt: string;
  durationMs: number;
  errorCode: 'PLACEHOLDER_MISMATCH';
  expected: string;
  actual: string;
  affectedEntries: Array<{
    id: string;
    key: string;
    file: string;
    line: number;
    placeholders: readonly string[];
  }>;
  providerState: {
    connected: number;
    notConfigured: number;
    offline: number;
  };
  validator: string;
  rimworldVersion: string;
  locales: { source: string; target: string; ui: string };
  /** Small on purpose: ~6 events, not a log dump. */
  trace: TraceEvent[];
}

function shortHex(n: number): string {
  let out = '';
  for (let i = 0; i < n; i++) out += Math.floor(Math.random() * 16).toString(16);
  return out;
}

/** op-xxxxxx: readable operation id for support conversations. */
export function newOperationId(stage: string): string {
  return `op-${stage}-${shortHex(6)}`;
}

export interface CausalEnv {
  providerCounts: { connected: number; notConfigured: number; offline: number };
  sourceLocale: string;
  targetLocale: string;
  uiLocale: string;
  /** Simulated validation latency shown in timings. */
  validateMs?: number;
}

/**
 * Build the causal context of the controlled known failure. The trace is
 * deliberately a handful of events (mandate: "малый релевантный trace, не
 * лог-флуд") — enough to follow operation start → inventory → the failing
 * check → verdict.
 */
export function buildCausalContext(env: CausalEnv): CausalContext {
  const validateMs = env.validateMs ?? 142;
  return {
    operationId: newOperationId('vld'),
    stage: 'validate',
    startedAt: new Date().toISOString(),
    durationMs: 214,
    errorCode: 'PLACEHOLDER_MISMATCH',
    expected: `target keeps every source placeholder: ${SCENARIO_ENTRY.placeholders.join(' ')} (1)`,
    actual: `target contains none of them (0) — draft «${SCENARIO_ENTRY.draftTarget}»`,
    affectedEntries: [
      {
        id: SCENARIO_ENTRY.id,
        key: SCENARIO_ENTRY.key,
        file: SCENARIO_ENTRY.file,
        line: SCENARIO_ENTRY.line,
        placeholders: SCENARIO_ENTRY.placeholders
      }
    ],
    providerState: { ...env.providerCounts },
    validator: `${VALIDATOR_ID}/${VALIDATOR_VERSION}`,
    rimworldVersion: RW_VERSION,
    locales: { source: env.sourceLocale, target: env.targetLocale, ui: env.uiLocale },
    trace: [
      { atMs: 0, level: 'info', phase: 'validate', message: `operation started (${VALIDATOR_ID}/${VALIDATOR_VERSION})` },
      { atMs: 12, level: 'info', phase: 'validate', message: `source inventory read: 60 entries, locale ${env.sourceLocale} → ${env.targetLocale}` },
      { atMs: 38, level: 'info', phase: 'validate', message: `inventory placeholder index built (Keyed 22 · DefInjected 30 · TKey 8)` },
      { atMs: 96, level: 'info', phase: 'validate', message: `entry ${SCENARIO_ENTRY.id} (${SCENARIO_ENTRY.key}) → placeholder check` },
      {
        atMs: validateMs,
        level: 'error',
        phase: 'validate',
        message: `PLACEHOLDER_MISMATCH in ${SCENARIO_ENTRY.id}: expected ${SCENARIO_ENTRY.placeholders.join(' ')}, found none`
      },
      { atMs: 214, level: 'info', phase: 'validate', message: 'operation failed: causal context assembled (1 affected entry)' }
    ]
  };
}

// ---------------------------------------------------------------------------
// Support bundle: raw lines → rule engine → Included / Redacted / Excluded
// ---------------------------------------------------------------------------

/** One raw (unsanitized) bundle line as the pipeline would emit it. */
export interface RawBundleLine {
  key: string;
  value: string;
  /** i18n label key for the preview (bundle.item.<key>). */
  labelKey: string;
  /** Declared excluded by design: the pipeline never puts this into a bundle. */
  excludedByDesign?: string;
}

export type BundleRuleId =
  | 'api-key'
  | 'auth-header'
  | 'token'
  | 'keychain'
  | 'signing'
  | 'env-secret'
  | 'third-party-content';

/** How one raw line survived sanitization. */
export interface BundleItem {
  key: string;
  labelKey: string;
  /** Final (possibly normalized) display value. */
  value: string;
  state: 'included' | 'redacted' | 'excluded';
  /** i18n key of the redaction/exclusion reason (bundle.reason.<id>). */
  reasonKey?: string;
  ruleId?: BundleRuleId | 'excluded-by-design';
  /** True when only the home prefix was rewritten (~), value stays included. */
  normalized?: boolean;
}

export interface BundlePreview {
  included: BundleItem[];
  redacted: BundleItem[];
  excluded: BundleItem[];
  counts: { included: number; redacted: number; excluded: number };
}

/**
 * The raw bundle of the controlled failure. Contains deliberately dangerous
 * lines (a fake API key, bearer token, keychain reference, absolute home
 * paths, quoted game text) so the preview proves the protection instead of
 * claiming it.
 */
export function buildRawBundle(ctx: CausalContext | null, extra: { sourceLocation: string; outputLocation: string }): RawBundleLine[] {
  const opId = ctx?.operationId ?? 'op-vld-000000';
  return [
    { key: 'app.version', value: `RimLoc ${APP_VERSION} (mock)`, labelKey: 'bundle.item.appVersion' },
    { key: 'causal.operation', value: opId, labelKey: 'bundle.item.operation' },
    { key: 'causal.stage', value: ctx?.stage ?? 'validate', labelKey: 'bundle.item.stage' },
    { key: 'causal.error', value: ctx ? `${ctx.errorCode}: ${ctx.expected} / ${ctx.actual}` : 'PLACEHOLDER_MISMATCH (known failure replay)', labelKey: 'bundle.item.error' },
    {
      key: 'causal.affected',
      value: ctx
        ? ctx.affectedEntries.map((e) => `${e.id} (${e.key}) @ ${e.file}:${e.line}`).join('; ')
        : `${SCENARIO_ENTRY.id} (${SCENARIO_ENTRY.key}) @ ${SCENARIO_ENTRY.file}:${SCENARIO_ENTRY.line}`,
      labelKey: 'bundle.item.affected'
    },
    // Third-party content: the source string belongs to the game, not to us —
    // its VALUE must not leave the machine even though the metadata above is
    // exactly what a reviewer needs.
    { key: 'entry.source.value', value: `Banish ${SCENARIO_ENTRY.placeholders.join(' ')}? — full game string`, labelKey: 'bundle.item.entrySource' },
    // The user's own draft is diagnostic gold and is the user's own content.
    { key: 'entry.target.draft', value: SCENARIO_ENTRY.draftTarget, labelKey: 'bundle.item.entryDraft' },
    { key: 'env.rimworld', value: ctx?.rimworldVersion ?? RW_VERSION, labelKey: 'bundle.item.rimworld' },
    { key: 'env.locales', value: `source ${ctx?.locales.source ?? 'en'} → target ${ctx?.locales.target ?? 'ru'} · ui ${ctx?.locales.ui ?? 'ru'}`, labelKey: 'bundle.item.locales' },
    { key: 'timing.validate', value: ctx ? `${ctx.durationMs} ms (validator ${ctx.validator})` : '142 ms', labelKey: 'bundle.item.timing' },
    {
      key: 'providers.state',
      value: ctx
        ? `connected ${ctx.providerState.connected} · not configured ${ctx.providerState.notConfigured} · offline ${ctx.providerState.offline}`
        : 'connected 1 · not configured 1 · offline 1',
      labelKey: 'bundle.item.providers'
    },
    // Paths arrive absolute from the OS — normalized to ~ before display so a
    // username never leaves the machine, while remaining fully useful.
    { key: 'path.source_location', value: extra.sourceLocation, labelKey: 'bundle.item.sourceLocation' },
    { key: 'path.output_location', value: extra.outputLocation, labelKey: 'bundle.item.outputLocation' },
    // --- dangerous lines the engine must catch ---
    { key: 'provider.openai.api_key', value: 'sk-proj-9f2XkQ7Lm3vT8wZ1cD4b', labelKey: 'bundle.item.apiKey' },
    { key: 'provider.request.headers', value: 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.mock', labelKey: 'bundle.item.authHeader' },
    { key: 'provider.openai.license_token', value: 'token=ghp_4tX9kQ7Lm3vT8wZ1cD4bN2pR6yU1sE8vW5q', labelKey: 'bundle.item.token' },
    { key: 'env.RIMLOC_PROVIDER_KEY', value: 'sk-proj-AAAAsecretvalueAAAA', labelKey: 'bundle.item.envSecret' },
    { key: 'env.RIMLOC_LOCALE', value: 'ru', labelKey: 'bundle.item.envLocale' },
    { key: 'credentials.keychain_ref', value: 'keychain://rimloc/openai (service tilda-… style refs)', labelKey: 'bundle.item.keychain' },
    { key: 'build.signing_identity', value: 'Developer ID Application: OOO PEREZAGRUZKA (TEAM1234AB)', labelKey: 'bundle.item.signing' },
    // --- excluded by design: never bundled, shown in the Excluded section ---
    { key: 'session.full_log', value: 'app.log (2.4 MB, 18 402 lines)', labelKey: 'bundle.item.fullLog', excludedByDesign: 'too-large' },
    { key: 'project.database', value: 'project.db — full inventory + translations', labelKey: 'bundle.item.database', excludedByDesign: 'bulk-user-content' },
    { key: 'screenshots', value: 'game screenshots attached by the user', labelKey: 'bundle.item.screenshots', excludedByDesign: 'third-party-visuals' }
  ];
}

const REDACTION_RULES: Array<{
  id: BundleRuleId;
  test: (key: string, value: string) => boolean;
  reasonKey: string;
}> = [
  { id: 'api-key', reasonKey: 'bundle.reason.apiKey', test: (_k, v) => /\bsk-[a-z0-9-]{8,}/i.test(v) },
  { id: 'auth-header', reasonKey: 'bundle.reason.authHeader', test: (_k, v) => /\bauthorization\s*:|\bbearer\s+\S/i.test(v) },
  { id: 'token', reasonKey: 'bundle.reason.token', test: (k, v) => /\btoken\b/i.test(k) || /\b(token|ghp_)[=:]\s*\S/i.test(v) || /\bghp_[a-z0-9]{16,}/i.test(v) },
  { id: 'env-secret', reasonKey: 'bundle.reason.envSecret', test: (k, v) => /^env\./i.test(k) && /(key|secret|token|password)/i.test(k) },
  { id: 'keychain', reasonKey: 'bundle.reason.keychain', test: (k, v) => /keychain/i.test(k) || /keychain:\/\//i.test(v) },
  { id: 'signing', reasonKey: 'bundle.reason.signing', test: (k, v) => /signing/i.test(k) || /(developer id application|codesign)/i.test(v) },
  { id: 'third-party-content', reasonKey: 'bundle.reason.thirdParty', test: (k, _v) => /^entry\.source\./i.test(k) }
];

const REDACTED_PLACEHOLDER = '[REDACTED]';

/** Normalize an absolute home path (`/Users/name/...` → `~/...`) for display. */
export function normalizeHomePath(value: string, homePrefix = '/Users/danielviktorovich'): string {
  return value.startsWith(homePrefix) ? `~${value.slice(homePrefix.length)}` : value;
}

/**
 * Run the raw bundle through the redaction engine. Home paths are normalized
 * (kept — a path is diagnostic value), secrets and third-party content are
 * redacted with a visible reason, by-design exclusions land in Excluded.
 */
export function sanitizeBundle(lines: RawBundleLine[], homePrefix?: string): BundlePreview {
  const included: BundleItem[] = [];
  const redacted: BundleItem[] = [];
  const excluded: BundleItem[] = [];

  for (const line of lines) {
    if (line.excludedByDesign) {
      excluded.push({
        key: line.key,
        labelKey: line.labelKey,
        value: line.value,
        state: 'excluded',
        reasonKey: `bundle.reason.${line.excludedByDesign}`,
        ruleId: 'excluded-by-design'
      });
      continue;
    }

    const rule = REDACTION_RULES.find((r) => r.test(line.key, line.value));
    if (rule) {
      redacted.push({
        key: line.key,
        labelKey: line.labelKey,
        value: REDACTED_PLACEHOLDER,
        state: 'redacted',
        reasonKey: rule.reasonKey,
        ruleId: rule.id
      });
      continue;
    }

    const normalizedValue = normalizeHomePath(line.value, homePrefix);
    const normalized = normalizedValue !== line.value;
    included.push({
      key: line.key,
      labelKey: line.labelKey,
      value: normalizedValue,
      state: 'included',
      normalized
    });
  }

  return {
    included,
    redacted,
    excluded,
    counts: { included: included.length, redacted: redacted.length, excluded: excluded.length }
  };
}

/**
 * Compact "copy for AI" text: a prompt + the sanitized payload (included
 * values verbatim, redactions marked). The reviewer must be able to name the
 * probable cause from this alone.
 */
export function buildAiPrompt(preview: BundlePreview): string {
  const head = [
    'RimLoc bug report — please analyze the failure below.',
    'The bundle is sanitized: secrets and third-party game content are redacted on purpose, the causal payload is intact.'
  ];
  const body = [
    ...preview.included.map((i) => `${i.key}: ${i.value}`),
    ...preview.redacted.map((i) => `${i.key}: ${REDACTED_PLACEHOLDER} (${i.ruleId})`)
  ];
  const tail = [
    `Bundle integrity: included ${preview.counts.included} · redacted ${preview.counts.redacted} · excluded ${preview.counts.excluded}.`,
    'Task: name the most probable root cause, the failing entry, and how to reproduce it.'
  ];
  return [...head, '', ...body, '', ...tail].join('\n');
}
