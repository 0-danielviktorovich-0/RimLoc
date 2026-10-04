// Localized contract-error text (mirrors the frozen Svelte messages.ts
// semantics): the snake_case code prefixes the localized sentence; unknown
// codes fall back to the raw backend message. Framework-neutral.
import { t } from '../i18n'

const KNOWN = new Set([
  'stale_epoch',
  'stale_revision',
  'save_failed',
  'project_changed_on_disk',
  'contract_violation',
  'guard_output_denied',
  'invalid_output_path',
  'unsupported_capability',
  'project_not_found',
  'schema_version',
  'validation_failed',
  'internal',
])

export function contractErrorText(code: string, message: string): string {
  if (KNOWN.has(code)) {
    // Reuse the frozen RU/EN catalog sentences when the lane joins the
    // shared pipeline; until then the code + raw message stays honest.
    return `${code}: ${t('contract.error.detail', { message })}`
  }
  return `${code}: ${message}`
}
