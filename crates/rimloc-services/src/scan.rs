use crate::plugins;
use crate::{
    util::{def_injected_target_path, is_under_languages_dir},
    Result, TransUnit,
};
use rimloc_core::{winner_reason, SourceRef};
use rimloc_parsers_xml::DefsMetaUnit;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_SOURCE_LANG_DIR: &str = "English";

fn seen_key(path: &Path, key: &str) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("{normalized}|{key}")
}

/// Defs duplicate semantics (Gate H): the game logs
/// `Adding duplicate <DefType> name: X` and GetDefSilentFail returns the
/// FIRST registration, so the first file's fields are authoritative and a
/// later file's same-identity fields never enter the inventory.
///
/// `owned` carries the registered identities ACROSS calls so a LoadFolders
/// mod's content dirs resolve in view order (`/` before `1.6`, unconditional
/// before IfModActive) and the first dir's registration wins — not whichever
/// file happens to sort last. Every kept unit records the real source
/// location (`src`) and the `first-file-wins` winner reason; `path` stays
/// the canonical DefInjected output path for export grouping.
fn merge_defs_units(
    units: &mut Vec<TransUnit>,
    seen: &mut HashSet<String>,
    scan_root: &Path,
    lang_dir: &str,
    defs_meta: Vec<DefsMetaUnit>,
    owned: &mut HashSet<(String, String)>,
) {
    // Deterministic "first file" = lexicographic path order within one batch
    // (the documented stand-in for the game's file-system enumeration on
    // Windows); cross-dir order comes from the caller's view order.
    let mut defs_meta = defs_meta;
    defs_meta.sort_by(|a, b| a.unit.path.as_os_str().cmp(b.unit.path.as_os_str()));
    for meta in defs_meta {
        let mut unit = meta.unit;
        if unit
            .source
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            continue;
        }
        if !owned.insert((meta.def_type.clone(), unit.key.clone())) {
            continue;
        }
        unit.src = Some(SourceRef {
            file: unit.path.clone(),
            line: unit.line,
        });
        unit.selected_by = Some(winner_reason::DEFS_FIRST_FILE.into());
        let target_path = def_injected_target_path(scan_root, lang_dir, &meta.def_type, &unit.path);
        unit.path = target_path;
        unit.line = None;
        let key = seen_key(&unit.path, &unit.key);
        if seen.insert(key) {
            units.push(unit);
        }
    }
}

/// TKey units share Defs semantics (first registration wins). The parser
/// deduplicates inside one scan call and already stamps the winner reason
/// (first-file-wins, or tkey-last-assignment when several same-file nodes
/// share the identity); this wrapper deduplicates ACROSS content dirs
/// (LoadFolders view order) via the shared `owned` set — the winning file's
/// own reason is preserved.
fn merge_tkey_units(
    units: &mut Vec<TransUnit>,
    seen: &mut HashSet<String>,
    tkey_units: Vec<TransUnit>,
    owned: &mut HashSet<(String, String)>,
) {
    let mut tkey_units = tkey_units;
    tkey_units.sort_by(|a, b| a.path.as_os_str().cmp(b.path.as_os_str()));
    for mut unit in tkey_units {
        let Some(def_type) = unit.tkey.as_ref().map(|m| m.def_type.clone()) else {
            continue;
        };
        if !owned.insert((def_type, unit.key.clone())) {
            continue;
        }
        if unit.selected_by.is_none() {
            unit.selected_by = Some(winner_reason::DEFS_FIRST_FILE.into());
        }
        let key = seen_key(&unit.path, &unit.key);
        if seen.insert(key) {
            units.push(unit);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutoDefsContext {
    pub dict: HashMap<String, Vec<String>>,
    pub extra_fields: Vec<String>,
    pub learned_sources: Vec<PathBuf>,
    pub dict_sources: Vec<PathBuf>,
}

fn merge_dict_sets(
    target: &mut HashMap<String, BTreeSet<String>>,
    source: &HashMap<String, Vec<String>>,
) {
    for (def_type, fields) in source {
        let entry = target.entry(def_type.clone()).or_default();
        for field in fields {
            entry.insert(field.clone());
        }
    }
}

fn flatten_dict_sets(map: HashMap<String, BTreeSet<String>>) -> HashMap<String, Vec<String>> {
    map.into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LearnedDefRow {
    #[serde(rename = "defType")]
    def_type: String,
    #[serde(rename = "fieldPath")]
    field_path: String,
}

fn load_learned_defs(path: &Path) -> Result<Vec<LearnedDefRow>> {
    let file = std::fs::File::open(path)?;
    let rows: Vec<LearnedDefRow> = serde_json::from_reader(file)?;
    Ok(rows)
}

fn autodiscover_learn_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    let mut push_dir = |p: PathBuf| {
        if p.is_dir() && seen.insert(p.clone()) {
            dirs.push(p);
        }
    };

    for name in ["_learn", "learn_out", "Learn", "learn"] {
        push_dir(root.join(name));
    }

    let languages_root = root.join("Languages");
    if languages_root.is_dir() {
        // Walk only shallow levels under Languages to catch nested _learn/learn_out folders.
        for entry in WalkDir::new(&languages_root)
            .min_depth(1)
            .max_depth(4)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.eq_ignore_ascii_case("_learn")
                || name.eq_ignore_ascii_case("learn_out")
                || name.eq_ignore_ascii_case("learn")
                || name.eq_ignore_ascii_case("Learn")
            {
                push_dir(entry.into_path());
            }
        }
    }

    dirs
}

fn load_dict_candidates(dirs: &[PathBuf]) -> (HashMap<String, Vec<String>>, Vec<PathBuf>) {
    let mut merged: HashMap<String, Vec<String>> = HashMap::new();
    let mut sources = Vec::new();
    for dir in dirs {
        if let Ok(read) = std::fs::read_dir(dir) {
            for entry in read.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let is_json = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false);
                if !is_json {
                    continue;
                }
                let file_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if !(file_name.contains("defs_dict") || file_name.ends_with(".dict.json")) {
                    continue;
                }
                if let Ok(dict) = rimloc_parsers_xml::load_defs_dict_from_file(&path) {
                    for (def_type, fields) in dict.0 {
                        merged.entry(def_type).or_default().extend(fields);
                    }
                    sources.push(path);
                }
            }
        }
    }
    for v in merged.values_mut() {
        v.sort();
        v.dedup();
    }
    (merged, sources)
}

pub fn autodiscover_defs_context(root: &Path) -> Result<AutoDefsContext> {
    let mut dict_sets: HashMap<String, BTreeSet<String>> = HashMap::new();
    merge_dict_sets(
        &mut dict_sets,
        &rimloc_parsers_xml::load_embedded_defs_dict().0,
    );

    let learn_dirs = autodiscover_learn_dirs(root);
    let mut learned_sources = Vec::new();
    let mut extra_fields: BTreeSet<String> = BTreeSet::new();

    for dir in &learn_dirs {
        let candidate = dir.join("learned_defs.json");
        if candidate.is_file() {
            if let Ok(rows) = load_learned_defs(&candidate) {
                learned_sources.push(candidate.clone());
                for row in rows {
                    if row.field_path.contains('.') {
                        dict_sets
                            .entry(row.def_type.clone())
                            .or_default()
                            .insert(row.field_path.clone());
                    } else {
                        extra_fields.insert(row.field_path.clone());
                    }
                }
            }
        }
    }

    let (mut discovered_dicts, dict_sources) = load_dict_candidates(&learn_dirs);
    for (k, v) in discovered_dicts.drain() {
        dict_sets.entry(k).or_default().extend(v);
    }

    Ok(AutoDefsContext {
        dict: flatten_dict_sets(dict_sets),
        extra_fields: extra_fields.into_iter().collect(),
        learned_sources,
        dict_sources,
    })
}

pub fn scan_units_auto(root: &Path) -> Result<Vec<TransUnit>> {
    let auto = autodiscover_defs_context(root)?;
    let mut units = rimloc_parsers_xml::scan_keyed_xml(root)?;
    let mut seen: HashSet<String> = units.iter().map(|u| seen_key(&u.path, &u.key)).collect();
    // TKey units are part of the canonical inventory (P1-1): consumers of the
    // auto entrypoint (word-info) must see the same entries as CLI scan.
    if let Ok(tkey) = rimloc_parsers_xml::scan_defs_tkey(root, None) {
        let mut owned_tkey: HashSet<(String, String)> = HashSet::new();
        merge_tkey_units(&mut units, &mut seen, tkey, &mut owned_tkey);
    }
    let defs_meta =
        rimloc_parsers_xml::scan_defs_with_dict_meta(root, None, &auto.dict, &auto.extra_fields)?;
    let mut owned_defs: HashSet<(String, String)> = HashSet::new();
    merge_defs_units(
        &mut units,
        &mut seen,
        root,
        DEFAULT_SOURCE_LANG_DIR,
        defs_meta,
        &mut owned_defs,
    );
    Ok(units)
}

/// Convert PatchOperation text candidates (learn/patches) into TransUnits when
/// a DefInjected key can be inferred from xpath/tag path.
pub fn scan_patches_as_units(
    root: &Path,
    min_len: usize,
    strict_xpath: bool,
) -> Result<Vec<TransUnit>> {
    let mut out = Vec::new();
    // Delegate to learn/patches to collect candidates
    let mut candidates = crate::learn::patches::scan_patches_texts(root, min_len)?;
    // Avoid excessive size; keep deterministic order
    candidates.sort_by(|a, b| {
        a.source_file
            .cmp(&b.source_file)
            .then(a.tag_path.cmp(&b.tag_path))
    });
    for c in candidates {
        let inferred = if strict_xpath {
            // already inferred using strict flag inside scan; but recompute just in case
            if let (Some(xp), tp) = (c.xpath.as_deref(), c.tag_path.as_str()) {
                crate::learn::patches::infer_definj_from_xpath_mode(xp, tp, true)
            } else {
                None
            }
        } else {
            c.inferred
        };
        if let Some(meta) = inferred {
            if !meta.def_name.trim().is_empty() && !meta.field_path.trim().is_empty() {
                out.push(TransUnit {
                    key: format!("{}.{}", meta.def_name, meta.field_path),
                    source: Some(c.value.clone()),
                    // Keep patch file as path; exporters group to _Imported.xml when not under Languages
                    path: c.source_file.clone(),
                    line: None,
                    tkey: None,
                    ..Default::default()
                });
            }
        }
    }
    Ok(out)
}

/// Override Keyed unit values with preceding XML comments that match a prefix (e.g., "EN:")
/// for files under `Languages/<lang_dir>/Keyed/`. Returns number of overrides applied.
pub fn override_keyed_units_from_comments(
    units: &mut [TransUnit],
    lang_dir: &str,
    prefix: &str,
) -> Result<usize> {
    use std::collections::BTreeMap;
    let mut applied = 0usize;
    let mut cache: HashMap<PathBuf, BTreeMap<String, String>> = HashMap::new();
    for u in units.iter_mut() {
        let path_str = u.path.to_string_lossy();
        if !(rimloc_core::path_text::has_path_marker(&path_str, "Languages")) {
            continue;
        }
        if !(rimloc_core::path_text::has_path_marker(&path_str, "Keyed")) {
            continue;
        }
        if !crate::util::is_under_languages_dir(&u.path, lang_dir) {
            continue;
        }
        let map = match cache.get(&u.path) {
            Some(m) => m,
            None => {
                let m =
                    rimloc_parsers_xml::read_keyed_file_map_with_comments(&u.path, Some(prefix))
                        .unwrap_or_default();
                cache.insert(u.path.clone(), m);
                cache.get(&u.path).unwrap()
            }
        };
        if let Some(val) = map.get(&u.key) {
            u.source = Some(val.clone());
            applied += 1;
        }
    }
    Ok(applied)
}

/// Scan a RimWorld mod folder and return discovered translation units.
/// This wraps `rimloc_parsers_xml::scan_keyed_xml` to provide a stable entrypoint
/// for higher-level clients (CLI, GUI, LSP) without importing parser crates.
pub fn scan_units(root: &Path) -> Result<Vec<TransUnit>> {
    // Include both LanguageData (Keyed/DefInjected) and implicit English from Defs
    let mut units = scan_units_auto(root)?;
    // Run registered parser plugins to augment scan results
    plugins::init_builtin();
    for p in plugins::iter() {
        if let Ok(mut more) = p.scan_units(root) {
            units.append(&mut more);
        }
    }
    Ok(units)
}

/// Like `scan_units`, but restrict Defs scanning to a particular directory when provided.
pub fn scan_units_with_defs(
    root: &Path,
    defs_root: Option<&std::path::Path>,
) -> Result<Vec<TransUnit>> {
    let auto = autodiscover_defs_context(root)?;
    scan_units_with_defs_and_dict(root, defs_root, &auto.dict, &auto.extra_fields)
}

pub fn scan_units_with_defs_and_fields(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    let auto = autodiscover_defs_context(root)?;
    let mut merged_fields: Vec<String> = auto.extra_fields.clone();
    merged_fields.extend(extra_fields.iter().cloned());
    merged_fields.sort();
    merged_fields.dedup();
    scan_units_with_defs_and_dict(root, defs_root, &auto.dict, &merged_fields)
}

pub fn scan_units_with_defs_and_dict(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    Ok(scan_units_with_defs_and_dict_full(root, defs_root, dict, extra_fields)?.units)
}

/// Full-result variant of [`scan_units_with_defs_and_dict`]: the same single
/// pipeline, but it also surfaces the REAL patch report (coverage computed
/// from the applied operations, not guessed from directory existence).
/// Flat mods have no mod view.
pub fn scan_units_with_defs_and_dict_full(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<EffectiveScan> {
    let mut units = rimloc_parsers_xml::scan_keyed_xml(root)?;
    let mut seen: HashSet<String> = units.iter().map(|u| seen_key(&u.path, &u.key)).collect();
    let defs_meta =
        rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)?;
    // TKey nodes (QuestScriptDefs/TipSetDefs/...; the TKey system exists
    // since RimWorld 1.1, 2020 — 1.6 is the primary tested corpus):
    // explicit translation keys
    // on def fields, identity `<defName>.<TKey>`.
    if let Ok(tkey) = rimloc_parsers_xml::scan_defs_tkey(root, defs_root) {
        let mut owned_tkey: HashSet<(String, String)> = HashSet::new();
        merge_tkey_units(&mut units, &mut seen, tkey, &mut owned_tkey);
    }
    let mut owned_defs: HashSet<(String, String)> = HashSet::new();
    merge_defs_units(
        &mut units,
        &mut seen,
        root,
        DEFAULT_SOURCE_LANG_DIR,
        defs_meta,
        &mut owned_defs,
    );
    // Optional fuzzy candidates from Defs
    if matches!(std::env::var("RIMLOC_FUZZY"), Ok(v) if v.trim()=="1") {
        if let Ok(mut fuzzy) = rimloc_parsers_xml::scan_defs_fuzzy(root, defs_root) {
            for u in fuzzy.drain(..) {
                let k = seen_key(&u.path, &u.key);
                if seen.insert(k) {
                    units.push(u);
                }
            }
            units.sort_by(|a, b| {
                (
                    a.path.to_string_lossy(),
                    a.line.unwrap_or(0),
                    a.key.as_str(),
                )
                    .cmp(&(
                        b.path.to_string_lossy(),
                        b.line.unwrap_or(0),
                        b.key.as_str(),
                    ))
            });
        }
    }
    let mut patch_dirs = vec![root.join("Patches")];
    let mut merged = crate::patches_effect::PatchReport::default();
    for pd in patch_dirs.drain(..) {
        if pd.is_dir() {
            let (u, r) = crate::patches_effect::apply_patch_stage(units, &pd);
            units = u;
            merged.merge_from(&r);
        }
    }
    merged.finalize();
    apply_effective_precedence(&mut units);
    Ok(EffectiveScan {
        units,
        patch: merged,
        view: None,
    })
}

/// Full result of the canonical scan pipeline: the effective units, the REAL
/// patch report (coverage from the operations actually evaluated) and the
/// resolved mod view (`None` for a flat mod without LoadFolders/version
/// dirs). One pipeline, richer evidence — never a parallel scanner.
#[derive(Debug, Default)]
pub struct EffectiveScan {
    pub units: Vec<TransUnit>,
    pub patch: crate::patches_effect::PatchReport,
    pub view: Option<crate::modview::EffectiveModView>,
}

/// Effective RimWorld source precedence (Gate H — semantic correctness, not
/// duplicate cleanup). Authoritative rules from the 1.6 decompile
/// (GAME_SOURCE_FINDINGS §1.4/§2.1):
/// - Keyed: duplicate within ONE file → error in game, FIRST value taken;
///   duplicates across files → `SetOrAdd`, LAST loaded wins (file order:
///   lexicographic path is RimLoc's deterministic stand-in for Windows FS
///   enumeration).
/// - DefInjected: duplicate key → `SetOrAdd` overwrite, LAST wins.
/// - Defs (duplicate defName): `GetDefSilentFail` takes the FIRST
///   registration — enforced inside `merge_defs_units` (first file owns the
///   identity); TKey shares Def semantics (first file wins, in-file
///   last-wins per field assignment).
///
/// Overridden values are dropped from the authoritative inventory; context
/// preservation as diagnostics lands with the canonical model (Gate I).
/// Paths stay case-exact except the already-documented case-insensitive
/// About/LoadFolders resolution.
///
/// Every surviving winner records WHY it won in `selected_by`
/// ([`rimloc_core::winner_reason`]): the family rule that decided it
/// (`first-file-wins` / `keyed-last-wins` / `keyed-first-in-file` /
/// `definjected-setoradd`), or an earlier, more specific producer
/// (`patch-applied`). View-selection facts stay in the canonical
/// provenance (`version_selected`/`conditional_branch`).
pub fn apply_effective_precedence(units: &mut Vec<TransUnit>) {
    // Deterministic load order: ascending (path, line) — the documented
    // stand-in for the game's file-system enumeration order.
    units.sort_by(|a, b| {
        (
            a.path.to_string_lossy(),
            a.line.unwrap_or(0),
            a.key.as_str(),
        )
            .cmp(&(
                b.path.to_string_lossy(),
                b.line.unwrap_or(0),
                b.key.as_str(),
            ))
    });
    let is_keyed = |p: &std::path::Path| {
        rimloc_core::path_text::has_path_marker(&p.to_string_lossy(), "Keyed")
    };
    let is_definj = |p: &std::path::Path| {
        rimloc_core::path_text::has_path_marker(&p.to_string_lossy(), "DefInjected")
    };
    // DefInjected identities live per DEF TYPE: two def types may share the
    // same `{defName}.{field}` key, and those are different Defs — never
    // competitors. The scope segment after /DefInjected/ (real sidecar or
    // canonical virtual output path) carries it.
    let definj_scope = |p: &std::path::Path| -> String {
        let s = p.to_string_lossy().replace('\\', "/");
        match s.find("/DefInjected/") {
            Some(i) => s[i + "/DefInjected/".len()..]
                .split('/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase(),
            None => String::new(),
        }
    };
    // Language-side scopes: fold to the effective winner per key WITHIN one
    // language pack (EN source and RU target are separate LanguageDatabase
    // entries — precedence never crosses languages).
    // Keyed: winner = last file (cross-file last-wins) but within one file
    // the FIRST occurrence (game logs an error and takes the first).
    // DefInjected: last occurrence everywhere (SetOrAdd overwrite).
    let lang_of = |p: &std::path::Path| -> String {
        let s = p.to_string_lossy();
        let s = s.replace('\\', "/");
        match s.find("/Languages/") {
            Some(i) => {
                let rest = &s[i + "/Languages/".len()..];
                rest.split('/').next().unwrap_or_default().to_string()
            }
            None => String::new(),
        }
    };
    let mut effective: Vec<TransUnit> = Vec::with_capacity(units.len());
    let mut index: std::collections::HashMap<(String, String, String), usize> =
        std::collections::HashMap::new();
    let mut first_in_file: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    for u in units.drain(..) {
        let scoped = is_keyed(&u.path) || is_definj(&u.path);
        if !scoped {
            effective.push(u);
            continue;
        }
        let file = u.path.to_string_lossy().into_owned();
        let lang = lang_of(&u.path);
        let same_file_seen = first_in_file.contains(&(file.clone(), u.key.clone()));
        first_in_file.insert((file.clone(), u.key.clone()));
        let scope = if is_keyed(&u.path) {
            "keyed".to_string()
        } else {
            format!("definj:{}", definj_scope(&u.path))
        };
        let idx_key = (lang, scope, u.key.clone());
        match index.get(&idx_key).copied() {
            None => {
                // First occurrence: it wins by its family rule — unless a
                // more specific producer (a patch operation) already claimed
                // the reason.
                let mut u = u;
                if u.selected_by.is_none() {
                    u.selected_by = Some(if is_keyed(&u.path) {
                        winner_reason::KEYED_FIRST_IN_FILE.into()
                    } else {
                        winner_reason::DEFINJECTED_SETORADD.into()
                    });
                }
                index.insert(idx_key, effective.len());
                effective.push(u);
            }
            Some(pos) if is_keyed(&effective[pos].path) && !same_file_seen => {
                // Cross-file Keyed duplicate: last loaded wins.
                let mut u = u;
                if u.selected_by.as_deref() != Some(winner_reason::PATCH_APPLIED) {
                    u.selected_by = Some(winner_reason::KEYED_LAST_FILE.into());
                }
                effective[pos] = u;
            }
            Some(pos) if is_definj(&effective[pos].path) => {
                // DefInjected SetOrAdd: every later occurrence overwrites,
                // including within the same file.
                let mut u = u;
                if u.selected_by.as_deref() != Some(winner_reason::PATCH_APPLIED) {
                    u.selected_by = Some(winner_reason::DEFINJECTED_SETORADD.into());
                }
                effective[pos] = u;
            }
            Some(_) => {
                // Keyed duplicate within the SAME file: the game logs
                // `Duplicate keyed translation key` and takes the FIRST
                // value, so the first stays the effective winner — but the
                // duplicate unit is kept so the validator still reports the
                // error (diagnostic preservation beats silent dropping).
                // The duplicate is a loser: it carries no winner reason.
                let mut u = u;
                u.selected_by = None;
                effective.push(u);
            }
        }
    }
    *units = effective;
}

/// Keep the canonical source inventory on the SOURCE language side: units
/// under `Languages/<other>` are existing TARGET packs (someone else's
/// translations), never English source text — the canonical model hard-codes
/// `source_locale: "en"`, so a foreign pack must not become the source even
/// when the mod ships no English sidecar at all (a target-only pack or a
/// TKey-only Defs mod). Units outside Languages/ (Defs, TKey, patch
/// candidates) are always source-side; this guard runs on the CANONICAL
/// path only — the legacy scan APIs keep their whole-language behaviour.
pub fn retain_source_language_units(units: &mut Vec<TransUnit>) {
    let in_any_languages = |p: &std::path::Path| {
        let s = p.to_string_lossy();
        rimloc_core::path_text::has_path_marker(&s, "Languages")
    };
    units.retain(|u| {
        if !in_any_languages(&u.path) {
            return true;
        }
        is_under_languages_dir(&u.path, DEFAULT_SOURCE_LANG_DIR)
    });
}

pub fn scan_defs_with_meta(
    root: &Path,
    defs_root: Option<&Path>,
    dict: &HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<DefsMetaUnit>> {
    rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn autodiscover_learn_dirs_finds_nested_under_languages() -> Result<()> {
        let dir = tempdir()?;
        let nested = dir
            .path()
            .join("Languages")
            .join("English")
            .join("DefInjected")
            .join("_learn");
        fs::create_dir_all(&nested)?;
        let learned = nested.join("learned_defs.json");
        fs::write(
            &learned,
            r#"[{"defType":"ThingDef","fieldPath":"description"}]"#,
        )?;

        let ctx = autodiscover_defs_context(dir.path())?;
        assert!(ctx
            .learned_sources
            .iter()
            .any(|p| p.file_name().and_then(|s| s.to_str()) == Some("learned_defs.json")));
        assert!(ctx
            .dict
            .get("ThingDef")
            .map(|fields| fields.iter().any(|f| f == "description"))
            .unwrap_or(false));
        Ok(())
    }
}

#[cfg(test)]
mod tkey_scan_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn tkey_defs_pass_includes_parms_shape_via_services_pipeline() {
        let dir = tempdir().unwrap();
        let defs = dir.path().join("Defs/Misc");
        fs::create_dir_all(&defs).unwrap();
        fs::write(
            defs.join("TKeySamples.xml"),
            r#"<Defs>
  <TipSetDef>
    <defName>SampleTips</defName>
    <tips><li TKey="DismissLetters">tip text</li></tips>
  </TipSetDef>
  <QuestScriptDef>
    <defName>SampleQuest</defName>
    <label TKey="LetterLabelFavorReceiver">favor label</label>
    <customLetterText TKey="LetterTextSample">quest text.</customLetterText>
    <node Class="QuestNode_SubScript">
      <parms>
        <li Class="NamedParm">
          <key>letterText</key>
          <value TKey="LetterTextParms">parms text</value>
        </li>
      </parms>
    </node>
  </QuestScriptDef>
</Defs>"#,
        )
        .unwrap();
        let units = scan_units_with_defs_and_dict(dir.path(), None, &HashMap::new(), &[]).unwrap();
        let tkey_keys: Vec<&str> = units
            .iter()
            .filter(|u| u.tkey.is_some())
            .map(|u| u.key.as_str())
            .collect();
        assert_eq!(tkey_keys.len(), 4, "{tkey_keys:?}");
        assert!(
            tkey_keys.contains(&"SampleQuest.LetterTextParms"),
            "{tkey_keys:?}"
        );
    }
}

/// LoadFolders-aware effective scan (Gate H). When the mod carries a
/// LoadFolders.xml and a target version is resolvable, Keyed/DefInjected come
/// from the EFFECTIVE languages dirs and Defs ONLY from the EFFECTIVE Defs
/// roots for that version — never the union across versions. Without
/// LoadFolders this is equivalent to [`scan_units_with_defs_and_dict`].
pub fn scan_units_effective(
    root: &Path,
    requested_version: Option<&str>,
    dict: &HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    Ok(scan_units_effective_full(root, requested_version, dict, extra_fields)?.units)
}

/// Full-result variant of [`scan_units_effective`]: the same single pipeline
/// plus the REAL patch report and the resolved mod view (version + which
/// dirs came from `IfModActive` branches), so callers can derive honest
/// provenance and view labels instead of re-deriving them.
pub fn scan_units_effective_full(
    root: &Path,
    requested_version: Option<&str>,
    dict: &HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<EffectiveScan> {
    let view = crate::modview::effective_view(root, requested_version)?;
    // A genuinely flat mod (no version dirs chosen, everything at root) is
    // exactly the legacy single-root pipeline. Everything else — LoadFolders
    // or a classic version-dir layout — is scanned through the SAME
    // view-driven path, so the version resolver always decides which roots
    // load (never a cross-version union).
    let flat = view.version.is_none()
        && view.conditional_dirs.is_empty()
        && view.content_dirs.len() == 1
        && view.content_dirs[0] == root;
    if flat {
        let mut scan = scan_units_with_defs_and_dict_full(root, None, dict, extra_fields)?;
        scan.view = Some(view);
        return Ok(scan);
    }
    scan_units_effective_view(root, view, dict, extra_fields)
}

/// View-driven effective scan: the resolved [`EffectiveModView`] decides
/// which Languages dirs and Defs roots load — reused verbatim by both the
/// LoadFolders and the classic version-dir layouts (one pipeline, one
/// resolver, no re-derived precedence anywhere).
pub fn scan_units_effective_view(
    root: &Path,
    view: crate::modview::EffectiveModView,
    dict: &HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<EffectiveScan> {
    let mut units: Vec<TransUnit> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    // First-registration books shared ACROSS content dirs: view order
    // (root, 1.6, IfModActive superset) decides which file owns an identity.
    let mut owned_defs: HashSet<(String, String)> = HashSet::new();
    let mut owned_tkey: HashSet<(String, String)> = HashSet::new();

    // Languages from the effective dirs only (root + content folders that
    // exist for this version).
    for lang_dir in view.languages_dirs() {
        if let Ok(mut scoped) = rimloc_parsers_xml::scan_keyed_xml(&lang_dir) {
            // Same-file duplicates are Gate H diagnostics (the in-file loser
            // becomes an Overridden context) — dedup only ACROSS language
            // dirs, never within one scan batch.
            let mut batch: HashSet<String> = HashSet::new();
            for u in scoped.drain(..) {
                let k = seen_key(&u.path, &u.key);
                let dup_in_batch = !batch.insert(k.clone());
                if dup_in_batch || seen.insert(k) {
                    units.push(u);
                }
            }
        }
    }
    // Defs strictly from the effective roots (version-scoped; IfModActive
    // dirs are included per the documented offline superset policy).
    for defs_root in view.defs_roots() {
        let defs_root = Some(defs_root.as_path());
        let defs_meta =
            rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)?;
        if let Ok(tkey) = rimloc_parsers_xml::scan_defs_tkey(root, defs_root) {
            merge_tkey_units(&mut units, &mut seen, tkey, &mut owned_tkey);
        }
        merge_defs_units(
            &mut units,
            &mut seen,
            root,
            DEFAULT_SOURCE_LANG_DIR,
            defs_meta,
            &mut owned_defs,
        );
    }
    let mut patch_dirs = vec![root.join("Patches")];
    for dir in view.content_dirs.iter().chain(view.conditional_dirs.iter()) {
        patch_dirs.push(dir.join("Patches"));
    }
    let mut merged = crate::patches_effect::PatchReport::default();
    for pd in patch_dirs.drain(..) {
        if pd.is_dir() {
            let (u, r) = crate::patches_effect::apply_patch_stage(units, &pd);
            units = u;
            merged.merge_from(&r);
        }
    }
    merged.finalize();
    // Per-entry conditional provenance: a unit whose effective source file
    // (or canonical path) sits under an IfModActive dir is part of the
    // offline superset only — the game loads it conditionally.
    if !view.conditional_dirs.is_empty() {
        for u in units.iter_mut() {
            let under = view.conditional_dirs.iter().any(|d| {
                u.path.starts_with(d) || u.src.as_ref().is_some_and(|s| s.file.starts_with(d))
            });
            u.conditional = under;
        }
    }
    apply_effective_precedence(&mut units);
    Ok(EffectiveScan {
        units,
        patch: merged,
        view: Some(view),
    })
}

/// Stable fingerprint of the source content a project inventory was built
/// from (M3): the resolved effective view (including the RESOLVED game
/// version - a LoadFolders rollback changes it) plus `(relative path,
/// sha256)` of every file under the trees the scan pipeline reads
/// (Languages, Defs, Patches). Deterministic across runs and platforms
/// (sorted walk, `/`-normalized relative paths). A version rollback, a
/// content edit or an added/removed file each change the value.
///
/// Cost note: one extra sequential read pass over the scanned trees, per
/// project (re)start - bounded by the same trees the scanner itself walks.
pub fn source_fingerprint(root: &Path, target_version: Option<&str>) -> Result<String> {
    // Catalog sources fingerprint by the SEMANTIC SOURCE contract
    // (`catalog.en.json` + `catalog.meta.json` only — SF-09: target
    // catalogs are exports, not source; drift = the source changed). A
    // catalog whose marker is present but broken is a typed refusal: the
    // session renders that as "drift unknown", never a false in-sync. See
    // `crate::ui_catalog` (self-localization wave B4).
    match crate::ui_catalog::recognize_catalog(root) {
        crate::ui_catalog::CatalogStatus::NotCatalog => {} // ordinary mod tree
        crate::ui_catalog::CatalogStatus::Valid(_) => {
            return crate::ui_catalog::source_fingerprint(root);
        }
        crate::ui_catalog::CatalogStatus::Invalid(reason) => {
            return Err(color_eyre::eyre::eyre!(
                "UI catalog source at `{}` is invalid: {reason}",
                root.display()
            ));
        }
    }
    let view = crate::modview::effective_view(root, target_version)?;
    let mut acc = String::new();
    acc.push_str(&format!(
        "version={}\n",
        view.version.as_deref().unwrap_or("-")
    ));
    // The LoadFolders manifest itself defines the view - hash it too, so a
    // rollback that only rewrites the manifest is still drift.
    if let Ok(bytes) = std::fs::read(root.join("LoadFolders.xml")) {
        acc.push_str("LoadFolders.xml\n");
        acc.push_str(&crate::observability::sha256_hex(&bytes));
        acc.push('\n');
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.extend(view.languages_dirs());
    dirs.extend(view.defs_roots());
    for dir in view.content_dirs.iter().chain(view.conditional_dirs.iter()) {
        dirs.push(dir.join("Patches"));
    }
    dirs.push(root.join("Patches"));
    dirs.sort();
    dirs.dedup();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir).sort_by_file_name() {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            acc.push_str(&rel);
            acc.push('\n');
            let bytes = std::fs::read(entry.path())?;
            acc.push_str(&crate::observability::sha256_hex(&bytes));
            acc.push('\n');
        }
    }
    Ok(crate::observability::sha256_hex(acc.as_bytes()))
}

#[cfg(test)]
mod source_fingerprint_tests {
    //! M3 regression: the fingerprint is stable for unchanged source and
    //! moves on every change class M3 is about — content edit, added/
    //! removed file, LoadFolders version rollback.
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    /// Versioned LoadFolders mod with per-version content trees.
    fn loadfolders_mod(root: &Path) {
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>/</li><li>Common16</li></v1.6>\
             <v1.5><li>/</li><li>Common15</li></v1.5></loadFolders>",
        );
        write(
            &root.join("Defs/A.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>v16 label</label></ThingDef></Defs>"#,
        );
        write(
            &root.join("Common16/Languages/Russian/Keyed/K.xml"),
            "<LanguageData><K1>шестнадцать</K1></LanguageData>",
        );
        write(
            &root.join("Common15/Languages/Russian/Keyed/K.xml"),
            "<LanguageData><K1>пятнадцать</K1></LanguageData>",
        );
    }

    /// Same content + same requested version → same value, across calls.
    #[test]
    fn deterministic_for_unchanged_source() {
        let dir = tempdir().unwrap();
        loadfolders_mod(dir.path());
        let a = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        let b = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        assert_eq!(a, b);
    }

    /// Edit / add / remove inside the scanned trees each move the value;
    /// reverting restores it.
    #[test]
    fn content_and_file_set_changes_move_the_value() {
        let dir = tempdir().unwrap();
        loadfolders_mod(dir.path());
        let base = source_fingerprint(dir.path(), Some("1.6")).unwrap();

        // Content edit.
        write(
            &dir.path().join("Common16/Languages/Russian/Keyed/K.xml"),
            "<LanguageData><K1>изменено</K1></LanguageData>",
        );
        let edited = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        assert_ne!(base, edited);

        // Revert, then ADD a file (the harness "sourcedrift" T99 shape).
        write(
            &dir.path().join("Common16/Languages/Russian/Keyed/K.xml"),
            "<LanguageData><K1>шестнадцать</K1></LanguageData>",
        );
        write(
            &dir.path()
                .join("Common16/Languages/Russian/Keyed/Extra.xml"),
            "<LanguageData><K2>добавлен</K2></LanguageData>",
        );
        let added = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        assert_ne!(base, added);
        assert_ne!(edited, added);

        // Remove it again → back in sync.
        fs::remove_file(
            dir.path()
                .join("Common16/Languages/Russian/Keyed/Extra.xml"),
        )
        .unwrap();
        let reverted = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        assert_eq!(reverted, base);
    }

    /// LoadFolders rollback (harness "cases"): 1.6 → 1.5-only manifest
    /// changes BOTH the resolved version and the active trees; even a
    /// manifest rewrite that keeps the view is drift (the manifest is
    /// hashed too).
    #[test]
    fn version_rollback_moves_the_value() {
        let dir = tempdir().unwrap();
        loadfolders_mod(dir.path());
        let v16 = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        // Unrequested version → the LATEST tag (1.6 here) — the same view
        // as an explicit 1.6, hence the same value.
        assert_eq!(v16, source_fingerprint(dir.path(), None).unwrap());
        assert_ne!(v16, source_fingerprint(dir.path(), Some("1.5")).unwrap());

        // Roll the manifest back to 1.5-only: a session fingerprinted at
        // 1.6 now resolves 1.5 via the game fallback → drift.
        write(
            &dir.path().join("LoadFolders.xml"),
            "<loadFolders><v1.5><li>/</li><li>Common15</li></v1.5></loadFolders>",
        );
        let rolled = source_fingerprint(dir.path(), Some("1.6")).unwrap();
        assert_ne!(v16, rolled);
    }
}

#[cfg(test)]
mod loadfolders_ru_only_tests {
    //! Corpus G3b regression (real forms: 2927850179 / 2126925929). A
    //! Russian-only translation mod under a versioned LoadFolders layout
    //! must RESOLVE fully (Languages of the root, plain `li` subpackages and
    //! IfModActive subpackages all scanned) — the empty canonical inventory
    //! then comes from the EN-source contract, not from a lost scan.
    use super::*;
    use rimloc_domain::canonical::ViewLabel;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &std::path::Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    /// 2126925929 shape: plain `li` subpackage (Common) + IfModActive one.
    #[test]
    fn ru_only_named_subpackages_resolve_before_guard() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>Common</li>             <li IfModActive=\"VanillaExpanded.VWEL\">Laser</li></v1.6></loadFolders>",
        );
        write(
            &root.join("Common/Languages/Russian/DefInjected/ThingDef/W.xml"),
            "<LanguageData>\n  <Gun.label>винтовка</Gun.label>\n</LanguageData>\n",
        );
        write(
            &root.join("Laser/Languages/Russian/Keyed/K.xml"),
            "<LanguageData>\n  <LaserKey>лазер</LaserKey>\n</LanguageData>\n",
        );
        let auto = autodiscover_defs_context(root).unwrap();
        let scan =
            scan_units_effective_full(root, Some("1.6"), &auto.dict, &auto.extra_fields).unwrap();
        // The resolver activated BOTH subpackages: plain and conditional.
        assert!(scan
            .units
            .iter()
            .any(|u| u.key == "Gun.label" && !u.conditional));
        assert!(scan
            .units
            .iter()
            .any(|u| u.key == "LaserKey" && u.conditional));
        assert_eq!(scan.view.as_ref().unwrap().version.as_deref(), Some("1.6"));
        // The canonical project honestly has zero EN-source entries: every
        // scanned unit is target-side (Russian), nothing was lost silently
        // by a resolver failure.
        let p = crate::project::build_project(root, Some("1.6")).unwrap();
        assert!(p.entries.is_empty());
        assert_eq!(p.context.view, ViewLabel::Potential);
    }

    /// 2927850179 shape: `li>/` root + IfModActive subpackages that carry
    /// Languages/Russian inside; the root Languages has no English either.
    #[test]
    fn ru_only_root_plus_conditional_subpackages_resolve() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>/</li>             <li IfModActive=\"VanillaExpanded.VFECore\">1.6/Main</li></v1.6></loadFolders>",
        );
        write(
            &root.join("1.6/Main/Languages/Russian/Keyed/K.xml"),
            "<LanguageData>\n  <MainKey>основной</MainKey>\n</LanguageData>\n",
        );
        write(
            &root.join("Languages/Russian/DefInjected/ThingDef/R.xml"),
            "<LanguageData>\n  <Root.label>корень</Root.label>\n</LanguageData>\n",
        );
        let auto = autodiscover_defs_context(root).unwrap();
        let scan =
            scan_units_effective_full(root, Some("1.6"), &auto.dict, &auto.extra_fields).unwrap();
        assert!(scan.units.iter().any(|u| u.key == "MainKey"));
        assert!(scan.units.iter().any(|u| u.key == "Root.label"));
        assert_eq!(scan.view.as_ref().unwrap().version.as_deref(), Some("1.6"));
        let p = crate::project::build_project(root, Some("1.6")).unwrap();
        assert!(p.entries.is_empty());
    }
}

#[cfg(test)]
mod gate_h_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// H1: two active Defs files with the same def identity but different
    /// text — RimWorld registers the first file and rejects the duplicate
    /// defName (`Adding duplicate` + GetDefSilentFail first registration),
    /// so the FIRST file's value is the effective source.
    #[test]
    fn def_duplicate_identity_first_file_wins() {
        let dir = tempdir().unwrap();
        let defs = dir.path().join("Defs");
        fs::create_dir_all(&defs).unwrap();
        fs::write(
            defs.join("A_First.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>from A</label></ThingDef></Defs>"#,
        )
        .unwrap();
        fs::write(
            defs.join("B_Second.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>from B</label></ThingDef></Defs>"#,
        )
        .unwrap();
        let auto = autodiscover_defs_context(dir.path()).unwrap();
        let units = scan_units_effective(dir.path(), None, &auto.dict, &auto.extra_fields).unwrap();
        let dup: Vec<_> = units.iter().filter(|u| u.key == "Dup.label").collect();
        assert_eq!(dup.len(), 1, "{units:?}");
        assert_eq!(dup[0].source.as_deref(), Some("from A"));
    }

    /// H2: Keyed duplicates. Cross-file → LAST loaded wins (SetOrAdd);
    /// within one file → the game errors and takes the FIRST value (and the
    /// duplicate is kept so validation still reports it).
    #[test]
    fn keyed_precedence_last_file_wins_in_file_first() {
        let dir = tempdir().unwrap();
        let en = dir.path().join("Languages/English/Keyed");
        fs::create_dir_all(&en).unwrap();
        fs::write(
            en.join("A_First.xml"),
            r#"<LanguageData><Greeting>first-file</Greeting></LanguageData>"#,
        )
        .unwrap();
        fs::write(
            en.join("Z_Last.xml"),
            r#"<LanguageData><Greeting>last-file</Greeting></LanguageData>"#,
        )
        .unwrap();
        fs::write(
            en.join("Dup.xml"),
            r#"<LanguageData><DupKey>one</DupKey><DupKey>two</DupKey></LanguageData>"#,
        )
        .unwrap();
        let auto = autodiscover_defs_context(dir.path()).unwrap();
        let units = scan_units_effective(dir.path(), None, &auto.dict, &auto.extra_fields).unwrap();
        let find = |k: &str| units.iter().find(|u| u.key == k);
        assert_eq!(
            find("Greeting").unwrap().source.as_deref(),
            Some("last-file"),
            "{units:?}"
        );
        let dup: Vec<_> = units.iter().filter(|u| u.key == "DupKey").collect();
        assert_eq!(dup.len(), 2, "{units:?}");
        assert_eq!(dup[0].source.as_deref(), Some("one"));
    }
}
