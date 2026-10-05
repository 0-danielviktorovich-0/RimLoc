// Backend-message localization by stable machine ids (self-localization
// audit §4/§7): `contract.error.<code>` and `finding.<kind>` keys must cover
// EVERY wire value the GUI can receive, and the display helpers must fall
// back to the raw server message when a key does not resolve — an unknown
// future id never renders as its own key, and never as silence.
//
// Mock findings (lib/client/mock.ts categories) are deliberately NOT
// covered here: they are simulated backend data, not UI copy (audit §4).
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { contractErrorText, findingText } from '../src/lib/client/messages';
import type { ContractErrorCode } from '../src/lib/client/types';

/** Exhaustive by construction: adding a ContractErrorCode variant without
 *  adding it here is a TYPE error (Record over the full union). */
const CODES: Record<ContractErrorCode, true> = {
  stale_epoch: true,
  stale_revision: true,
  save_failed: true,
  project_changed_on_disk: true,
  contract_violation: true,
  guard_output_denied: true,
  invalid_output_path: true,
  unsupported_capability: true,
  project_not_found: true,
  schema_version: true,
  validation_failed: true,
  internal: true
};

/** Every finding kind `project_validate` can emit: the five rimloc-validate
 *  kinds (lib.rs emission sites) plus the session-level ones (session.rs). */
const FINDING_KINDS = [
  'empty',
  'invisible-char',
  'placeholder-check',
  'duplicate',
  'duplicate-global',
  'lost-placeholder',
  'case-collision',
  'source-drift'
] as const;

describe('backend-message localization: full key coverage', () => {
  it('every ContractErrorCode has contract.error.<code> in en and ru', () => {
    for (const code of Object.keys(CODES)) {
      expect(en[`contract.error.${code}`], `en contract.error.${code}`).toBeTruthy();
      expect(ru[`contract.error.${code}`], `ru contract.error.${code}`).toBeTruthy();
    }
  });

  it('every finding kind has finding.<kind> in en and ru', () => {
    for (const kind of FINDING_KINDS) {
      expect(en[`finding.${kind}`], `en finding.${kind}`).toBeTruthy();
      expect(ru[`finding.${kind}`], `ru finding.${kind}`).toBeTruthy();
    }
  });
});

describe('backend-message localization: display helpers', () => {
  it('contractErrorText prefixes the stable code and embeds the raw message', () => {
    const text = contractErrorText('guard_output_denied', 'target sits in the source tree');
    expect(text.startsWith('guard_output_denied:')).toBe(true);
    expect(text).toContain('target sits in the source tree');
  });

  it('contractErrorText falls back to the raw message for an unknown code', () => {
    const text = contractErrorText('some_future_code' as ContractErrorCode, 'raw server text');
    expect(text).toBe('some_future_code: raw server text');
  });

  it('findingText localizes a known kind and keeps the raw detail', () => {
    const text = findingText('lost-placeholder', 'source has placeholder token(s) ["count"]');
    expect(text).not.toBe('finding.lost-placeholder');
    expect(text).toContain('source has placeholder token(s) ["count"]');
  });

  it('findingText falls back to the raw message for an unknown kind', () => {
    expect(findingText('mock-only-kind', 'suspicious placeholder')).toBe('suspicious placeholder');
  });
});
