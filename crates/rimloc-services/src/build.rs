use crate::Result;
use rimloc_core::path_text::has_path_marker;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Group the `Languages/<lang_folder>` tree of a mod root for `build-mod
/// --from-root`. Wave-5 MUST_FIX parity with scan: without an explicit
/// version filter each key resolves to its NEWEST defining version — the
/// plain version folders are tiers in the game's load priority (newest
/// folder first, then Common, then the root; GAME_SOURCE_FINDINGS §1.4) and
/// a key claimed by a higher tier is never repeated by a lower one. The
/// previous whole-root grouping let the same key enter the output twice with
/// a scan-order-dependent winner. `--from-game-version` keeps its strict
/// filter semantics (only paths carrying one of the markers) — explicit
/// selection wins over inference.
#[allow(clippy::too_many_arguments)]
fn group_root_language_items(
    from_root: &Path,
    lang_folder: &str,
    versions: Option<&[String]>,
    skip_empty: bool,
) -> Result<BTreeMap<PathBuf, Vec<(String, String)>>> {
    use std::collections::HashSet;

    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](.+)$").unwrap();
    let lang_marker_slash = format!("Languages/{lang_folder}");
    let lang_marker_back = format!("Languages\\{lang_folder}");

    let family_of = |p: &Path| -> &'static str {
        let s = p.to_string_lossy();
        if has_path_marker(&s, "DefInjected") {
            "definj"
        } else if has_path_marker(&s, "Keyed") {
            "keyed"
        } else {
            "other"
        }
    };

    // Priority tiers when no explicit filter is given: newest version folder
    // first, then Common, then the root.
    let mut tiers: Vec<PathBuf> = crate::modview::classic_version_dirs(from_root)
        .into_iter()
        .map(|(_, dir)| dir)
        .collect();
    tiers.reverse();
    let common = from_root.join("Common");
    if common.is_dir() {
        tiers.push(common);
    }
    tiers.push(from_root.to_path_buf());
    let tier_of = |p: &Path| -> usize {
        tiers
            .iter()
            .position(|t| p.starts_with(t))
            .unwrap_or_else(|| tiers.len() - 1)
    };

    let units = rimloc_parsers_xml::scan_keyed_xml(from_root)?;
    let mut claimed: HashSet<(&'static str, String)> = HashSet::new();
    let mut tier_claimed: HashSet<(&'static str, String)> = HashSet::new();
    let mut current_tier: Option<usize> = None;
    let mut grouped: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();

    // Deterministic input order: (tier priority) then (path, key). Within a
    // tier the order only affects duplicate retention, never the winner.
    let mut filtered: Vec<rimloc_core::TransUnit> = units
        .into_iter()
        .filter(|u| {
            let path_str = u.path.to_string_lossy();
            if !has_path_marker(&path_str, "Languages") {
                return false;
            }
            if !(has_path_marker(&path_str, &lang_marker_slash)
                || has_path_marker(&path_str, &lang_marker_back))
            {
                return false;
            }
            if let Some(vers) = versions {
                let mut matched = false;
                for ver in vers {
                    if has_path_marker(&path_str, ver)
                        || has_path_marker(&path_str, &format!("v{ver}"))
                    {
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return false;
                }
            }
            true
        })
        .collect();
    filtered.sort_by(|a, b| {
        let ta = if versions.is_some() {
            0
        } else {
            tier_of(&a.path)
        };
        let tb = if versions.is_some() {
            0
        } else {
            tier_of(&b.path)
        };
        (ta, a.path.to_string_lossy(), a.key.as_str()).cmp(&(
            tb,
            b.path.to_string_lossy(),
            b.key.as_str(),
        ))
    });

    for u in filtered {
        let Some(src) = u
            .source
            .as_deref()
            .filter(|s| !skip_empty || !s.trim().is_empty())
        else {
            continue;
        };
        let lossy = u.path.to_string_lossy();
        let Some(caps) = re.captures(&lossy) else {
            continue;
        };
        let rel = PathBuf::from(&caps[1]);
        // Cross-tier claiming: a key owned by a higher tier never repeats in
        // a lower one. Within one tier duplicates pass (the caller's
        // `--dedupe` handles in-file duplicates; behaviour is unchanged).
        if versions.is_none() {
            let tier = tier_of(&u.path);
            if current_tier != Some(tier) {
                claimed.extend(tier_claimed.drain());
                current_tier = Some(tier);
            }
            let identity = (family_of(&u.path), u.key.clone());
            if claimed.contains(&identity) {
                continue;
            }
            tier_claimed.insert(identity);
        }
        grouped
            .entry(rel)
            .or_default()
            .push((u.key, src.to_string()));
    }
    Ok(grouped)
}

/// Build translation mod from an existing Languages/<lang> tree under `from_root`.
/// Returns a list of files to write with number of keys; optionally writes when `write=true`.
/// `skip_empty` drops untranslated (empty source) keys instead of writing
/// empty `<Key></Key>` elements (wave-5 MUST_FIX №2; default keeps the
/// previous output).
#[allow(clippy::too_many_arguments)]
pub fn build_from_root(
    from_root: &Path,
    out_mod: &Path,
    lang_folder: &str,
    versions: Option<&[String]>,
    write: bool,
    dedupe: bool,
    skip_empty: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize)> {
    build_from_root_with_progress(
        from_root,
        out_mod,
        lang_folder,
        versions,
        write,
        dedupe,
        skip_empty,
        |_idx, _total, _path| {},
    )
}

/// Progress variant of [`build_from_root`]; `skip_empty` drops untranslated
/// (empty source) keys instead of writing empty `<Key></Key>` elements
/// (wave-5 MUST_FIX №2; the default keeps the previous output).
#[allow(clippy::too_many_arguments)]
pub fn build_from_root_with_progress(
    from_root: &Path,
    out_mod: &Path,
    lang_folder: &str,
    versions: Option<&[String]>,
    write: bool,
    dedupe: bool,
    skip_empty: bool,
    mut progress: impl FnMut(usize, usize, &Path),
) -> Result<(Vec<(PathBuf, usize)>, usize)> {
    // H1: strict form + containment before anything is planned or written.
    crate::util::ensure_lang_write_target(out_mod, lang_folder)?;
    use std::collections::HashSet;

    let grouped = group_root_language_items(from_root, lang_folder, versions, skip_empty)?;

    let total_files = grouped.len();
    let mut idx = 0usize;
    let mut files: Vec<(PathBuf, usize)> = Vec::new();
    for (rel, mut items) in grouped {
        if dedupe {
            let mut seen: HashSet<String> = HashSet::new();
            let mut outv: Vec<(String, String)> = Vec::new();
            for (k, v) in items.into_iter().rev() {
                if seen.insert(k.clone()) {
                    outv.push((k, v));
                }
            }
            outv.reverse();
            items = outv;
        }
        let full = out_mod.join("Languages").join(lang_folder).join(&rel);
        if write {
            rimloc_import_po::write_language_data_xml(&full, &items)?;
        }
        files.push((full.clone(), items.len()));
        idx += 1;
        progress(idx, total_files, &full);
    }
    if write {
        let _ = std::fs::create_dir_all(out_mod.join("About"));
    }
    let total_keys = files.iter().map(|(_, n)| *n).sum();
    Ok((files, total_keys))
}

/// Build a translation mod from a PO file (wrappers around importer crate).
pub struct BuildPlan {
    pub mod_name: String,
    pub package_id: String,
    pub rw_version: String,
    pub out_mod: PathBuf,
    pub lang_dir: String,
    pub files: Vec<(PathBuf, usize)>,
    pub total_keys: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn build_from_po_dry_run(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
    skip_empty: bool,
) -> Result<BuildPlan> {
    // H1: strict form + containment before anything is planned or written.
    crate::util::ensure_lang_write_target(out_mod, lang_folder)?;
    let plan = rimloc_import_po::build_translation_mod_dry_run_opts(
        po,
        out_mod,
        lang_folder,
        name,
        package_id,
        rw_version,
        dedupe,
        skip_empty,
    )?;
    Ok(BuildPlan {
        mod_name: plan.mod_name,
        package_id: plan.package_id,
        rw_version: plan.rw_version,
        out_mod: plan.out_mod,
        lang_dir: plan.lang_dir,
        files: plan.files,
        total_keys: plan.total_keys,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn build_from_po_execute(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
    skip_empty: bool,
) -> Result<()> {
    // H1: strict form + containment before anything is planned or written.
    crate::util::ensure_lang_write_target(out_mod, lang_folder)?;
    rimloc_import_po::build_translation_mod_with_langdir_opts(
        po,
        out_mod,
        lang_folder,
        name,
        package_id,
        rw_version,
        dedupe,
        skip_empty,
    )
}

/// Build from PO with per-file progress events
#[allow(clippy::too_many_arguments)]
pub fn build_from_po_with_progress(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
    skip_empty: bool,
    mut progress: impl FnMut(usize, usize, &Path),
) -> Result<()> {
    // H1: strict form + containment before anything is planned or written.
    crate::util::ensure_lang_write_target(out_mod, lang_folder)?;
    // Read entries and group by relative path under Languages/
    let mut entries = rimloc_import_po::read_po_entries(po)?;
    if skip_empty {
        entries.retain(|e| !e.value.trim().is_empty());
    }
    let re =
        regex::Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](?P<rel>.+?)(?::\d+)?$").unwrap();
    use std::collections::{BTreeMap, HashSet};
    let mut grouped: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();
    for e in entries {
        let rel = e
            .reference
            .as_ref()
            .and_then(|r| re.captures(r))
            .and_then(|c| c.name("rel").map(|m| PathBuf::from(m.as_str())))
            .unwrap_or_else(|| PathBuf::from("Keyed/_Imported.xml"));
        grouped.entry(rel).or_default().push((e.key, e.value));
    }

    // Ensure About (write minimal About.xml)
    let about_dir = out_mod.join("About");
    std::fs::create_dir_all(&about_dir)?;
    let about_xml = about_dir.join("About.xml");
    let _ = std::fs::write(
        &about_xml,
        format!(
            "<ModMetaData>\n  <packageId>{}</packageId>\n  <name>{}</name>\n  <description>Translation mod (generated by RimLoc)</description>\n  <supportedVersions>\n    <li>{}</li>\n  </supportedVersions>\n</ModMetaData>\n",
            package_id, name, rw_version
        ),
    );

    let total = grouped.len();
    let mut idx = 0usize;
    for (rel, mut items) in grouped {
        if dedupe {
            let mut seen: HashSet<String> = HashSet::new();
            let mut outv: Vec<(String, String)> = Vec::new();
            for (k, v) in items.into_iter().rev() {
                if seen.insert(k.clone()) {
                    outv.push((k, v));
                }
            }
            outv.reverse();
            items = outv;
        }
        let out_path = out_mod.join("Languages").join(lang_folder).join(&rel);
        rimloc_import_po::write_language_data_xml(&out_path, &items)?;
        idx += 1;
        progress(idx, total, &out_path);
    }
    Ok(())
}

/// Contract `project_build_mod` — the FULL drop-in mod package from the
/// canonical session state (GUI parity with `rimloc build-mod`). The
/// `Languages/<lang>` tree and the per-key accounting reuse the PROVEN
/// export writer ([`crate::project::write_rimworld_translation`], same
/// Keyed/TKey/DefInjected acceptance and `skipped_unknown_type` report);
/// then the manifest is stamped in the CLI build-mod shape
/// ([`crate::project::write_modmetadata_about`], `<ModMetaData>` — what the
/// game loads from a folder dropped into Mods) instead of the export's
/// internal `<RimWorldManifest>`. The H1 guard runs here as on every other
/// build entry point; the session layer adds its own partition before
/// calling in (epoch, locale form, absolute out dir, source/managed deny).
pub fn build_mod_from_project_execute(
    project: &rimloc_domain::canonical::Project,
    out_mod: &Path,
    lang_folder: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> crate::Result<crate::project::WriteReport> {
    // H1: strict form + containment before anything is planned or written.
    crate::util::ensure_lang_write_target(out_mod, lang_folder)?;
    let report = crate::project::write_rimworld_translation(
        project,
        out_mod,
        lang_folder,
        mod_name,
        package_id,
        rw_version,
    )?;
    // Replace the export-internal manifest with the game-loadable shape.
    // The Languages tree above is unaffected; the final package matches the
    // `rimloc build-mod` output byte-shape for About.xml.
    crate::project::write_modmetadata_about(out_mod, mod_name, package_id, rw_version)?;
    Ok(report)
}
