//! Compare several candidate translation sets of one mod against the same
//! English source (`compare` — Phase 6 of the roadmap, future `rimloc compare`).
//!
//! The caller passes the source units (English) and one or more target sets
//! (e.g. human-made Russian vs LLM-generated Russian), each pre-filtered by
//! language the same way [`crate::coverage_report`] filters its inputs
//! (`is_source_for_lang_dir` + `normalize_lang_dir`). This module is pure:
//! it does no scanning, IO, or network work, which keeps it testable and
//! reusable from both CLI and GUI.
//!
//! Metrics computed per set:
//! - coverage of the source key set and unique key count;
//! - placeholder parity vs source (printf `%s/%d`, `{0}`/`{NAME}`, `[VAR]`);
//! - average target/source length ratio and an "overlong" counter
//!   (ratio > 1.6 is a UI-overflow risk), only for sources >= 8 chars;
//! - preserved `{lookup: …; Case; N}` wrappers;
//! - glossary adherence against an optional EN->RU dictionary.

use regex::Regex;
use rimloc_core::TransUnit;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

/// Minimum source length (in chars) below which the length ratio is not judged.
const MIN_SOURCE_LEN: usize = 8;
/// Length ratio above which a translation is flagged as overlong.
const OVERLONG_RATIO: f64 = 1.6;
/// How many uncovered source keys to sample into the report.
const SAMPLE_LIMIT: usize = 25;

/// One candidate translation set ("human", "llm", …).
///
/// `units` must already be filtered to the target language by the caller.
#[derive(Debug, Clone, Serialize)]
pub struct CompareInputSet {
    /// Free-form label shown in the report ("human", "llm", …).
    pub label: String,
    /// Translation units of this set (target language).
    pub units: Vec<TransUnit>,
}

/// Metrics of one compared set.
#[derive(Debug, Clone, Serialize)]
pub struct SetMetrics {
    /// Label copied from [`CompareInputSet::label`].
    pub label: String,
    /// Valid records (non-empty value), duplicates included.
    pub entries: usize,
    /// Unique keys among the valid records.
    pub unique_keys: usize,
    /// Share of source keys covered by this set (0..1).
    pub coverage_of_source: f64,
    /// Records whose placeholder set fully matches the source.
    pub placeholder_ok: usize,
    /// Records with a placeholder mismatch vs source.
    pub placeholder_bad: usize,
    /// Average len(target)/len(source) over records with source >= 8 chars.
    pub avg_len_ratio: f64,
    /// Records with ratio above [`OVERLONG_RATIO`] (UI overflow risk).
    pub overlong: usize,
    /// Records keeping a `{lookup: …; Case; N}`-style wrapper.
    pub lookup_wrappers: usize,
    /// Records where a source glossary term is rendered with the RU term.
    pub glossary_hits: usize,
    /// Records where a source glossary term is translated some other way.
    pub glossary_conflicts: usize,
}

/// Final comparison report. Serialize it with `serde_json` for the JSON view
/// and render via [`report_markdown`] for the human-readable one.
#[derive(Debug, Clone, Serialize)]
pub struct CompareReport {
    /// Unique source keys with non-empty text.
    pub source_total: usize,
    /// Up to 25 source keys covered by none of the sets (sorted).
    pub source_keys_sample: Vec<String>,
    /// Metrics per input set, in input order.
    pub sets: Vec<SetMetrics>,
    /// RFC3339 timestamp of report generation (UTC).
    pub generated_at: String,
}

/// Compare source units against any number of candidate sets.
///
/// `glossary` maps an English term to the expected target-language term. For
/// every record whose source contains the EN term as a whole word, the target
/// must contain the RU term (lowercase substring check) to count as a hit.
pub fn compare(
    source: &[TransUnit],
    sets: &[CompareInputSet],
    glossary: Option<&BTreeMap<String, String>>,
) -> CompareReport {
    // Source key -> source text (first non-empty occurrence wins), same
    // convention as `coverage_report` in validate.rs.
    let mut source_map: HashMap<String, String> = HashMap::new();
    for u in source {
        if let Some(text) = u.source.as_deref().filter(|t| !t.trim().is_empty()) {
            source_map
                .entry(u.key.clone())
                .or_insert_with(|| text.to_string());
        }
    }
    let source_total = source_map.len();

    // Per set: key -> value map used for coverage/unique keys (first wins).
    let key_maps: Vec<HashMap<String, String>> = sets
        .iter()
        .map(|set| {
            let mut map: HashMap<String, String> = HashMap::new();
            for u in &set.units {
                if let Some(value) = u.source.as_deref().filter(|t| !t.trim().is_empty()) {
                    map.entry(u.key.clone())
                        .or_insert_with(|| value.to_string());
                }
            }
            map
        })
        .collect();

    let union_covered: BTreeSet<String> = key_maps.iter().flat_map(|m| m.keys().cloned()).collect();
    let mut uncovered: Vec<String> = source_map
        .keys()
        .filter(|k| !union_covered.contains(*k))
        .cloned()
        .collect();
    uncovered.sort();
    let source_keys_sample: Vec<String> = uncovered.into_iter().take(SAMPLE_LIMIT).collect();

    let metrics = sets
        .iter()
        .zip(key_maps.iter())
        .map(|(set, key_map)| metrics_for_set(set, key_map, &source_map, source_total, glossary))
        .collect();

    CompareReport {
        source_total,
        source_keys_sample,
        sets: metrics,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn metrics_for_set(
    set: &CompareInputSet,
    key_map: &HashMap<String, String>,
    source_map: &HashMap<String, String>,
    source_total: usize,
    glossary: Option<&BTreeMap<String, String>>,
) -> SetMetrics {
    let mut m = SetMetrics {
        label: set.label.clone(),
        entries: 0,
        unique_keys: 0,
        coverage_of_source: 0.0,
        placeholder_ok: 0,
        placeholder_bad: 0,
        avg_len_ratio: 0.0,
        overlong: 0,
        lookup_wrappers: 0,
        glossary_hits: 0,
        glossary_conflicts: 0,
    };
    let mut ratio_sum = 0.0_f64;
    let mut ratio_count = 0_usize;

    for u in &set.units {
        let Some(value) = u.source.as_deref().filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        m.entries += 1;

        // {lookup: X; Case; N} wrapper kept intact?
        if value.contains("{lookup:") && (value.contains("; Case;") || value.contains("; Plural;"))
        {
            m.lookup_wrappers += 1;
        }

        // Metrics that require a source counterpart.
        if let Some(src) = source_map.get(&u.key) {
            if placeholder_set(src) == placeholder_set(value) {
                m.placeholder_ok += 1;
            } else {
                m.placeholder_bad += 1;
            }

            let src_len = src.chars().count();
            if src_len >= MIN_SOURCE_LEN {
                let ratio = value.chars().count() as f64 / src_len as f64;
                ratio_sum += ratio;
                ratio_count += 1;
                if ratio > OVERLONG_RATIO {
                    m.overlong += 1;
                }
            }

            if let Some(glossary) = glossary {
                let target_lc = value.to_lowercase();
                for (en_term, ru_term) in glossary {
                    if !contains_word_ci(src, en_term) {
                        continue;
                    }
                    if target_lc.contains(&ru_term.to_lowercase()) {
                        m.glossary_hits += 1;
                    } else {
                        m.glossary_conflicts += 1;
                    }
                }
            }
        }
    }

    m.unique_keys = key_map.len();
    m.coverage_of_source = if source_total == 0 {
        0.0
    } else {
        key_map
            .keys()
            .filter(|k| source_map.contains_key(*k))
            .count() as f64
            / source_total as f64
    };
    m.avg_len_ratio = if ratio_count == 0 {
        0.0
    } else {
        ratio_sum / ratio_count as f64
    };
    m
}

fn re_pct() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").expect("valid regex"))
}

fn re_brace() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\s*([^{}\s]+)\s*\}").expect("valid regex"))
}

fn re_bracket() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[([A-Za-z0-9_.]+)\]").expect("valid regex"))
}

/// Extract the placeholder set of a string: printf `%s/%d`, `{0}`/`{NAME}`
/// and `[VAR]` forms. `{lookup: …}` wrappers carry spaces inside the braces
/// and therefore produce no brace token here; they are judged separately.
fn placeholder_set(text: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for m in re_pct().find_iter(text) {
        out.insert(m.as_str().to_string());
    }
    for cap in re_brace().captures_iter(text) {
        if let Some(name) = cap.get(1) {
            out.insert(format!("{{{}}}", name.as_str()));
        }
    }
    for cap in re_bracket().captures_iter(text) {
        if let Some(name) = cap.get(1) {
            out.insert(format!("[{}]", name.as_str()));
        }
    }
    out
}

/// Case-insensitive whole-word containment check (word = run of alphanumeric
/// characters). Used for glossary terms on the source side.
fn contains_word_ci(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let hay_lc = haystack.to_lowercase();
    let needle_lc = needle.to_lowercase();
    for (pos, matched) in hay_lc.match_indices(&needle_lc) {
        let end = pos + matched.len();
        // Explicit match keeps MSRV 1.70 while staying clippy-clean.
        let before_ok = match hay_lc[..pos].chars().next_back() {
            Some(c) => !c.is_alphanumeric(),
            None => true,
        };
        let after_ok = match hay_lc[end..].chars().next() {
            Some(c) => !c.is_alphanumeric(),
            None => true,
        };
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Render the report as a compact human-readable markdown table.
pub fn report_markdown(report: &CompareReport) -> String {
    let mut out = String::new();
    out.push_str("# RimLoc compare report\n\n");
    out.push_str(&format!(
        "- Source strings (unique keys): {}\n",
        report.source_total
    ));
    out.push_str(&format!("- Generated: {}\n", report.generated_at));
    out.push_str(
        "\n| Set | Entries | Unique keys | Coverage | Placeholders ok/bad | Avg len ratio | Overlong | Lookup wrappers | Glossary hit/conflict |\n",
    );
    out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for s in &report.sets {
        out.push_str(&format!(
            "| {} | {} | {} | {:.1}% | {}/{} | {:.2} | {} | {} | {}/{} |\n",
            s.label,
            s.entries,
            s.unique_keys,
            s.coverage_of_source * 100.0,
            s.placeholder_ok,
            s.placeholder_bad,
            s.avg_len_ratio,
            s.overlong,
            s.lookup_wrappers,
            s.glossary_hits,
            s.glossary_conflicts
        ));
    }
    if !report.source_keys_sample.is_empty() {
        out.push_str(&format!(
            "\nUncovered by any set ({} of {}):\n",
            report.source_keys_sample.len(),
            report.source_total
        ));
        for key in &report.source_keys_sample {
            out.push_str(&format!("- `{key}`\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(key: &str, source: &str) -> TransUnit {
        TransUnit {
            key: key.to_string(),
            source: Some(source.to_string()),
            path: std::path::PathBuf::from("Languages/Russian/Keyed/Test.xml"),
            line: Some(1),
        }
    }

    fn set(label: &str, units: Vec<TransUnit>) -> CompareInputSet {
        CompareInputSet {
            label: label.to_string(),
            units,
        }
    }

    #[test]
    fn coverage_and_unique_keys() {
        let source = vec![
            unit("Key.Alpha", "Alpha text here"),
            unit("Key.Beta", "Beta text here"),
            unit("Key.Gamma", "Gamma text here"),
            unit("Key.Delta", "Delta text here"),
        ];
        let human = set(
            "human",
            vec![unit("Key.Alpha", "Альфа"), unit("Key.Beta", "Бета")],
        );
        // Duplicate key must not inflate unique_keys; empty value is invalid.
        let llm = set(
            "llm",
            vec![
                unit("Key.Alpha", "Альфа-2"),
                unit("Key.Alpha", "Альфа-3"),
                unit("Key.Gamma", "   "),
            ],
        );
        let report = compare(&source, &[human, llm], None);

        assert_eq!(report.source_total, 4);
        let human = &report.sets[0];
        assert_eq!(human.entries, 2);
        assert_eq!(human.unique_keys, 2);
        assert!((human.coverage_of_source - 0.5).abs() < 1e-9);
        let llm = &report.sets[1];
        assert_eq!(llm.entries, 2);
        assert_eq!(llm.unique_keys, 1);
        assert!((llm.coverage_of_source - 0.25).abs() < 1e-9);
        // Gamma and Delta are covered by no set.
        assert_eq!(report.source_keys_sample, vec!["Key.Delta", "Key.Gamma"]);
    }

    #[test]
    fn placeholder_mismatch_is_counted() {
        let source = vec![unit("Key.Pills", "Take {0} pills now")];
        let human = set("human", vec![unit("Key.Pills", "Принять {0} таблеток")]);
        let llm = set("llm", vec![unit("Key.Pills", "Принять {1} таблеток")]);
        let report = compare(&source, &[human, llm], None);
        assert_eq!(report.sets[0].placeholder_ok, 1);
        assert_eq!(report.sets[0].placeholder_bad, 0);
        assert_eq!(report.sets[1].placeholder_ok, 0);
        assert_eq!(report.sets[1].placeholder_bad, 1);
    }

    #[test]
    fn lookup_wrapper_counts_as_preserved() {
        let source = vec![unit("Key.Holds", "He is holding {0} items")];
        let wrapped = set(
            "human",
            vec![unit("Key.Holds", "Он несёт {lookup: STUFF; Case; 2}")],
        );
        let plain = set("llm", vec![unit("Key.Holds", "Он несёт {0} предметов")]);
        let report = compare(&source, &[wrapped, plain], None);
        assert_eq!(report.sets[0].lookup_wrappers, 1);
        assert_eq!(report.sets[1].lookup_wrappers, 0);
    }

    #[test]
    fn overlong_flags_long_translation_only_for_long_source() {
        let source = vec![unit("Key.Button", "Small button"), unit("Key.Ok", "OK")];
        let long_target = "Крошечная кнопка которая никогда не поместится в интерфейсе";
        let human = set(
            "human",
            vec![unit("Key.Button", long_target), unit("Key.Ok", long_target)],
        );
        let report = compare(&source, &[human], None);
        // "OK" source is 2 chars -> excluded by the 8-char rule.
        assert_eq!(report.sets[0].overlong, 1);
        assert!(report.sets[0].avg_len_ratio > 1.6);
    }

    #[test]
    fn glossary_hits_and_conflicts() {
        let source = vec![
            unit("Key.A", "The doctor heals you"),
            unit("Key.B", "Talk to the doctor again"),
            unit("Key.C", "The doctors are busy"),
        ];
        let human = set(
            "human",
            vec![
                unit("Key.A", "Врач лечит вас"),
                unit("Key.B", "Снова поговорите с медиком"),
                unit("Key.C", "Доктора заняты"),
            ],
        );
        let mut glossary = BTreeMap::new();
        glossary.insert("doctor".to_string(), "врач".to_string());
        let report = compare(&source, &[human], Some(&glossary));
        // Key.A hits; Key.B conflicts; Key.C "doctors" fails the word
        // boundary check, so its target ("Доктора") counts as nothing.
        assert_eq!(report.sets[0].glossary_hits, 1);
        assert_eq!(report.sets[0].glossary_conflicts, 1);
    }

    #[test]
    fn markdown_contains_label_and_coverage() {
        let source = vec![
            unit("Key.Alpha", "Alpha text here"),
            unit("Key.Beta", "Beta text here"),
        ];
        let human = set("human", vec![unit("Key.Alpha", "Альфа")]);
        let report = compare(&source, &[human], None);
        let md = report_markdown(&report);
        let line = md
            .lines()
            .find(|l| l.contains("| human |"))
            .expect("human row in markdown");
        assert!(line.contains("50.0%"), "coverage in row: {line}");
        assert!(md.contains("Source strings (unique keys): 2"));
    }
}
