// Mutability provenance for glossary/TM records (W4.5 integration
// requirement #2, mandate §10/§11). Every record carries where it came from;
// `reference-read-only` marks reference corpora that the live binding must
// never destructively mutate — the editors disable edit/delete for those and
// explain why in a tooltip. Semantics live here; the editors only render.

export type Mutability = 'user-created' | 'project-created' | 'imported' | 'reference-read-only';

export const MUTABILITY_VALUES: Mutability[] = [
  'user-created',
  'project-created',
  'imported',
  'reference-read-only'
];

/** i18n key suffix under `mutability.<value>`. */
export function mutabilityKey(m: Mutability): string {
  return `mutability.${m}`;
}

/** Reference corpora are immutable: no edit, no delete, no rename. */
export function isReadOnly(m: Mutability): boolean {
  return m === 'reference-read-only';
}
