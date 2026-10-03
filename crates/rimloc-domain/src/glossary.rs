//! Project glossary (wave 13): terminology pairs attached to a managed
//! project. A GENERIC core concept (LOCALIZATION_ADAPTERS.md) — the glossary
//! is adapter-independent state of the canonical project, never RimWorld
//! metadata. Persistence rides the ordinary project envelope
//! (`Project.glossary`, serde default → legacy files load empty).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One glossary term. `term` is unique per project CASE-INSENSITIVE — a
/// re-upsert of an existing term (any casing) updates that entry in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GlossaryTerm {
    /// Stable opaque id minted once at creation; survives upserts.
    pub id: String,
    /// The source-language term (canonical spelling as entered).
    pub term: String,
    /// The target-language translation.
    pub translation: String,
    /// Optional usage note (context, style,禁忌 — translator's aid).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Hard field limits (chars) — contract-level form, enforced BEFORE any
/// persistence so a refused write never touches the revision.
pub const TERM_MAX: usize = 200;
pub const TRANSLATION_MAX: usize = 2000;
pub const NOTE_MAX: usize = 500;

/// Validate + normalize one term for upsert. Returns the cleaned fields.
/// Refusals are exact, never averaged: empty required fields, over-limit
/// lengths, control characters (the project canon forbids them — they ride
/// invisibly into exports and break XML/PO writers downstream).
pub fn validate_input(
    term: &str,
    translation: &str,
    note: Option<&str>,
) -> Result<(String, String, Option<String>), String> {
    let term = term.trim();
    let translation = translation.trim();
    if term.is_empty() {
        return Err("glossary term must not be empty".into());
    }
    if translation.is_empty() {
        return Err("glossary translation must not be empty".into());
    }
    if term.chars().count() > TERM_MAX {
        return Err(format!("glossary term exceeds {TERM_MAX} chars"));
    }
    if translation.chars().count() > TRANSLATION_MAX {
        return Err(format!(
            "glossary translation exceeds {TRANSLATION_MAX} chars"
        ));
    }
    let note = match note {
        Some(n) => {
            let n = n.trim();
            if n.is_empty() {
                None
            } else {
                if n.chars().count() > NOTE_MAX {
                    return Err(format!("glossary note exceeds {NOTE_MAX} chars"));
                }
                Some(n.to_string())
            }
        }
        None => None,
    };
    for (field, value) in [("term", term), ("translation", translation)] {
        if let Some(pos) = value.chars().position(is_control_char) {
            return Err(format!(
                "glossary {field} contains a control character at offset {pos}"
            ));
        }
    }
    if let Some(n) = &note {
        if let Some(pos) = n.chars().position(is_control_char) {
            return Err(format!(
                "glossary note contains a control character at offset {pos}"
            ));
        }
    }
    Ok((term.to_string(), translation.to_string(), note))
}

/// The project canon forbids control characters: C0 (incl. tab/newline —
/// a note is single-line) and DEL.
fn is_control_char(c: char) -> bool {
    c < ' ' || c == '\u{7f}'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_clean_input_and_trims() {
        let (t, tr, note) =
            validate_input("  scorched earth  ", " выжженная земля ", Some("  ")).unwrap();
        assert_eq!(t, "scorched earth");
        assert_eq!(tr, "выжженная земля");
        assert_eq!(note, None, "whitespace-only note normalizes to None");
    }

    #[test]
    fn validate_refuses_empty_overlimit_and_control() {
        assert!(validate_input("  ", "x", None).is_err());
        assert!(validate_input("x", " ", None).is_err());
        let long = "x".repeat(TERM_MAX + 1);
        assert!(validate_input(&long, "y", None).is_err());
        let long_tr = "y".repeat(TRANSLATION_MAX + 1);
        assert!(validate_input("x", &long_tr, None).is_err());
        assert!(validate_input("a\nb", "y", None).is_err());
        assert!(validate_input("a\tb", "y", None).is_err());
        assert!(validate_input("x", "y", Some("n\u{7f}")).is_err());
    }

    #[test]
    fn limits_at_boundary_are_accepted() {
        let t = "x".repeat(TERM_MAX);
        let tr = "y".repeat(TRANSLATION_MAX);
        let note = "n".repeat(NOTE_MAX);
        assert!(validate_input(&t, &tr, Some(&note)).is_ok());
    }
}
