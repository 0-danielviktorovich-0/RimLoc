//! Capability-aware WordInfo support (§10).
//!
//! WordInfo (Gender/Case/Plural declension tables) exists only for languages
//! whose LanguageWorker uses it (ru/de/uk per OFFICIAL_LANG_PACKS.md). It is
//! NEVER generated for languages without a worker (ja/zh/en) — capability
//! model, not a Russian-universal assumption.
//!
//! MVP scope: diagnostics (translated labels missing from WordInfo) + review
//! scaffolds (`?` forms for human completion). Actual form generation belongs
//! to `rimloc morph` (Morpher/pymorphy2) and stays a separate explicit step.

use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WordInfoCapability {
    /// Gender/Case/Plural tables + LanguageWorker declension.
    Full,
    /// No WordInfo: the language's worker does not decline nouns.
    None,
}

/// Capability lookup by language code or folder name (case-insensitive).
pub fn capability_for(lang: &str) -> WordInfoCapability {
    let l = lang.to_lowercase();
    let ru = l.starts_with("ru") || l.starts_with("russ");
    let de = l.starts_with("de") || l.starts_with("german") || l.starts_with("deutsch");
    let uk = l.starts_with("uk") || l.starts_with("ukrain");
    if ru || de || uk {
        WordInfoCapability::Full
    } else {
        WordInfoCapability::None
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WordInfoDiag {
    pub capability: WordInfoCapability,
    pub wordinfo_dir: Option<String>,
    /// Translated labels not present in any WordInfo table.
    pub missing_labels: Vec<String>,
    /// Labels already covered (first word found in a table).
    pub covered_labels: usize,
}

/// Collect translated `.label` values for a language folder. Scans the mod
/// root (scanner expects a mod layout) and filters by the language folder.
fn translated_labels(root: &Path, lang_dir: &str) -> Result<Vec<String>> {
    if !root.join("Languages").join(lang_dir).is_dir() {
        return Ok(Vec::new());
    }
    let units = crate::scan_units_auto(root)?;
    let mut out = Vec::new();
    for u in units {
        if u.key.ends_with(".label") && crate::is_under_languages_dir(&u.path, lang_dir) {
            if let Some(v) = u.source {
                let v = v.trim().to_string();
                if !v.is_empty() {
                    out.push(v);
                }
            }
        }
    }
    Ok(out)
}

/// First word of a label, lowercased — the WordInfo lookup key.
fn head_word(label: &str) -> String {
    label
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphabetic())
        .to_lowercase()
}

/// Words present in any WordInfo table (first `;` column of non-comment lines).
fn collect_known_words(wordinfo_dir: &Path) -> BTreeSet<String> {
    let mut known = BTreeSet::new();
    if !wordinfo_dir.is_dir() {
        return known;
    }
    for entry in walkdir::WalkDir::new(wordinfo_dir).into_iter().flatten() {
        let p = entry.path();
        if !p.is_file() || p.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(p) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some(first) = line.split(';').next() {
                let w = first.trim().to_lowercase();
                if !w.is_empty() {
                    known.insert(w);
                }
            }
        }
    }
    known
}

/// Diagnose WordInfo coverage of translated labels.
pub fn diagnose(root: &Path, lang_dir: &str) -> Result<WordInfoDiag> {
    let capability = capability_for(lang_dir);
    if capability == WordInfoCapability::None {
        return Ok(WordInfoDiag {
            capability,
            wordinfo_dir: None,
            missing_labels: Vec::new(),
            covered_labels: 0,
        });
    }
    let wordinfo_dir = root.join("Languages").join(lang_dir).join("WordInfo");
    let known = collect_known_words(&wordinfo_dir);
    let labels = translated_labels(root, lang_dir)?;
    let mut missing = Vec::new();
    let mut covered = 0usize;
    for label in labels {
        let head = head_word(&label);
        if head.is_empty() {
            continue;
        }
        if known.contains(&head) {
            covered += 1;
        } else if !missing.contains(&label) {
            missing.push(label);
        }
    }
    Ok(WordInfoDiag {
        capability,
        wordinfo_dir: wordinfo_dir
            .is_dir()
            .then(|| wordinfo_dir.display().to_string()),
        missing_labels: missing,
        covered_labels: covered,
    })
}

/// Emit a Case.txt scaffold for missing labels: `word;?;?;?;?;?` (6 Russian
/// cases) grouped under a `// scaffold` section. Review by a human (or the
/// morph provider) is REQUIRED before the forms are playable.
pub fn scaffold_case(diag: &WordInfoDiag) -> String {
    let mut out = String::from("// scaffold: generated by RimLoc; `?` forms need review\n");
    let mut seen = BTreeSet::new();
    for label in &diag.missing_labels {
        let head = head_word(label);
        if head.is_empty() || !seen.insert(head.clone()) {
            continue;
        }
        out.push_str(&format!("{head};?;?;?;?;?\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_is_language_specific() {
        assert_eq!(
            capability_for("Russian (Русский)"),
            WordInfoCapability::Full
        );
        assert_eq!(capability_for("de"), WordInfoCapability::Full);
        assert_eq!(capability_for("uk-UA"), WordInfoCapability::Full);
        assert_eq!(capability_for("ja"), WordInfoCapability::None);
        assert_eq!(capability_for("Japanese"), WordInfoCapability::None);
        assert_eq!(capability_for("en"), WordInfoCapability::None);
    }

    #[test]
    fn diagnose_reports_missing_and_covered_labels() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let lang = root.join("Languages/Russian");
        std::fs::create_dir_all(lang.join("DefInjected/ThingDef")).unwrap();
        std::fs::create_dir_all(lang.join("WordInfo/Gender")).unwrap();
        std::fs::write(
            lang.join("DefInjected/ThingDef/T.xml"),
            "<LanguageData>\n  <A.label>пиво</A.label>\n  <B.label>квас</B.label>\n</LanguageData>\n",
        )
        .unwrap();
        // «пиво» already registered, «квас» missing.
        std::fs::write(lang.join("WordInfo/Gender/Male.txt"), "// ThingDef\nпиво\n").unwrap();

        let diag = diagnose(root, "Russian").unwrap();
        assert_eq!(diag.capability, WordInfoCapability::Full);
        assert_eq!(diag.covered_labels, 1);
        assert_eq!(diag.missing_labels, vec!["квас".to_string()]);
        let scaffold = scaffold_case(&diag);
        assert!(scaffold.contains("квас;?;?;?;?;?"));
        assert!(!scaffold.contains("пиво"));
    }

    #[test]
    fn none_capability_short_circuits() {
        let tmp = tempfile::tempdir().unwrap();
        let diag = diagnose(tmp.path(), "Japanese").unwrap();
        assert_eq!(diag.capability, WordInfoCapability::None);
        assert!(diag.missing_labels.is_empty());
    }
}
