use crate::plugins;
use crate::{util::def_injected_target_path, Result, TransUnit};
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

fn merge_defs_units(
    units: &mut Vec<TransUnit>,
    seen: &mut HashSet<String>,
    scan_root: &Path,
    lang_dir: &str,
    defs_meta: Vec<DefsMetaUnit>,
) {
    // Gate H: duplicate defName across files — the game logs
    // `Adding duplicate <DefType> name: X` and GetDefSilentFail returns the
    // FIRST registration, so the first file's fields are authoritative and a
    // later file's same-identity fields never enter the inventory.
    let mut owned_identities: HashSet<(String, String)> = HashSet::new();
    // Deterministic "first file" = lexicographic path order (the documented
    // stand-in for the game's file-system enumeration on Windows).
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
        if !owned_identities.insert((meta.def_type.clone(), unit.key.clone())) {
            continue;
        }
        let target_path = def_injected_target_path(scan_root, lang_dir, &meta.def_type, &unit.path);
        unit.path = target_path;
        unit.line = None;
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
    if let Ok(mut tkey) = rimloc_parsers_xml::scan_defs_tkey(root, None) {
        for u in tkey.drain(..) {
            let k = seen_key(&u.path, &u.key);
            if seen.insert(k) {
                units.push(u);
            }
        }
    }
    let defs_meta =
        rimloc_parsers_xml::scan_defs_with_dict_meta(root, None, &auto.dict, &auto.extra_fields)?;
    merge_defs_units(
        &mut units,
        &mut seen,
        root,
        DEFAULT_SOURCE_LANG_DIR,
        defs_meta,
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
        if !(path_str.contains("/Languages/") || path_str.contains("\\Languages\\")) {
            continue;
        }
        if !(path_str.contains("/Keyed/") || path_str.contains("\\Keyed\\")) {
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
    let mut units = rimloc_parsers_xml::scan_keyed_xml(root)?;
    let mut seen: HashSet<String> = units.iter().map(|u| seen_key(&u.path, &u.key)).collect();
    let defs_meta =
        rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)?;
    // TKey nodes (QuestScriptDefs/TipSetDefs/...; the TKey system exists
    // since RimWorld 1.1, 2020 — 1.6 is the primary tested corpus):
    // explicit translation keys
    // on def fields, identity `<defName>.<TKey>`.
    if let Ok(mut tkey) = rimloc_parsers_xml::scan_defs_tkey(root, defs_root) {
        for u in tkey.drain(..) {
            let k = seen_key(&u.path, &u.key);
            if seen.insert(k) {
                units.push(u);
            }
        }
    }
    merge_defs_units(
        &mut units,
        &mut seen,
        root,
        DEFAULT_SOURCE_LANG_DIR,
        defs_meta,
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
    apply_effective_precedence(&mut units);
    Ok(units)
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
/// Overridden values are dropped from the authoritative inventory; context
/// preservation as diagnostics lands with the canonical model (Gate I).
/// Paths stay case-exact except the already-documented case-insensitive
/// About/LoadFolders resolution.
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
        p.to_string_lossy().contains("/Keyed/") || p.to_string_lossy().contains("\\Keyed\\")
    };
    let is_definj = |p: &std::path::Path| {
        p.to_string_lossy().contains("/DefInjected/")
            || p.to_string_lossy().contains("\\DefInjected\\")
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
    let mut index: std::collections::HashMap<(String, String), usize> =
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
        let idx_key = (lang, u.key.clone());
        match index.get(&idx_key).copied() {
            None => {
                index.insert(idx_key, effective.len());
                effective.push(u);
            }
            Some(pos) if is_keyed(&effective[pos].path) && !same_file_seen => {
                // Cross-file Keyed duplicate: last loaded wins.
                effective[pos] = u;
            }
            Some(pos) if is_definj(&effective[pos].path) => {
                // DefInjected SetOrAdd: every later occurrence overwrites,
                // including within the same file.
                effective[pos] = u;
            }
            Some(_) => {
                // Keyed duplicate within the SAME file: the game logs
                // `Duplicate keyed translation key` and takes the FIRST
                // value, so the first stays the effective winner — but the
                // duplicate unit is kept so the validator still reports the
                // error (diagnostic preservation beats silent dropping).
                effective.push(u);
            }
        }
    }
    *units = effective;
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
    if !root.join("LoadFolders.xml").is_file() {
        return scan_units_with_defs_and_dict(root, None, dict, extra_fields);
    }
    let view = crate::modview::effective_view(root, requested_version)?;

    fn push_unique(units: &mut Vec<TransUnit>, seen: &mut HashSet<String>, u: TransUnit) {
        let k = seen_key(&u.path, &u.key);
        if seen.insert(k) {
            units.push(u);
        }
    }

    let mut units: Vec<TransUnit> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Languages from the effective dirs only (root + content folders that
    // exist for this version).
    for lang_dir in view.languages_dirs() {
        if let Ok(mut scoped) = rimloc_parsers_xml::scan_keyed_xml(&lang_dir) {
            for u in scoped.drain(..) {
                push_unique(&mut units, &mut seen, u);
            }
        }
    }
    // Defs strictly from the effective roots (version-scoped; IfModActive
    // dirs are included per the documented offline superset policy).
    for defs_root in view.defs_roots() {
        let defs_root = Some(defs_root.as_path());
        let defs_meta =
            rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)?;
        if let Ok(mut tkey) = rimloc_parsers_xml::scan_defs_tkey(root, defs_root) {
            for u in tkey.drain(..) {
                push_unique(&mut units, &mut seen, u);
            }
        }
        merge_defs_units(
            &mut units,
            &mut seen,
            root,
            DEFAULT_SOURCE_LANG_DIR,
            defs_meta,
        );
    }
    apply_effective_precedence(&mut units);
    Ok(units)
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
