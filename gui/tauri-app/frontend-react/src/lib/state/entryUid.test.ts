import { describe, expect, it } from 'vitest'
import { draftsStoreKey, entryUid, isLocaleScopedStoreKey } from './entryUid'

// The serialization MUST mirror the backend `SourceEntryId::display_identity()`
// exactly (crates/rimloc-domain/src/canonical.rs): `kind·[def_type·]key`,
// snake_case kind tokens, `·` separator — chat-batch entry_keys already speak
// this string on the wire.
describe('entryUid — display_identity() mirror', () => {
  it('renders kind · def_type · key when a def type is present', () => {
    expect(
      entryUid({ kind: 'def_injected', def_type: 'ThingDef', key: 'Shared.label' }),
    ).toBe('def_injected·ThingDef·Shared.label')
  })

  it('renders kind · key when the kind is not def-type-scoped', () => {
    expect(entryUid({ kind: 'keyed', key: 'Settings' })).toBe('keyed·Settings')
  })

  it('renders kind · key when the def type is genuinely unknown', () => {
    expect(entryUid({ kind: 'def_injected', key: 'Orphan.label' })).toBe(
      'def_injected·Orphan.label',
    )
  })
})

describe('drafts store keys — locale scoping', () => {
  it('prefixes the registry-form locale', () => {
    expect(draftsStoreKey('ru', 'keyed·Settings')).toBe('ru::keyed·Settings')
  })

  it('recognizes v2 keys and rejects legacy display keys', () => {
    expect(isLocaleScopedStoreKey('ru::keyed·Settings')).toBe(true)
    expect(isLocaleScopedStoreKey('uk::def_injected·HediffDef·Shared.label')).toBe(true)
    expect(isLocaleScopedStoreKey('Settings')).toBe(false)
    expect(isLocaleScopedStoreKey('Shared.label')).toBe(false)
  })
})
