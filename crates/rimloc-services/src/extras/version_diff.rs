//! Cross-version workflow (§ version compatibility): compare the translatable
//! source inventory of one RimWorld version of a mod against another
//! (A → B): unchanged / changed / new / removed entries.
//!
//! [`version_diff`] is the pure function over two already-scanned
//! inventories; [`version_diff_scan`] is the IO wrapper that scans two mod
//! roots through the normal services::scan pipeline (same pipeline the CLI
//! `version_diff` command used to inline — the command now delegates here).

use rimloc_core::{Result, TransUnit};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

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
    /// File carrying the entry in version A (absent for New entries).
    /// Additive optional fields — wire appends, never renames.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_a: Option<String>,
    /// File carrying the entry in version B (absent for Removed entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_b: Option<String>,
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

/// Index keyed by the unit key (the (family,key) identity: a RimLoc key
/// already carries the family segment, e.g. `DefInjected/ThingDef/…`).
/// First non-empty source wins — same winner rule as before; the value
/// carries the carrying file so the report can show paths.
fn index_units(units: &[TransUnit]) -> BTreeMap<String, (String, String)> {
    let mut map: BTreeMap<String, (String, String)> = BTreeMap::new();
    for u in units {
        if let Some(src) = u.source.as_deref() {
            if !src.trim().is_empty() {
                map.entry(u.key.clone())
                    .or_insert_with(|| (src.to_string(), u.path.to_string_lossy().into_owned()));
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

    for (key, (src_a, path_a)) in &a {
        match b.get(key) {
            Some((src_b, path_b)) if src_b == src_a => {
                unchanged += 1;
                entries.push(VersionDiffEntry {
                    key: key.clone(),
                    category: DiffCategory::Unchanged,
                    source_a: Some(src_a.clone()),
                    source_b: Some(src_b.clone()),
                    path_a: Some(path_a.clone()),
                    path_b: Some(path_b.clone()),
                });
            }
            Some((src_b, path_b)) => {
                changed += 1;
                entries.push(VersionDiffEntry {
                    key: key.clone(),
                    category: DiffCategory::Changed,
                    source_a: Some(src_a.clone()),
                    source_b: Some(src_b.clone()),
                    path_a: Some(path_a.clone()),
                    path_b: Some(path_b.clone()),
                });
            }
            None => {
                removed += 1;
                entries.push(VersionDiffEntry {
                    key: key.clone(),
                    category: DiffCategory::Removed,
                    source_a: Some(src_a.clone()),
                    source_b: None,
                    path_a: Some(path_a.clone()),
                    path_b: None,
                });
            }
        }
    }
    for (key, (src_b, path_b)) in &b {
        if !a.contains_key(key) {
            new += 1;
            entries.push(VersionDiffEntry {
                key: key.clone(),
                category: DiffCategory::New,
                source_a: None,
                source_b: Some(src_b.clone()),
                path_a: None,
                path_b: Some(path_b.clone()),
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

/// Options for [`version_diff_scan`]. `source_lang` names the language
/// folder treated as the SOURCE set (`English` when unset); the labels
/// default to the roots' folder names and only feed the report heading.
#[derive(Debug, Clone, Default)]
pub struct VersionDiffScanOptions {
    pub source_lang: Option<String>,
    pub from_label: Option<String>,
    pub to_label: Option<String>,
}

/// Scan BOTH mod roots through the normal services pipeline and diff the
/// resulting source inventories. This is the scan half that the CLI
/// `version_diff` command used to inline — the command now delegates here,
/// and the GUI `contract_version_diff` reuses the same entry (one diff
/// semantic everywhere). Read-only: both trees are only ever read.
pub fn version_diff_scan(
    old_root: &Path,
    new_root: &Path,
    opts: &VersionDiffScanOptions,
) -> Result<VersionDiffReport> {
    let scan_source_units = |root: &Path| -> Result<Vec<TransUnit>> {
        let auto = crate::autodiscover_defs_context(root)?;
        let units =
            crate::scan_units_with_defs_and_dict(root, None, &auto.dict, &auto.extra_fields)?;
        // Source-language units only (Defs + Languages/<source>), the same
        // filter the CLI applied inline.
        let src_dir =
            rimloc_import_po::rimworld_lang_dir(opts.source_lang.as_deref().unwrap_or("English"));
        Ok(units
            .into_iter()
            .filter(|u| crate::is_source_for_lang_dir(&u.path, &src_dir))
            .collect())
    };

    let units_a = scan_source_units(old_root)?;
    let units_b = scan_source_units(new_root)?;
    let label = |p: &Path, explicit: &Option<String>| -> String {
        if let Some(l) = explicit {
            return l.clone();
        }
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.to_string_lossy().into_owned())
    };
    Ok(version_diff(
        &units_a,
        &units_b,
        &label(old_root, &opts.from_label),
        &label(new_root, &opts.to_label),
    ))
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
            tkey: None,
            ..Default::default()
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

    /// Synthetic old/new mod trees scanned through the REAL pipeline
    /// (`version_diff_scan`): same / changed / gone / fresh keys across two
    /// `Languages/English/Keyed` trees, paths reported per side.
    #[test]
    fn version_diff_scan_classifies_synthetic_old_new_roots() {
        use std::fs;

        let old_dir = tempfile::tempdir().unwrap();
        let new_dir = tempfile::tempdir().unwrap();
        let write_keyed = |root: &Path, body: &str| {
            let keyed = root.join("Languages/English/Keyed");
            fs::create_dir_all(&keyed).unwrap();
            fs::write(keyed.join("Main.xml"), body).unwrap();
        };
        write_keyed(
            old_dir.path(),
            "<LanguageData><Same>Same</Same><Changed>Old text</Changed><Gone>Vanished</Gone></LanguageData>",
        );
        write_keyed(
            new_dir.path(),
            "<LanguageData><Same>Same</Same><Changed>New text</Changed><Fresh>Added</Fresh></LanguageData>",
        );

        let report = version_diff_scan(
            old_dir.path(),
            new_dir.path(),
            &VersionDiffScanOptions {
                source_lang: None,
                from_label: None,
                to_label: None,
            },
        )
        .unwrap();

        assert_eq!(report.unchanged, 1);
        assert_eq!(report.changed, 1);
        assert_eq!(report.new, 1);
        assert_eq!(report.removed, 1);
        // Labels default to the roots' folder names.
        assert_eq!(
            report.from_version,
            old_dir.path().file_name().unwrap().to_string_lossy()
        );
        assert_eq!(
            report.to_version,
            new_dir.path().file_name().unwrap().to_string_lossy()
        );

        let by_cat = |c: DiffCategory| -> Vec<&VersionDiffEntry> {
            report.entries.iter().filter(|e| e.category == c).collect()
        };
        let changed = by_cat(DiffCategory::Changed);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].key, "Changed");
        assert_eq!(changed[0].source_a.as_deref(), Some("Old text"));
        assert_eq!(changed[0].source_b.as_deref(), Some("New text"));
        assert!(changed[0]
            .path_a
            .as_deref()
            .unwrap_or("")
            .ends_with("Main.xml"));
        assert!(changed[0]
            .path_b
            .as_deref()
            .unwrap_or("")
            .ends_with("Main.xml"));
        // New/Removed carry the path of their side only.
        let fresh = by_cat(DiffCategory::New);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].key, "Fresh");
        assert!(fresh[0].path_a.is_none());
        assert!(fresh[0].path_b.is_some());
        let gone = by_cat(DiffCategory::Removed);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].key, "Gone");
        assert!(gone[0].path_a.is_some());
        assert!(gone[0].path_b.is_none());
    }

    /// Pure `version_diff` JSON stays additive: the new path fields may
    /// appear, but source fields and category names keep their wire form.
    #[test]
    fn entry_json_keeps_stable_field_names() {
        let a = vec![u("k", "old")];
        let b = vec![u("k", "new")];
        let r = version_diff(&a, &b, "1.5", "1.6");
        let v = serde_json::to_value(&r).unwrap();
        let entry = &v["entries"][0];
        // No rename_all on the enum: the variant name IS the wire value
        // (compat with the CLI `version_diff --format json` output).
        assert_eq!(entry["category"], "Changed");
        assert!(entry["source_a"].is_string());
        assert!(entry["source_b"].is_string());
    }
}
