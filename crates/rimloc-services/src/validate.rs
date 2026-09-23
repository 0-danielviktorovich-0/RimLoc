/// Canonical inventory sourcing for every validator/coverage consumer (B):
/// delegates to the SAME pipeline as CLI scan (Keyed, Defs via learned dict,
/// TKey and DefInjected-path merge). Convenience variants below must never
/// build a subtly different entry set (see the canonical inventory doc).
fn scan_canonical(
    scan_root: &Path,
    defs_root: Option<&Path>,
) -> Result<Vec<rimloc_core::TransUnit>> {
    let auto = crate::scan::autodiscover_defs_context(scan_root)?;
    crate::scan::scan_units_with_defs_and_dict(scan_root, defs_root, &auto.dict, &auto.extra_fields)
}

use crate::{util::is_source_for_lang_dir, Result, ValidationMessage};
use std::path::Path;

/// Validate scanned units under a root with optional filtering by language folder/code.
pub fn validate_under_root(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
) -> Result<Vec<ValidationMessage>> {
    let mut units = scan_canonical(scan_root, None)?;
    if let Some(dir) = source_lang_dir {
        let dir = crate::util::normalize_lang_dir(dir);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

/// Same as `validate_under_root`, but allows restricting Defs scanning to a path.
pub fn validate_under_root_with_defs(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    defs_root: Option<&Path>,
) -> Result<Vec<ValidationMessage>> {
    let mut units = scan_canonical(scan_root, defs_root)?;
    if let Some(dir) = source_lang_dir {
        let dir = crate::util::normalize_lang_dir(dir);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

pub fn validate_under_root_with_defs_and_fields(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> Result<Vec<ValidationMessage>> {
    let _ = extra_fields; // already covered by autodiscovered canonical scan
    let mut units = scan_canonical(scan_root, defs_root)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| is_source_for_lang_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

pub fn validate_under_root_with_defs_and_dict(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    defs_root: Option<&Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<ValidationMessage>> {
    let mut units =
        crate::scan::scan_units_with_defs_and_dict(scan_root, defs_root, dict, extra_fields)?;
    if let Some(dir) = source_lang_dir {
        let dir = crate::util::normalize_lang_dir(dir);
        units.retain(|u| crate::util::is_source_for_lang_dir(&u.path, &dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| crate::util::is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

/// Compare placeholders between source (English) and a target language by matching on keys.
/// This is stricter than the per-string `validate` and similar in spirit to `validate-po`.
/// Not wired to CLI by default; GUI or advanced flows can opt-in.
pub fn validate_placeholders_cross_language(
    scan_root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    defs_root: Option<&Path>,
) -> Result<Vec<ValidationMessage>> {
    let source_lang_dir = crate::util::normalize_lang_dir(source_lang_dir);
    let target_lang_dir = crate::util::normalize_lang_dir(target_lang_dir);
    fn extract_placeholders_like_cli(text: &str) -> std::collections::BTreeSet<String> {
        use regex::Regex;
        use std::sync::OnceLock;
        static RE_PCT: OnceLock<Regex> = OnceLock::new();
        static RE_BRACE: OnceLock<Regex> = OnceLock::new();
        let re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
        let re_brace = RE_BRACE.get_or_init(|| Regex::new(r"\{\s*([^{}\s]+)\s*\}").unwrap());
        let mut out: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for m in re_pct.find_iter(text) {
            out.insert(m.as_str().to_string());
        }
        for cap in re_brace.captures_iter(text) {
            if let Some(name) = cap.get(1) {
                out.insert(format!("{{{}}}", name.as_str()));
            }
        }
        out
    }
    // Scan everything, then filter per language
    let mut units = if let Some(defs) = defs_root {
        scan_canonical(scan_root, Some(defs))?
    } else {
        scan_canonical(scan_root, None)?
    };

    // Split into source and target
    let mut src_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut tgt_map: std::collections::HashMap<String, (String, String, Option<usize>)> =
        std::collections::HashMap::new();
    let mut tkey_identities: Vec<String> = Vec::new();
    for u in units.drain(..) {
        let path = u.path.clone();
        let key = u.key.clone();
        let is_tkey = u.tkey.is_some();
        if is_source_for_lang_dir(&path, &source_lang_dir) {
            if is_tkey {
                tkey_identities.push(key.clone());
            }
            if let Some(s) = u.source.as_deref() {
                if !s.trim().is_empty() {
                    src_map.entry(key).or_insert_with(|| s.to_string());
                }
            }
        } else if is_source_for_lang_dir(&path, &target_lang_dir) {
            if let Some(s) = u.source.as_deref() {
                tgt_map.insert(
                    key,
                    (s.to_string(), path.to_string_lossy().into_owned(), u.line),
                );
            }
        }
    }

    // Gate C: ONE shared resolution for every consumer. TKey-suffixed targets
    // are checked against their resolved source; ambiguity is a diagnostic.
    let src_units: Vec<rimloc_core::TransUnit> = src_map
        .iter()
        .map(|(k, s)| rimloc_core::TransUnit {
            key: k.clone(),
            source: Some(s.clone()),
            path: std::path::PathBuf::new(),
            line: None,
            tkey: None,
        })
        .collect();
    let tkey_registry = crate::matching::TKeyRegistry::from_identities(tkey_identities);
    let matcher = crate::matching::SourceMatcher::new(&src_units, &tkey_registry);
    let mut msgs = Vec::new();
    for (key, (tgt, path, line)) in tgt_map.into_iter() {
        match matcher.resolve_target(&key) {
            crate::matching::Resolution::Matched { source_key, .. } => {
                if let Some(src) = src_map.get(&source_key) {
                    let src_ph = extract_placeholders_like_cli(src);
                    let tgt_ph = extract_placeholders_like_cli(&tgt);
                    if src_ph != tgt_ph {
                        msgs.push(ValidationMessage {
                            kind: "placeholder-check".into(),
                            key,
                            path,
                            line,
                            message: "Placeholder mismatch vs source".into(),
                        });
                    }
                }
            }
            crate::matching::Resolution::Ambiguous { candidates, .. } => {
                msgs.push(ValidationMessage {
                    kind: "ambiguous-key".into(),
                    key,
                    path,
                    line,
                    message: format!("Ambiguous key: could address {}", candidates.join(", ")),
                });
            }
            crate::matching::Resolution::Unmatched { .. } => {
                // no source counterpart — the orphan validator owns this
            }
        }
    }

    Ok(msgs)
}

/// Compare list-like values by counting line breaks between source and target for matching keys.
pub fn validate_lists_cross_language(
    scan_root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    defs_root: Option<&Path>,
) -> Result<Vec<ValidationMessage>> {
    let source_lang_dir = crate::util::normalize_lang_dir(source_lang_dir);
    let target_lang_dir = crate::util::normalize_lang_dir(target_lang_dir);
    let mut units = if let Some(defs) = defs_root {
        scan_canonical(scan_root, Some(defs))?
    } else {
        scan_canonical(scan_root, None)?
    };
    let mut src: std::collections::HashMap<String, (String, Option<usize>)> =
        std::collections::HashMap::new();
    let mut tgt: std::collections::HashMap<String, (String, String, Option<usize>)> =
        std::collections::HashMap::new();
    let mut tkey_identities: Vec<String> = Vec::new();
    for u in units.drain(..) {
        if let Some(text) = u.source.as_deref() {
            let is_tkey = u.tkey.is_some();
            if is_source_for_lang_dir(&u.path, &source_lang_dir) {
                if is_tkey {
                    tkey_identities.push(u.key.clone());
                }
                src.insert(u.key.clone(), (text.to_string(), u.line));
            } else if is_source_for_lang_dir(&u.path, &target_lang_dir) {
                tgt.insert(
                    u.key.clone(),
                    (
                        text.to_string(),
                        u.path.to_string_lossy().into_owned(),
                        u.line,
                    ),
                );
            }
        }
    }
    fn li_count(s: &str) -> usize {
        if s.is_empty() {
            0
        } else {
            // Heuristic: <li> merged with newlines
            s.matches('\n').count() + 1
        }
    }
    let src_units: Vec<rimloc_core::TransUnit> = src
        .iter()
        .map(|(k, (s, line))| rimloc_core::TransUnit {
            key: k.clone(),
            source: Some(s.clone()),
            path: std::path::PathBuf::new(),
            line: *line,
            tkey: None,
        })
        .collect();
    let tkey_registry = crate::matching::TKeyRegistry::from_identities(tkey_identities);
    let matcher = crate::matching::SourceMatcher::new(&src_units, &tkey_registry);
    let mut msgs = Vec::new();
    for (k, (t, path, line)) in tgt.into_iter() {
        match matcher.resolve_target(&k) {
            crate::matching::Resolution::Matched { source_key, .. } => {
                if let Some((s, _)) = src.get(&source_key) {
                    let cs = li_count(s);
                    let ct = li_count(&t);
                    if cs != ct {
                        msgs.push(ValidationMessage {
                            kind: "list-mismatch".into(),
                            key: k,
                            path,
                            line,
                            message: format!("List items mismatch: src={cs} tgt={ct}"),
                        });
                    }
                }
            }
            crate::matching::Resolution::Ambiguous { candidates, .. } => {
                msgs.push(ValidationMessage {
                    kind: "ambiguous-key".into(),
                    key: k,
                    path,
                    line,
                    message: format!("Ambiguous key: could address {}", candidates.join(", ")),
                });
            }
            crate::matching::Resolution::Unmatched { .. } => {}
        }
    }
    Ok(msgs)
}

/// Report keys present in target language but missing in source.
pub fn validate_orphans_cross_language(
    scan_root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    defs_root: Option<&Path>,
) -> Result<Vec<ValidationMessage>> {
    let source_lang_dir = crate::util::normalize_lang_dir(source_lang_dir);
    let target_lang_dir = crate::util::normalize_lang_dir(target_lang_dir);
    let mut units = if let Some(defs) = defs_root {
        scan_canonical(scan_root, Some(defs))?
    } else {
        scan_canonical(scan_root, None)?
    };
    let mut src_units: Vec<rimloc_core::TransUnit> = Vec::new();
    let mut tgt_map: std::collections::HashMap<String, (String, Option<usize>)> =
        std::collections::HashMap::new();
    let mut tkey_identities: Vec<String> = Vec::new();
    for u in units.drain(..) {
        let is_tkey = u.tkey.is_some();
        if is_source_for_lang_dir(&u.path, &source_lang_dir) {
            if is_tkey {
                tkey_identities.push(u.key.clone());
            }
            src_units.push(u);
        } else if is_source_for_lang_dir(&u.path, &target_lang_dir) {
            tgt_map.insert(
                u.key.clone(),
                (u.path.to_string_lossy().into_owned(), u.line),
            );
        }
    }
    let tkey_registry = crate::matching::TKeyRegistry::from_identities(tkey_identities);
    let matcher = crate::matching::SourceMatcher::new(&src_units, &tkey_registry);
    let mut msgs = Vec::new();
    for (k, (path, line)) in tgt_map.into_iter() {
        match matcher.resolve_target(&k) {
            // Valid TKey alias/suffix variants are NOT orphans.
            crate::matching::Resolution::Matched { .. } => {}
            crate::matching::Resolution::Ambiguous { candidates, .. } => {
                msgs.push(ValidationMessage {
                    kind: "ambiguous-key".into(),
                    key: k,
                    path,
                    line,
                    message: format!("Ambiguous key: could address {}", candidates.join(", ")),
                });
            }
            crate::matching::Resolution::Unmatched { .. } => {
                msgs.push(ValidationMessage {
                    kind: "orphan".into(),
                    key: k,
                    path,
                    line,
                    message: "Key missing in source".into(),
                });
            }
        }
    }
    Ok(msgs)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverageReport {
    pub source_total: usize,
    pub target_total: usize,
    pub translated: usize,
    pub missing: usize,
    /// Benchmark provenance (owner clarification 24.09): SEMANTIC methodology
    /// identity, deliberately separate from the transport `schema_version`.
    /// Historical numbers must stay comparable to their own methodology —
    /// improving coverage semantics bumps this, not the JSON contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methodology: Option<CoverageMethodology>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverageMethodology {
    /// Inventory semantics: which entries exist and which one is effective.
    pub inventory_semantics: &'static str,
    /// Matcher semantics: how target keys resolve to source identities.
    pub matcher_semantics: &'static str,
    /// Eligibility ruleset currently shaping the inventory.
    pub ruleset: &'static str,
    /// Corpus identity: About packageId when readable, else the root path.
    pub corpus_identity: String,
}

/// Compute translation coverage by matching keys between source and target.
pub fn coverage_report(
    scan_root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    defs_root: Option<&Path>,
) -> Result<CoverageReport> {
    let source_lang_dir = crate::util::normalize_lang_dir(source_lang_dir);
    let target_lang_dir = crate::util::normalize_lang_dir(target_lang_dir);
    let mut units = if let Some(defs) = defs_root {
        scan_canonical(scan_root, Some(defs))?
    } else {
        scan_canonical(scan_root, None)?
    };
    use std::collections::HashMap;
    // Known TKey identities gate the canonical fallback below (§P1-5);
    // captured before the drain consumes the units.
    let tkey_registry = crate::matching::TKeyRegistry::from_identities(
        units
            .iter()
            .filter(|u| u.tkey.is_some())
            .map(|u| u.key.clone()),
    );
    let mut src: HashMap<String, String> = HashMap::new();
    let mut tgt: HashMap<String, String> = HashMap::new();
    for u in units.drain(..) {
        if let Some(text) = u.source.as_deref() {
            if crate::util::is_source_for_lang_dir(&u.path, &source_lang_dir) {
                src.entry(u.key.clone()).or_insert_with(|| text.to_string());
            } else if crate::util::is_source_for_lang_dir(&u.path, &target_lang_dir) {
                tgt.insert(u.key.clone(), text.to_string());
            }
        }
    }
    // TKey entries: target side carries canonical suffixes (.slateRef / .value.slateRef)
    // while extracted source identity is the base `<defName>.<TKey>`.
    let tgt_canonical: HashMap<String, String> = tgt
        .iter()
        .map(|(k, v)| (crate::util::canonical_match_key(k), v.clone()))
        .collect();
    let source_total = src.len();
    let mut translated = 0usize;
    for (k, _) in src.iter() {
        let value = tgt.get(k).or_else(|| {
            tkey_registry
                .identity_for(k)
                .and_then(|base| tgt_canonical.get(&base))
        });
        if let Some(v) = value {
            // RimWorld parity (1.6 decompile, LoadedLanguage): a translation
            // equal to the TODO placeholder counts as MISSING, not translated.
            let todo = v.trim().eq_ignore_ascii_case("TODO") || v.trim().is_empty();
            if !todo {
                translated += 1;
            }
        }
    }
    let missing = source_total.saturating_sub(translated);
    Ok(CoverageReport {
        source_total,
        target_total: tgt.len(),
        translated,
        missing,
        methodology: Some(CoverageMethodology {
            inventory_semantics: INVENTORY_SEMANTICS,
            matcher_semantics: MATCHER_SEMANTICS,
            ruleset: RULESET_SEMANTICS,
            corpus_identity: crate::modview::about_package_id(scan_root)
                .unwrap_or_else(|| scan_root.display().to_string()),
        }),
    })
}

/// Semantic identity of the current inventory pipeline (bump on behaviour
/// change; these are NOT transport versions).
pub const INVENTORY_SEMANTICS: &str = "canonical-v2-effective-precedence";
pub const MATCHER_SEMANTICS: &str = "tkey-registry-exact-alias-suffix-v2";
pub const RULESET_SEMANTICS: &str = "built-in-dictionaries-v0";

#[cfg(test)]
mod gate_c_tests {
    use super::*;

    const DEFS: &str = r#"<Defs>
  <QuestScriptDef>
    <defName>Q</defName>
    <label TKey="Lbl">Hello {0}</label>
    <customLetterText TKey="Txt">Body text.</customLetterText>
  </QuestScriptDef>
  <ThingDef>
    <defName>Bogus</defName>
    <label>bogus label</label>
  </ThingDef>
</Defs>"#;

    const RU: &str = r#"<LanguageData>
  <Q.Lbl.slateRef>Привет</Q.Lbl.slateRef>
  <Q.Txt.slateRef>Текст.</Q.Txt.slateRef>
  <Ghost.Field.slateRef>хм</Ghost.Field.slateRef>
  <Bogus.label.slateRef>не алиас</Bogus.label.slateRef>
</LanguageData>"#;

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let defs = tmp.path().join("Defs/Misc");
        std::fs::create_dir_all(&defs).unwrap();
        std::fs::write(defs.join("Q.xml"), DEFS).unwrap();
        let ru = tmp
            .path()
            .join("Languages/Russian/DefInjected/QuestScriptDef");
        std::fs::create_dir_all(&ru).unwrap();
        std::fs::write(ru.join("Q.xml"), RU).unwrap();
        tmp
    }

    /// Valid TKey-suffix targets are not orphans; genuinely unrelated
    /// `.slateRef` paths (no TKey identity behind the base) stay orphans —
    /// shape-stripping must not alias them to a plain source unit.
    #[test]
    fn orphans_use_shared_resolution() {
        let tmp = fixture();
        let mut msgs =
            validate_orphans_cross_language(tmp.path(), "English", "Russian", None).unwrap();
        msgs.sort_by(|a, b| a.key.cmp(&b.key));
        let keys: Vec<&str> = msgs.iter().map(|m| m.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["Bogus.label.slateRef", "Ghost.Field.slateRef"],
            "{msgs:?}"
        );
        assert!(msgs.iter().all(|m| m.kind == "orphan"), "{msgs:?}");
        // The two valid TKey-suffixed entries produced no orphan messages.
    }

    /// Placeholder checking works ACROSS the TKey-suffix resolution.
    #[test]
    fn placeholders_checked_across_tkey_suffix() {
        let tmp = fixture();
        let msgs =
            validate_placeholders_cross_language(tmp.path(), "English", "Russian", None).unwrap();
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert_eq!(msgs[0].key, "Q.Lbl.slateRef");
        assert_eq!(msgs[0].kind, "placeholder-check");
    }

    #[test]
    fn lists_validator_resolves_tkey_suffix_silently() {
        let tmp = fixture();
        let msgs = validate_lists_cross_language(tmp.path(), "English", "Russian", None).unwrap();
        assert!(msgs.is_empty(), "{msgs:?}");
    }
}
