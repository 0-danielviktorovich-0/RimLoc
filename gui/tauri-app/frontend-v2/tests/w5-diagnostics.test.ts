// W5 regression: diagnostics causal context, support-bundle redaction and the
// diagnostics store phase machine (mandate §16-§17, W4.5/W5 items 5-6).
// Pure-function coverage over mock/diagnostics.ts + fake-timer coverage over
// the store. Fixtures are synthetic generic paths only — no real usernames,
// companies or workshop ids.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  MOCK_APP_VERSION,
  MOCK_HOME_PREFIX,
  SCENARIO_ENTRY,
  buildAiPrompt,
  buildCausalContext,
  buildRawBundle,
  normalizeHomePath,
  sanitizeBundle,
  type CausalContext,
  type RawBundleLine
} from '../src/lib/mock/diagnostics';
import { SOURCE_LOCATION, OUTPUT_LOCATION, DiagnosticsStore } from '../src/lib/stores/diagnostics.svelte';
import { providers } from '../src/lib/stores/providers.svelte';

const CAUSAL_ENV = {
  providerCounts: { connected: 1, notConfigured: 2, offline: 1 },
  sourceLocale: 'en',
  targetLocale: 'ru',
  uiLocale: 'en'
};

const SECRET_VALUES = [
  'sk-proj-9f2XkQ7Lm3vT8wZ1cD4b',
  'Bearer eyJhbGciOiJIUzI1NiJ9.mock',
  'ghp_4tX9kQ7Lm3vT8wZ1cD4bN2pR6yU1sE8vW5q',
  'sk-proj-AAAAsecretvalueAAAA',
  'plain-sensitive',
  'opaque'
];

describe('causal context (controlled failure)', () => {
  const ctx = buildCausalContext(CAUSAL_ENV);

  it('formats the operation id as op-<stage>-<6 hex>', () => {
    expect(ctx.operationId).toMatch(/^op-vld-[0-9a-f]{6}$/);
    // Each run gets a fresh operation id.
    expect(buildCausalContext(CAUSAL_ENV).operationId).not.toBe(ctx.operationId);
  });

  it('names expected vs actual around the placeholder loss', () => {
    expect(ctx.stage).toBe('validate');
    expect(ctx.errorCode).toBe('PLACEHOLDER_MISMATCH');
    expect(ctx.expected).toContain('{PAWN_nameDef}');
    expect(ctx.expected).toContain('(1)');
    expect(ctx.actual).toContain('(0)');
    expect(ctx.actual).toContain(SCENARIO_ENTRY.draftTarget);
  });

  it('scopes affected entries to the failing one with placeholders', () => {
    expect(ctx.affectedEntries).toHaveLength(1);
    const entry = ctx.affectedEntries[0];
    expect(entry.id).toBe(SCENARIO_ENTRY.id);
    expect(entry.placeholders).toContain('{PAWN_nameDef}');
    expect(entry.file).toContain('Dialogs.xml');
  });

  it('keeps the trace small and relevant, with the error visible', () => {
    expect(ctx.trace.length).toBeLessThanOrEqual(8);
    expect(ctx.trace.filter((e) => e.level === 'error')).toHaveLength(1);
    expect(ctx.trace.at(-1)?.message).toContain('causal context assembled');
  });

  it('carries environment facts (providers, validator, RW version, locales, timings)', () => {
    expect(ctx.providerState).toEqual(CAUSAL_ENV.providerCounts);
    expect(ctx.validator).toContain('rimloc-structural-validator');
    expect(ctx.rimworldVersion).toContain('1.6');
    expect(ctx.locales).toEqual({ source: 'en', target: 'ru', ui: 'en' });
    expect(ctx.durationMs).toBeGreaterThan(0);
  });
});

describe('bundle sanitization (Included / Redacted / Excluded)', () => {
  const ctx: CausalContext = buildCausalContext(CAUSAL_ENV);
  const preview = sanitizeBundle(buildRawBundle(ctx, { sourceLocation: SOURCE_LOCATION, outputLocation: OUTPUT_LOCATION }));

  it('splits every line into exactly one of the three sections', () => {
    const total = preview.counts.included + preview.counts.redacted + preview.counts.excluded;
    expect(total).toBe(buildRawBundle(ctx, { sourceLocation: 'x', outputLocation: 'y' }).length);
    expect(preview.included.every((i) => i.state === 'included')).toBe(true);
    expect(preview.redacted.every((i) => i.state === 'redacted' && i.reasonKey)).toBe(true);
    expect(preview.excluded.every((i) => i.state === 'excluded' && i.reasonKey)).toBe(true);
  });

  it('keeps the causal payload and the user draft intact', () => {
    const keys = preview.included.map((i) => i.key);
    for (const key of ['causal.operation', 'causal.error', 'causal.affected', 'entry.target.draft', 'env.rimworld', 'providers.state']) {
      expect(keys).toContain(key);
    }
    expect(preview.included.find((i) => i.key === 'entry.target.draft')?.value).toBe(SCENARIO_ENTRY.draftTarget);
    expect(preview.included.find((i) => i.key === 'causal.operation')?.value).toBe(ctx.operationId);
  });

  it('normalizes home paths to ~ instead of dropping them', () => {
    const source = preview.included.find((i) => i.key === 'path.source_location');
    expect(source?.normalized).toBe(true);
    expect(source?.value.startsWith('~/')).toBe(true);
    expect(preview.included.map((i) => i.value).join('\n')).not.toContain(MOCK_HOME_PREFIX);
  });

  it('redacts value-shaped secrets: api keys, bearer headers, tokens', () => {
    const redacted = preview.redacted;
    expect(redacted.map((i) => i.key)).toEqual(
      expect.arrayContaining(['provider.openai.api_key', 'provider.request.headers', 'provider.openai.license_token', 'env.RIMLOC_PROVIDER_KEY'])
    );
    expect(redacted.every((i) => i.value === '[REDACTED]')).toBe(true);
  });

  it('redacts third-party game content but keeps the structural metadata', () => {
    expect(preview.redacted.find((i) => i.key === 'entry.source.value')?.ruleId).toBe('third-party-content');
    expect(preview.included.find((i) => i.key === 'causal.affected')?.value).toContain(SCENARIO_ENTRY.id);
  });

  it('redacts plain values when the FIELD NAME is sensitive (finding 4)', () => {
    const raw: RawBundleLine[] = [
      { key: 'provider.password', value: 'plain-sensitive', labelKey: 'bundle.item.apiKey' },
      { key: 'provider.apiKey', value: 'opaque', labelKey: 'bundle.item.apiKey' },
      { key: 'credentials.secret_ref', value: 'harmless-lookalike', labelKey: 'bundle.item.keychain' },
      { key: 'provider.display_name', value: 'Ollama (local)', labelKey: 'bundle.item.providers' }
    ];
    const p = sanitizeBundle(raw);
    expect(p.redacted.map((i) => i.key)).toEqual(
      expect.arrayContaining(['provider.password', 'provider.apiKey', 'credentials.secret_ref'])
    );
    // A non-sensitive name with a benign value stays included.
    expect(p.included.find((i) => i.key === 'provider.display_name')?.value).toBe('Ollama (local)');
  });

  it('masks excluded-by-design values that would leak a secret (finding 4)', () => {
    const raw: RawBundleLine[] = [
      { key: 'session.full_log', value: 'app.log — retry used key sk-proj-abcdef12345678', labelKey: 'bundle.item.fullLog', excludedByDesign: 'too-large' },
      { key: 'screenshots', value: 'game screenshots attached by the user', labelKey: 'bundle.item.screenshots', excludedByDesign: 'third-party-visuals' }
    ];
    const p = sanitizeBundle(raw);
    const log = p.excluded.find((i) => i.key === 'session.full_log');
    expect(log?.state).toBe('excluded');
    expect(log?.value).toBe('[REDACTED]');
    expect(p.excluded.find((i) => i.key === 'screenshots')?.value).not.toBe('[REDACTED]');
  });
});

describe('copy-for-AI text (the artifact a reviewer actually reads)', () => {
  const ctx = buildCausalContext(CAUSAL_ENV);
  const preview = sanitizeBundle(buildRawBundle(ctx, { sourceLocation: SOURCE_LOCATION, outputLocation: OUTPUT_LOCATION }));
  const prompt = buildAiPrompt(preview);

  it('contains no secret values, raw or generic (finding 4)', () => {
    for (const secret of SECRET_VALUES) {
      expect(prompt).not.toContain(secret);
    }
  });

  it('contains no absolute home paths and no game source string', () => {
    expect(prompt).not.toContain(MOCK_HOME_PREFIX);
    expect(prompt).not.toContain(SOURCE_LOCATION);
    expect(prompt).toContain('[REDACTED]');
    expect(prompt).toContain('third-party-content');
  });

  it('is sufficient to name the probable cause (acceptance item 5)', () => {
    expect(prompt).toContain(ctx.operationId);
    expect(prompt).toContain('PLACEHOLDER_MISMATCH');
    expect(prompt).toContain('{PAWN_nameDef}');
    expect(prompt).toContain(SCENARIO_ENTRY.id);
    expect(prompt).toContain('root cause');
  });
});

describe('diagnostics store phase machine', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function run(store: DiagnosticsStore) {
    store.runScenario();
    vi.advanceTimersByTime(2000);
  }

  it('goes idle → running → diagnosed with a fresh causal context', () => {
    const store = new DiagnosticsStore();
    expect(store.phase).toBe('idle');
    store.runScenario();
    expect(store.phase).toBe('running');
    vi.advanceTimersByTime(2000);
    expect(store.phase).toBe('diagnosed');
    expect(store.causal).not.toBeNull();
    expect(store.causal?.operationId).toMatch(/^op-vld-[0-9a-f]{6}$/);
  });

  it('rerun drops the stale operation immediately (finding 3)', () => {
    const store = new DiagnosticsStore();
    run(store);
    const firstId = store.causal?.operationId;
    store.prepareBundle();
    expect(store.bundle).not.toBeNull();

    store.runScenario();
    // While the new run is in flight, NOTHING of the old operation may be
    // readable or copyable as fresh.
    expect(store.phase).toBe('running');
    expect(store.causal).toBeNull();
    expect(store.bundle).toBeNull();
    expect(store.aiPrompt).toBe('');

    vi.advanceTimersByTime(2000);
    expect(store.causal?.operationId).not.toBe(firstId);
  });

  it('reset mid-run cancels pending timers and stale results (finding 3)', () => {
    const store = new DiagnosticsStore();
    store.runScenario();
    vi.advanceTimersByTime(500); // mid-run
    store.reset();
    vi.advanceTimersByTime(5000); // would have fired the remaining steps
    expect(store.phase).toBe('idle');
    expect(store.causal).toBeNull();
    expect(store.bundle).toBeNull();
    expect(store.steps.every((s) => s.state === 'pending')).toBe(true);
  });

  it('bundles from the live session: provider counts and locales mirror the stores', () => {
    const store = new DiagnosticsStore();
    run(store);
    store.prepareBundle();
    expect(store.phase).toBe('bundled');
    expect(store.bundle?.counts.included).toBeGreaterThan(0);
    expect(store.bundle?.counts.redacted).toBeGreaterThan(0);
    expect(store.bundle?.counts.excluded).toBeGreaterThan(0);
    expect(store.causal?.providerState).toEqual(providers.counts());
    expect(store.aiPrompt).toBe(buildAiPrompt(store.bundle!));
  });

  it('prepareBundle without a scenario still yields a safe default bundle', () => {
    const store = new DiagnosticsStore();
    store.prepareBundle();
    const prompt = buildAiPrompt(store.bundle!);
    expect(prompt).not.toContain('sk-proj');
    expect(prompt).toContain('[REDACTED]');
  });
});

describe('mock fixtures stay synthetic', () => {
  it('version literal is a labelled mock stand-in, not canonical metadata', () => {
    expect(MOCK_APP_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it('locations use generic placeholders, not real owner paths', () => {
    expect(SOURCE_LOCATION).toContain('/Users/<user>');
    expect(SOURCE_LOCATION).toContain('<publishedfileid>');
    expect(OUTPUT_LOCATION).toContain('/Users/<user>');
    expect(normalizeHomePath(`${MOCK_HOME_PREFIX}/somewhere`, MOCK_HOME_PREFIX)).toBe('~/somewhere');
  });
});
