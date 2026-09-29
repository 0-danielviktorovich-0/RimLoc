// Backend-message localization by stable machine ids (self-localization
// audit §4/§7): ContractError.code and ValidationFinding.kind are stable
// wire values (append-only enums in rimloc-services), so the UI maps them
// onto i18n keys — `contract.error.<code>` and `finding.<kind>` — and falls
// back to the raw EN server message when a key does not resolve. An unknown
// future id must never render as its own key or as silence.
//
// The free-form backend message carries the CONCRETE detail (which revision,
// which placeholder names, which control chars); the localized sentence
// carries the MEANING. The two compose through the {message} param: keys
// that describe a detail-bearing category embed `{message}` (always passed),
// keys for self-explanatory categories omit it — t() then ignores the param.
import { i18n, t } from '../../i18n/store.svelte';
import type { ContractErrorCode } from './types';

/** Localized text for a typed contract error: `contract.error.<code>`.
 *  The snake_case code prefixes the result in BOTH branches — tests and
 *  support diagnostics grep for it, and the code stays visible next to the
 *  human sentence instead of being hidden behind it. */
export function contractErrorText(code: ContractErrorCode, message: string): string {
  const key = `contract.error.${code}`;
  if (i18n.has(key)) {
    return `${code}: ${t(key, { message })}`;
  }
  return `${code}: ${message}`;
}

/** Localized text for a validation finding: `finding.<kind>`. Falls back
 *  to the raw backend message when the kind has no key (mock data and
 *  kinds this build does not know stay backend EN data, audit §4). */
export function findingText(kind: string, message: string): string {
  const key = `finding.${kind}`;
  if (i18n.has(key)) {
    return t(key, { message });
  }
  return message;
}
