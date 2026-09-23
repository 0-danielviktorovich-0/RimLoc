//! Shared key-matching resolver for coverage/compare/cross-language validators.
//!
//! ONE implementation serves every consumer (§P1-6): coverage counts,
//! uncovered samples, placeholder pairing and glossary mapping must never
//! re-implement matching independently.
//!
//! Matching rules (§P1-5):
//! 1. EXACT key match always wins.
//! 2. Otherwise, a target key whose `.slateRef` / `.value.slateRef` suffix is
//!    stripped matches a source entry ONLY when that base identity is a known
//!    TKey identity (from `scan_defs_tkey`). Shape-based stripping alone is
//!    forbidden — a genuine `X.slateRef` DefInjected path must never alias an
//!    unrelated `X` source entry.
//! 3. When several target variants resolve to one source entry, resolution is
//!    deterministic (§P1-7): candidates are sorted by key; the first with a
//!    non-empty, non-TODO value wins; otherwise the alphabetically first.
//!    Multi-variant presence is reported as a collision count, never silently
//!    collapsed.

use rimloc_core::TransUnit;
use std::collections::{BTreeMap, BTreeSet};

/// Base TKey identities (`<defName>.<TKey>`) known from extraction.
pub type TKeyIndex = BTreeSet<String>;

pub fn is_todo(value: &str) -> bool {
    let t = value.trim();
    t.is_empty() || t.eq_ignore_ascii_case("TODO")
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

/// Index built from the source inventory + known TKey identities.
pub struct SourceMatcher<'a> {
    source: BTreeMap<String, &'a str>,
    tkey: &'a TKeyIndex,
}

impl<'a> SourceMatcher<'a> {
    pub fn new(source: &'a [TransUnit], tkey: &'a TKeyIndex) -> Self {
        let mut map = BTreeMap::new();
        for u in source {
            if let Some(text) = u.source.as_deref().filter(|t| !t.trim().is_empty()) {
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

    /// Resolve a target key to its source entry (exact first, TKey fallback).
    /// `None` = no source counterpart (orphan candidate).
    pub fn source_for_target(&self, target_key: &str) -> Option<String> {
        if self.source.contains_key(target_key) {
            return Some(target_key.to_string());
        }
        let base = strip_tkey_suffix(target_key)?;
        if self.tkey.contains(&base) && self.source.contains_key(&base) {
            return Some(base);
        }
        None
    }

    /// Resolve a source key to target values (0, 1 or many candidates).
    /// Multiple candidates = TKey suffix variants; caller applies
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
        let base = format!("{source_key}.");
        for (k, v) in targets {
            if k.starts_with(&base) && strip_tkey_suffix(k).as_deref() == Some(source_key) {
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

    fn idx(items: &[&str]) -> TKeyIndex {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exact_wins_over_tkey_fallback() {
        let tkey = idx(&["Mod.A"]);
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
        let tkey = idx(&["Mod.A"]);
        let units = [u("Mod.A", "base")];
        let m = SourceMatcher::new(&units, &tkey);
        assert_eq!(
            m.source_for_target("Mod.A.slateRef").as_deref(),
            Some("Mod.A")
        );
        // Non-TKey .slateRef path must NOT alias unrelated Mod.X.
        assert_eq!(m.source_for_target("Other.slateRef"), None);
    }

    fn u(key: &str, text: &str) -> TransUnit {
        TransUnit {
            key: key.into(),
            source: Some(text.into()),
            path: "x".into(),
            line: None,
        }
    }

    use rimloc_core::TransUnit;
}
