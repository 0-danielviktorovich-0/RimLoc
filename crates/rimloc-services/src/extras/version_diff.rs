//! Cross-version workflow (§ version compatibility): compare the translatable
//! source inventory of one RimWorld version of a mod against another
//! (A → B): unchanged / changed / new / removed entries.
//!
//! Pure function over two scanned inventories — no IO here; callers scan
//! version directories via the normal services::scan pipeline.

use rimloc_core::TransUnit;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DiffCategory {
    /// Same key, same source text — translation carries over as-is.
    Unchanged,
    /// Same key, source text changed — translation needs human review.
    Changed,
    /// Key exists only in the target version.
    New,
    /// Key existed only in the source version — its translation is obsolete.
    Removed,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionDiffEntry {
    pub key: String,
    pub category: DiffCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_a: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_b: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionDiffReport {
    pub from_version: String,
    pub to_version: String,
    pub unchanged: usize,
    pub changed: usize,
    pub new: usize,
    pub removed: usize,
    /// Changed entries first (review queue), then new, then removed.
    pub entries: Vec<VersionDiffEntry>,
}

fn index_units(units: &[TransUnit]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for u in units {
        if let Some(src) = u.source.as_deref() {
            if !src.trim().is_empty() {
                map.entry(u.key.clone()).or_insert_with(|| src.to_string());
            }
        }
    }
    map
}

/// Compare source inventories of version A and version B.
pub fn version_diff(
    units_a: &[TransUnit],
    units_b: &[TransUnit],
    from_version: &str,
    to_version: &str,
) -> VersionDiffReport {
    let a = index_units(units_a);
    let b = index_units(units_b);

    let mut entries = Vec::new();
    let (mut unchanged, mut changed, mut new, mut removed) = (0usize, 0usize, 0usize, 0usize);

    for (key, src_a) in &a {
        match b.get(key) {
            Some(src_b) if src_b == src_a => {
                unchanged += 1;
                entries.push(VersionDiffEntry {
                    key: key.clone(),
                    category: DiffCategory::Unchanged,
                    source_a: Some(src_a.clone()),
                    source_b: Some(src_b.clone()),
                });
            }
            Some(src_b) => {
                changed += 1;
                entries.push(VersionDiffEntry {
                    key: key.clone(),
                    category: DiffCategory::Changed,
                    source_a: Some(src_a.clone()),
                    source_b: Some(src_b.clone()),
                });
            }
            None => {
                removed += 1;
                entries.push(VersionDiffEntry {
                    key: key.clone(),
                    category: DiffCategory::Removed,
                    source_a: Some(src_a.clone()),
                    source_b: None,
                });
            }
        }
    }
    for (key, src_b) in &b {
        if !a.contains_key(key) {
            new += 1;
            entries.push(VersionDiffEntry {
                key: key.clone(),
                category: DiffCategory::New,
                source_a: None,
                source_b: Some(src_b.clone()),
            });
        }
    }

    // Review queue ordering: changed first, then new, then removed, then rest.
    let rank = |c: DiffCategory| match c {
        DiffCategory::Changed => 0,
        DiffCategory::New => 1,
        DiffCategory::Removed => 2,
        DiffCategory::Unchanged => 3,
    };
    entries.sort_by(|x, y| {
        rank(x.category)
            .cmp(&rank(y.category))
            .then(x.key.cmp(&y.key))
    });

    VersionDiffReport {
        from_version: from_version.to_string(),
        to_version: to_version.to_string(),
        unchanged,
        changed,
        new,
        removed,
        entries,
    }
}

pub fn report_markdown(report: &VersionDiffReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Version diff {} → {}",
        report.from_version, report.to_version
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Category | Count |\n|---|---:|\n| Unchanged (reusable) | {} |\n| Changed (review) | {} |\n| New | {} |\n| Removed (obsolete translation) | {} |",
        report.unchanged, report.changed, report.new, report.removed
    );
    fn sample(entries: &[VersionDiffEntry], cat: DiffCategory) -> Vec<&VersionDiffEntry> {
        entries
            .iter()
            .filter(|e| e.category == cat)
            .take(15)
            .collect()
    }
    let section = |title: &str, list: Vec<&VersionDiffEntry>, out: &mut String| {
        if list.is_empty() {
            return;
        }
        let _ = writeln!(out, "\n## {title}\n");
        for e in list {
            let _ = writeln!(out, "- `{}`", e.key);
        }
    };
    section(
        "Changed — translations need review",
        sample(&report.entries, DiffCategory::Changed),
        &mut out,
    );
    section(
        "New in target version",
        sample(&report.entries, DiffCategory::New),
        &mut out,
    );
    section(
        "Removed from target version",
        sample(&report.entries, DiffCategory::Removed),
        &mut out,
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(key: &str, source: &str) -> TransUnit {
        TransUnit {
            key: key.into(),
            source: Some(source.into()),
            path: "x.xml".into(),
            line: None,
        }
    }

    #[test]
    fn categorizes_all_four_outcomes() {
        let a = vec![
            u("same", "Same"),
            u("changed", "Old text"),
            u("gone", "Vanished"),
        ];
        let b = vec![
            u("same", "Same"),
            u("changed", "New text"),
            u("fresh", "Added"),
        ];
        let r = version_diff(&a, &b, "1.5", "1.6");
        assert_eq!(r.unchanged, 1);
        assert_eq!(r.changed, 1);
        assert_eq!(r.new, 1);
        assert_eq!(r.removed, 1);
        assert_eq!(r.entries[0].category, DiffCategory::Changed);
    }

    #[test]
    fn markdown_reports_review_queue() {
        let a = vec![u("k", "old")];
        let b = vec![u("k", "new")];
        let md = report_markdown(&version_diff(&a, &b, "1.5", "1.6"));
        assert!(md.contains("1.5 → 1.6"));
        assert!(md.contains("| Changed (review) | 1 |"));
    }
}
