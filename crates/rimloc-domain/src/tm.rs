//! Translation memory (TM live, owner decision A+B+C): reusable
//! source→target pairs attached to a managed project. A GENERIC core
//! concept (LOCALIZATION_ADAPTERS.md) — the TM is adapter-independent state
//! of the canonical project, never RimWorld metadata. Persistence rides the
//! ordinary project envelope (`Project.tm`, serde default → legacy files
//! load empty), exactly like the glossary (wave 13).
//!
//! Semantics (owner-approved, do not change):
//! - **A = auto-accumulation**: when a translation is ACCEPTED (the
//!   `project_apply_intents` ack — never a draft/save), an entry with
//!   provenance=AUTO, status=ACCEPTED lands in the TM.
//! - **B = import**: imported records get provenance=IMPORT and
//!   status=DRAFT by default — an import is never trusted blindly.
//! - **C = manual CRUD**: `tm_upsert`/`tm_delete` with provenance=MANUAL;
//!   the user picks the status (default ACCEPTED).
//! - Key of a record: (source_text, target_locale) — target locales are
//!   ISOLATED: a lookup always runs within one locale.
//! - When the source text changes, the old record stays (it ages); the new
//!   source is a lookup MISS until translated again.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One TM record. Unique per project by the (source_text, target_locale)
/// key — the write policies (auto/import/manual) keep the invariant, an
/// upsert never duplicates a key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TranslationMemoryEntry {
    /// Stable opaque id minted once at creation; survives upserts.
    pub id: String,
    /// Source-language text (trimmed, exact case — TM matching is
    /// case-sensitive: `Save` and `save` are different keys).
    pub source_text: String,
    /// Target-language translation.
    pub target_text: String,
    /// Target locale (folder contract, e.g. `Russian`) — the ISOLATION
    /// key: every lookup is scoped to one locale.
    pub target_locale: String,
    /// Trust status (see [`TmStatus`]).
    pub status: TmStatus,
    /// Where the record came from (see [`TmProvenance`]).
    pub provenance: TmProvenance,
}

/// Trust status of a TM record. Priority REVIEWED > ACCEPTED > DRAFT:
/// a stronger record is never weakened by a weaker write path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TmStatus {
    Draft,
    Accepted,
    Reviewed,
}

impl TmStatus {
    /// Numeric trust rank — HIGHER is MORE trusted. Lookup ordering and
    /// every write policy compare through this, never through enum order
    /// by accident.
    pub fn rank(self) -> u8 {
        match self {
            TmStatus::Draft => 0,
            TmStatus::Accepted => 1,
            TmStatus::Reviewed => 2,
        }
    }

    /// Parse the wire/CSV wording (case-insensitive). Import metadata is
    /// explicit user intent, so an explicit status IS honored — the
    /// "DRAFT by default" rule covers its ABSENCE only.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "draft" => Some(TmStatus::Draft),
            "accepted" => Some(TmStatus::Accepted),
            "reviewed" => Some(TmStatus::Reviewed),
            _ => None,
        }
    }
}

/// Where a TM record came from. Pure provenance (honest bookkeeping) — it
/// never upgrades trust by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TmProvenance {
    /// Accepted-translation auto-accumulation (A).
    Auto,
    /// Bulk import (B) — DRAFT by default, never trusted blindly.
    Import,
    /// Manual CRUD (C).
    Manual,
}

/// Hard field limits (chars) — contract-level form, enforced BEFORE any
/// persistence so a refused write never touches the revision.
pub const TM_SOURCE_MAX: usize = 4000;
pub const TM_TARGET_MAX: usize = 4000;
pub const TM_LOCALE_MAX: usize = 16;

/// Validate + normalize one record for write (auto/import/manual share the
/// same gate). Returns the cleaned fields. Refusals are exact, never
/// averaged: empty required fields, over-limit lengths, control characters
/// (they ride invisibly into exports and break XML/PO writers downstream).
pub fn validate_input(
    source_text: &str,
    target_text: &str,
    target_locale: &str,
) -> Result<(String, String, String), String> {
    let source_text = source_text.trim();
    let target_text = target_text.trim();
    let target_locale = target_locale.trim();
    if source_text.is_empty() {
        return Err("tm source text must not be empty".into());
    }
    if target_text.is_empty() {
        return Err("tm target text must not be empty".into());
    }
    if target_locale.is_empty() {
        return Err("tm target locale must not be empty".into());
    }
    if source_text.chars().count() > TM_SOURCE_MAX {
        return Err(format!("tm source text exceeds {TM_SOURCE_MAX} chars"));
    }
    if target_text.chars().count() > TM_TARGET_MAX {
        return Err(format!("tm target text exceeds {TM_TARGET_MAX} chars"));
    }
    if target_locale.chars().count() > TM_LOCALE_MAX {
        return Err(format!("tm target locale exceeds {TM_LOCALE_MAX} chars"));
    }
    for (field, value) in [
        ("source text", source_text),
        ("target text", target_text),
        ("target locale", target_locale),
    ] {
        if let Some(pos) = value.chars().position(is_control_char) {
            return Err(format!(
                "tm {field} contains a control character at offset {pos}"
            ));
        }
    }
    Ok((
        source_text.to_string(),
        target_text.to_string(),
        target_locale.to_string(),
    ))
}

/// The project canon forbids control characters: C0 (incl. tab/newline) and
/// DEL.
fn is_control_char(c: char) -> bool {
    c < ' ' || c == '\u{7f}'
}

/// Normalization for the non-exact lookup tiers: trim, collapse inner
/// whitespace runs to one space, lowercase. Pure (no allocation tricks
/// needed) and deterministic — the same function runs on the query and on
/// the stored sources, so both sides always agree.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_ws = false;
    for c in text.trim().chars() {
        if c.is_whitespace() {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            prev_ws = false;
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        }
    }
    out
}

/// Bounded Levenshtein distance (own DP, two rows, NO new dependencies).
/// Returns `Some(distance)` when the distance is within `max` (inclusive),
/// `None` when it is known to exceed the bound — the early cutoff keeps
/// fuzzy lookup linear in practice. Character-based (not bytes), so
/// Cyrillic counts honestly.
pub fn bounded_levenshtein(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    if a == b {
        return Some(0);
    }
    // Two-row DP; `prev` is the row for a[..i-1], `cur` for a[..i].
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur: Vec<usize> = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        let mut row_min = cur[0];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let v = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
            cur[j + 1] = v;
            row_min = row_min.min(v);
        }
        // Early cutoff: the whole row already exceeds the bound.
        if row_min > max {
            return None;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let d = prev[b.len()];
    if d <= max {
        Some(d)
    } else {
        None
    }
}

/// Fuzzy distance bound for one query: `max(2, len/10)` over the
/// NORMALIZED query length (chars). Fixed floor of 2 keeps short UI
/// strings reachable; the proportional term grows with the query.
pub fn fuzzy_threshold(normalized_query: &str) -> usize {
    std::cmp::max(2, normalized_query.chars().count() / 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_clean_input_and_trims() {
        let (s, t, loc) =
            validate_input("  Save game  ", " Сохранить игру ", "  Russian  ").unwrap();
        assert_eq!(s, "Save game");
        assert_eq!(t, "Сохранить игру");
        assert_eq!(loc, "Russian");
    }

    #[test]
    fn validate_refuses_empty_and_control_chars() {
        assert!(validate_input("  ", "x", "Russian").is_err());
        assert!(validate_input("x", " ", "Russian").is_err());
        assert!(validate_input("x", "y", "").is_err());
        assert!(validate_input("a\nb", "y", "Russian").is_err());
        assert!(validate_input("a\tb", "y", "Russian").is_err());
        assert!(validate_input("x", "y\u{7f}", "Russian").is_err());
        assert!(validate_input("x", "y", "Russ\u{1}ian").is_err());
    }

    #[test]
    fn validate_enforces_limits_exact() {
        let src = "x".repeat(TM_SOURCE_MAX + 1);
        assert!(validate_input(&src, "y", "Russian").is_err());
        let tgt = "y".repeat(TM_TARGET_MAX + 1);
        assert!(validate_input("x", &tgt, "Russian").is_err());
        let loc = "z".repeat(TM_LOCALE_MAX + 1);
        assert!(validate_input("x", "y", &loc).is_err());
        // Boundaries are accepted.
        assert!(validate_input(&"x".repeat(TM_SOURCE_MAX), "y", "Russian").is_ok());
        assert!(validate_input("x", &"y".repeat(TM_TARGET_MAX), "Russian").is_ok());
        assert!(validate_input("x", "y", &"z".repeat(TM_LOCALE_MAX)).is_ok());
    }

    #[test]
    fn status_rank_orders_trust() {
        assert!(TmStatus::Reviewed.rank() > TmStatus::Accepted.rank());
        assert!(TmStatus::Accepted.rank() > TmStatus::Draft.rank());
    }

    #[test]
    fn status_parse_is_case_insensitive_and_strict() {
        assert_eq!(TmStatus::parse(" Reviewed "), Some(TmStatus::Reviewed));
        assert_eq!(TmStatus::parse("draft"), Some(TmStatus::Draft));
        assert_eq!(TmStatus::parse("ACCEPTED"), Some(TmStatus::Accepted));
        assert_eq!(TmStatus::parse("published"), None);
        assert_eq!(TmStatus::parse(""), None);
    }

    #[test]
    fn normalize_collapses_case_and_whitespace() {
        assert_eq!(normalize("  Save   the GAME "), "save the game");
        assert_eq!(normalize("Сохранить  Игру"), "сохранить игру");
        assert_eq!(normalize("\t\n"), "");
        // Non-ASCII case folding works char-wise.
        assert_eq!(normalize("ÉLAN"), "élan");
    }

    #[test]
    fn bounded_levenshtein_is_exact_within_bound() {
        assert_eq!(bounded_levenshtein("kitten", "sitting", 3), Some(3));
        assert_eq!(bounded_levenshtein("kitten", "sitting", 2), None);
        assert_eq!(bounded_levenshtein("abc", "abc", 0), Some(0));
        assert_eq!(bounded_levenshtein("", "", 0), Some(0));
        assert_eq!(bounded_levenshtein("", "abc", 3), Some(3));
        assert_eq!(bounded_levenshtein("abc", "", 2), None);
        // Length-difference shortcut fires before the DP.
        assert_eq!(bounded_levenshtein("a", "abcdef", 2), None);
    }

    #[test]
    fn bounded_levenshtein_counts_chars_not_bytes() {
        // ё vs е is one edit (2 bytes differ, 1 char).
        assert_eq!(bounded_levenshtein("ёлка", "елка", 1), Some(1));
        assert_eq!(bounded_levenshtein("сохранить", "сохранённый", 4), None);
        assert_eq!(bounded_levenshtein("сохранить", "сохранённый", 5), Some(5));
    }

    #[test]
    fn fuzzy_threshold_floor_and_proportion() {
        assert_eq!(fuzzy_threshold(""), 2);
        assert_eq!(fuzzy_threshold("ab"), 2);
        assert_eq!(fuzzy_threshold("save the game"), 2, "13/10 -> 1, floor 2");
        assert_eq!(fuzzy_threshold(&"x".repeat(100)), 10);
    }
}
