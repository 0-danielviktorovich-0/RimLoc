//! Shared key-matching resolver for coverage/compare/cross-language validators.
//!
//! ONE implementation serves every consumer (§P1-6): coverage counts,
//! uncovered samples, placeholder pairing and glossary mapping must never
//! re-implement matching independently.
//!
//! Matching rules (§P1-5, adjudication §3/§4):
//! 1. EXACT key match always wins.
//! 2. Otherwise a PROVEN alias matches: the target key is a known alternative
//!    serialization path of a known TKey identity (typed registry entry, e.g.
//!    official packs addressing an Odyssey TKey node via structural
//!    `root.nodes.*` paths). Aliases are data, not code — they never come
//!    from shape heuristics.
//! 3. Otherwise, a target key whose `.slateRef` / `.value.slateRef` suffix is
//!    stripped matches a source entry ONLY when that base identity is a known
//!    TKey identity. Shape-based stripping alone is forbidden — a genuine
//!    `X.slateRef` DefInjected path must never alias an unrelated `X` entry.
//! 4. When several target variants resolve to one source entry, resolution is
//!    deterministic (§P1-7): candidates are sorted by key; the first with a
//!    non-empty, non-TODO value wins; otherwise the alphabetically first.
//!    Multi-variant presence is reported as a collision count, never silently
//!    collapsed.

use rimloc_core::TransUnit;
use std::collections::{BTreeMap, BTreeSet};

/// Typed TKey metadata (general form; per the meta-freeze correction this is
/// NOT a growing string-special-case field):
/// - `identities` — logical TKey identities (`<defName>.<TKey>`) proven to
///   exist in the source corpus (`scan_defs_tkey`);
/// - `aliases` — proven ALTERNATIVE serialization paths of a logical identity
///   (structural addressing, e.g. official RU addressing an Odyssey TKey node
///   via `SurveySite.root.nodes.…Letter.label.slateRef`). Map is
///   `proven target path -> logical identity`.
///
/// Suffix handling stays context-derived at scan time; this registry never
/// infers aliases from key shape.
#[derive(Default, Debug, Clone)]
pub struct TKeyRegistry {
    pub identities: BTreeSet<String>,
    pub aliases: BTreeMap<String, String>,
}

impl TKeyRegistry {
    pub fn from_identities<I, S>(identities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            identities: identities.into_iter().map(Into::into).collect(),
            aliases: BTreeMap::new(),
        }
    }

    /// Resolve a target key to the logical source identity it addresses.
    /// Order: exact identity -> proven alias -> known-suffix fallback.
    /// `None` = the target does not address any known TKey identity.
    pub fn identity_for(&self, target_key: &str) -> Option<String> {
        if self.identities.contains(target_key) {
            return Some(target_key.to_string());
        }
        if let Some(base) = self.aliases.get(target_key) {
            return Some(base.clone());
        }
        let base = strip_tkey_suffix(target_key)?;
        self.identities.contains(&base).then_some(base)
    }

    /// The logical identity a source key represents (identity or alias base).
    /// Used when building the source-side index so an aliased source entry
    /// lands under its logical identity, not its structural path.
    pub fn identity_of_source(&self, source_key: &str) -> String {
        self.aliases
            .get(source_key)
            .cloned()
            .unwrap_or_else(|| source_key.to_string())
    }

    pub fn len(&self) -> usize {
        self.identities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.identities.is_empty()
    }
}

pub fn is_todo(value: &str) -> bool {
    let t = value.trim();
    t.is_empty() || t.eq_ignore_ascii_case("TODO")
}

/// How a target key reached its source entry (Gate C provenance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOrigin {
    /// Target key equals the source key verbatim.
    Exact,
    /// Target key is a registered proven alias of the identity.
    ProvenAlias,
    /// Target key = identity + a known context-derived TKey suffix.
    SuffixFallback,
}

/// Shared resolution outcome for cross-language consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Matched {
        origin: MatchOrigin,
        source_key: String,
    },
    /// The target key plausibly belongs to more than one source identity;
    /// callers must emit a diagnostic, never an arbitrary winner.
    Ambiguous {
        target_key: String,
        candidates: Vec<String>,
    },
    /// No source counterpart (orphan candidate).
    Unmatched { target_key: String },
}

/// Strip a TKey canonical suffix if present.
pub fn strip_tkey_suffix(key: &str) -> Option<String> {
    for suffix in [".value.slateRef", ".slateRef"] {
        if let Some(base) = key.strip_suffix(suffix) {
            return Some(base.to_string());
        }
    }
    None
}

/// Deterministic resolution of a source key against target values.
///
/// `candidates` are all target values whose exact or TKey-normalized key equals
/// the source key. The winner (if any) is chosen deterministically.
pub fn resolve_value<'a>(candidates: &[&'a str]) -> Option<&'a str> {
    let mut sorted: Vec<&&str> = candidates.iter().collect();
    sorted.sort();
    // Prefer a real translation; fall back to the deterministic first entry
    // (which the caller then treats as TODO/missing-equivalent).
    let winner = sorted.iter().copied().find(|v| !is_todo(v));
    match winner {
        Some(v) => Some(v),
        None => sorted.first().map(|v| **v),
    }
}

/// Index built from the source inventory + the TKey registry.
pub struct SourceMatcher<'a> {
    source: BTreeMap<String, &'a str>,
    tkey: &'a TKeyRegistry,
}

impl<'a> SourceMatcher<'a> {
    pub fn new(source: &'a [TransUnit], tkey: &'a TKeyRegistry) -> Self {
        let mut map = BTreeMap::new();
        for u in source {
            if let Some(text) = u.source.as_deref().filter(|t| !t.trim().is_empty()) {
                // A source entry recorded under a proven alias belongs to the
                // logical identity; exact identity wins if both exist.
                let logical = tkey.identity_of_source(&u.key);
                map.entry(logical).or_insert(text);
                map.entry(u.key.clone()).or_insert(text);
            }
        }
        Self { source: map, tkey }
    }

    pub fn source_keys(&self) -> impl Iterator<Item = &str> {
        self.source.keys().map(String::as_str)
    }

    pub fn contains_source_key(&self, key: &str) -> bool {
        self.source.contains_key(key)
    }

    /// Resolve a target key to its source entry
    /// (exact first, proven alias, then TKey-suffix fallback).
    /// `None` = no source counterpart (orphan candidate).
    pub fn source_for_target(&self, target_key: &str) -> Option<String> {
        match self.resolve_target(target_key) {
            Resolution::Matched { source_key, .. } => Some(source_key),
            _ => None,
        }
    }

    /// THE shared resolved-match result (Gate C): coverage, compare,
    /// placeholder/list/orphan/sourceChanged diagnostics all map THIS to their
    /// own reports; none re-derives alias resolution.
    pub fn resolve_target(&self, target_key: &str) -> Resolution {
        if self.source.contains_key(target_key) {
            // Exact presence is authoritative UNLESS the same key is also a
            // proven alias pointing at a DIFFERENT identity — then the target
            // could legitimately belong to either and the caller must surface
            // ambiguity instead of picking an arbitrary winner.
            if let Some(base) = self.tkey.aliases.get(target_key) {
                if base != target_key {
                    return Resolution::Ambiguous {
                        target_key: target_key.to_string(),
                        candidates: vec![target_key.to_string(), base.clone()],
                    };
                }
            }
            return Resolution::Matched {
                origin: MatchOrigin::Exact,
                source_key: target_key.to_string(),
            };
        }
        match self.tkey.identity_for(target_key) {
            Some(logical) if self.source.contains_key(&logical) => {
                let origin = if self.tkey.aliases.contains_key(target_key) {
                    MatchOrigin::ProvenAlias
                } else {
                    MatchOrigin::SuffixFallback
                };
                Resolution::Matched {
                    origin,
                    source_key: logical,
                }
            }
            _ => Resolution::Unmatched {
                target_key: target_key.to_string(),
            },
        }
    }

    /// Resolve a source key to target values (0, 1 or many candidates).
    /// Multiple candidates = TKey suffix/alias variants; caller applies
    /// [`resolve_value`] for a deterministic winner and may count collisions.
    pub fn targets_for_source<'b>(
        &self,
        targets: &'b BTreeMap<String, String>,
        source_key: &str,
    ) -> Vec<&'b str> {
        let mut out = Vec::new();
        if let Some(v) = targets.get(source_key) {
            out.push(v.as_str());
        }
        let logical = self.tkey.identity_of_source(source_key);
        for (k, v) in targets {
            if k == source_key {
                continue;
            }
            let addresses = self
                .tkey
                .identity_for(k)
                .as_deref()
                .is_some_and(|id| id == logical);
            if addresses {
                out.push(v.as_str());
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Deterministic winner for a resolved candidate list.
    pub fn pick<'b>(&self, candidates: &[&'b str]) -> Option<&'b str> {
        resolve_value(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(ids: &[&str]) -> TKeyRegistry {
        TKeyRegistry::from_identities(ids.iter().copied())
    }

    #[test]
    fn exact_wins_over_tkey_fallback() {
        let tkey = registry(&["Mod.A"]);
        let units = [u("Mod.A", "base"), u("Mod.A.slateRef", "slate path")];
        let m = SourceMatcher::new(&units, &tkey);
        // Exact source entry exists for Mod.A.slateRef itself.
        assert_eq!(
            m.source_for_target("Mod.A.slateRef").as_deref(),
            Some("Mod.A.slateRef")
        );
    }

    #[test]
    fn tkey_fallback_requires_known_identity() {
        let tkey = registry(&["Mod.A"]);
        let units = [u("Mod.A", "base")];
        let m = SourceMatcher::new(&units, &tkey);
        assert_eq!(
            m.source_for_target("Mod.A.slateRef").as_deref(),
            Some("Mod.A")
        );
        // Non-TKey .slateRef path must NOT alias unrelated Mod.X.
        assert_eq!(m.source_for_target("Other.slateRef"), None);
    }

    #[test]
    fn proven_alias_matches_without_suffix_rules() {
        // Structural addressing (Odyssey style): the target path carries no
        // TKey-shaped suffix at all — only the typed alias maps it.
        let mut tkey = registry(&["SurveySite.LetterLabelDone"]);
        tkey.aliases.insert(
            "SurveySite.root.nodes.AllSignals.nodes.Letter.label.slateRef".into(),
            "SurveySite.LetterLabelDone".into(),
        );
        let units = [u("SurveySite.LetterLabelDone", "Quest complete")];
        let m = SourceMatcher::new(&units, &tkey);
        assert_eq!(
            m.source_for_target("SurveySite.root.nodes.AllSignals.nodes.Letter.label.slateRef")
                .as_deref(),
            Some("SurveySite.LetterLabelDone")
        );
        // An unregistered structural path stays unmatched (no shape guessing).
        assert_eq!(
            m.source_for_target("SurveySite.root.nodes.Other.label.slateRef"),
            None
        );
    }

    #[test]
    fn alias_source_entry_lands_under_logical_identity() {
        let mut tkey = registry(&["Def.Key"]);
        tkey.aliases.insert(
            "Def.root.nodes.Letter.text.slateRef".into(),
            "Def.Key".into(),
        );
        // Source inventory stores the alias path (e.g. re-imported pack).
        let units = [u("Def.root.nodes.Letter.text.slateRef", "text")];
        let m = SourceMatcher::new(&units, &tkey);
        assert_eq!(
            m.source_for_target("Def.Key.value.slateRef").as_deref(),
            Some("Def.Key")
        );
    }

    #[test]
    fn resolve_target_reports_origin_precedence() {
        let mut tkey = registry(&["A.B"]);
        tkey.aliases.insert("A.struct.path".into(), "A.B".into());
        let units = [u("A.B", "v"), u("A.B.value.slateRef", "w")];
        let m = SourceMatcher::new(&units, &tkey);
        assert_eq!(
            m.resolve_target("A.B"),
            Resolution::Matched {
                origin: MatchOrigin::Exact,
                source_key: "A.B".into()
            }
        );
        assert_eq!(
            m.resolve_target("A.struct.path"),
            Resolution::Matched {
                origin: MatchOrigin::ProvenAlias,
                source_key: "A.B".into()
            }
        );
        assert_eq!(
            m.resolve_target("A.B.slateRef"),
            Resolution::Matched {
                origin: MatchOrigin::SuffixFallback,
                source_key: "A.B".into()
            }
        );
        assert_eq!(
            m.resolve_target("Z.Nope.slateRef"),
            Resolution::Unmatched {
                target_key: "Z.Nope.slateRef".into()
            }
        );
    }

    #[test]
    fn resolve_target_flags_exact_alias_collision_as_ambiguous() {
        let mut tkey = registry(&["A.B"]);
        tkey.aliases.insert("A.C".into(), "A.B".into());
        let units = [u("A.B", "v"), u("A.C", "other entry")];
        let m = SourceMatcher::new(&units, &tkey);
        // A.C exists as a real source entry AND aliases A.B: a target "A.C"
        // could belong to either — diagnostic, not arbitrary winner.
        assert_eq!(
            m.resolve_target("A.C"),
            Resolution::Ambiguous {
                target_key: "A.C".into(),
                candidates: vec!["A.C".into(), "A.B".into()]
            }
        );
        // Without the colliding exact entry the same alias resolves cleanly.
        let units2 = [u("A.B", "v")];
        let m2 = SourceMatcher::new(&units2, &tkey);
        assert_eq!(
            m2.resolve_target("A.C"),
            Resolution::Matched {
                origin: MatchOrigin::ProvenAlias,
                source_key: "A.B".into()
            }
        );
    }

    #[test]
    fn suffix_fallback_requires_known_identity_even_with_alias_support() {
        let mut tkey = registry(&["A.B"]);
        tkey.aliases.insert("A.alias.path".into(), "A.B".into());
        let units = [u("A.B", "v")];
        let m = SourceMatcher::new(&units, &tkey);
        // Registered alias works...
        assert_eq!(m.source_for_target("A.alias.path").as_deref(), Some("A.B"));
        // ...unknown sibling paths still do not (suffix + shape are not proof).
        assert_eq!(m.source_for_target("A.alias.other"), None);
    }

    fn u(key: &str, text: &str) -> TransUnit {
        TransUnit {
            key: key.into(),
            source: Some(text.into()),
            path: "x".into(),
            line: None,
            tkey: None,
            ..Default::default()
        }
    }

    use rimloc_core::TransUnit;
}
