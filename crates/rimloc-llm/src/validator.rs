//! Post-translation validation: placeholder/structural integrity of provider
//! output against the source, before anything is accepted.

use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
pub struct PlaceholderIssue {
    pub id: String,
    pub kind: String,
    pub message: String,
}

/// Extract the placeholder set: printf tokens, {name}/{0} braces, [GRAMMAR] vars.
/// `{lookup: ...; Case; N}` wrappers are reduced to their inner text before
/// extraction so the inner placeholders still count.
pub fn extract_placeholders(text: &str) -> BTreeSet<String> {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE_PCT: OnceLock<Regex> = OnceLock::new();
    static RE_BRACE: OnceLock<Regex> = OnceLock::new();
    static RE_VAR: OnceLock<Regex> = OnceLock::new();
    let re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
    let re_brace = RE_BRACE.get_or_init(|| Regex::new(r"\{\s*([^{}\s]+)\s*\}").unwrap());
    let re_var = RE_VAR.get_or_init(|| Regex::new(r"\[([A-Za-z0-9_]+)\]").unwrap());

    // Reduce lookup wrappers: `{lookup: X; Case; 3}` -> `X` so inner {0} is counted.
    static RE_LOOKUP: OnceLock<Regex> = OnceLock::new();
    let re_lookup = RE_LOOKUP
        .get_or_init(|| Regex::new(r"\{lookup:\s*(.*?);\s*[A-Za-z]+\s*;\s*\d+\s*\}").unwrap());
    let reduced = re_lookup.replace_all(text, "$1");

    let mut out = BTreeSet::new();
    for m in re_pct.find_iter(&reduced) {
        out.insert(m.as_str().to_string());
    }
    for cap in re_brace.captures_iter(&reduced) {
        if let Some(name) = cap.get(1) {
            out.insert(format!("{{{}}}", name.as_str()));
        }
    }
    for cap in re_var.captures_iter(&reduced) {
        if let Some(name) = cap.get(1) {
            out.insert(format!("[{}]", name.as_str()));
        }
    }
    out
}

/// Compare source vs translation placeholder sets. Returns issues; an empty
/// vec means the translation may be accepted (structurally).
pub fn check_pair(id: &str, source: &str, translation: &str) -> Vec<PlaceholderIssue> {
    let src = extract_placeholders(source);
    let tgt = extract_placeholders(translation);
    let mut issues = Vec::new();
    let missing: Vec<_> = src.difference(&tgt).collect();
    let extra: Vec<_> = tgt.difference(&src).collect();
    if !missing.is_empty() {
        issues.push(PlaceholderIssue {
            id: id.into(),
            kind: "placeholder-missing".into(),
            message: format!("missing in translation: {missing:?}"),
        });
    }
    if !extra.is_empty() {
        issues.push(PlaceholderIssue {
            id: id.into(),
            kind: "placeholder-extra".into(),
            message: format!("extra in translation: {extra:?}"),
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_wrapper_inner_placeholder_is_preserved() {
        let src = "Drink {lookup: {0}; Case; 3}";
        assert_eq!(extract_placeholders(src), extract_placeholders("Drink {0}"));
        assert!(check_pair("x", src, "Пить {lookup: {0}; Case; 3}").is_empty());
    }

    #[test]
    fn german_replace_macro_inner_placeholders_counted() {
        // Official de pack nests {0} inside {replace: ...} macros (Alerts.xml).
        // Set-based comparison must see the inner {0}; the wrapper itself is
        // not a placeholder token.
        let src = "Colony has {0} invalid security entrances.";
        let de = "{replace: sind {0} unmengen}";
        let issues = check_pair("x", src, de);
        assert!(
            !issues.iter().any(|i| i.kind == "placeholder-missing"),
            "inner {{0}} must be counted inside {{replace:}}: {issues:?}"
        );
    }

    #[test]
    fn reordered_placeholders_are_legitimate_for_cjk() {
        // Official ja pack reorders {0},{4},{2},{3},{5} to fit Japanese syntax.
        let src = "{0} is hunting {1}. ({2}/{3}) {4}";
        let ja = "{1} が {0} を狩っています ({3}/{4}) {2}";
        assert!(check_pair("x", src, ja).is_empty());
    }

    #[test]
    fn mismatch_is_detected() {
        let issues = check_pair("x", "Hello {0} and [PAWN_label]", "Привет {0}");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "placeholder-missing");
    }
}
