// Full structural identity serialization (review P0-2 fix). ONE module so
// the workspace key-space has exactly one definition — the previous state
// (a private copy per screen + the bare display key everywhere else) let
// two Def types sharing one key collapse into a single row/draft/finding.
import type { SourceEntryIdDto } from '../client/types'

/** The exact serialization of the FULL structural identity, mirroring the
 *  backend `SourceEntryId::display_identity()` (Rust domain canonical.rs):
 *  `<kind>·[<def_type>·]<key>` with the same `·` separator and the same
 *  snake_case kind tokens the wire carries. Chat-batch entry_keys already
 *  speak this exact string; selection, drafts and validator findings use
 *  it too — never the bare display key, which two Def types can share. */
export function entryUid(id: SourceEntryIdDto): string {
  return id.def_type ? `${id.kind}·${id.def_type}·${id.key}` : `${id.kind}·${id.key}`
}

/** Durable-store key of ONE draft: the TARGET LOCALE is part of the key
 *  (`<locale>::<uid>`), so parallel target locales keep parallel unsaved
 *  slices and a locale switch can never erase another slice (review P0-1).
 *  Locale ids here are the registry form ('ru') — the same form the store
 *  keeps in `targetLocale`; the folder-form contract concerns the
 *  confirmed-translation write paths, not this key convention. */
export function draftsStoreKey(locale: string, uid: string): string {
  return `${locale}::${uid}`
}

/** v2 store keys carry the `<locale>::` prefix. Legacy v1 stores hold bare
 *  display keys; those are re-anchored on restore (never destroyed), and
 *  unresolvable ones survive verbatim until the inventory can place them. */
export function isLocaleScopedStoreKey(key: string): boolean {
  return /^[A-Za-z][A-Za-z0-9_-]*::/.test(key)
}
