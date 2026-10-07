use crate::version::resolve_game_version_root;
use rimloc_services::scan::scan_patches_as_units;
use std::collections::{BTreeSet, HashMap};
use std::io::IsTerminal;

/// Loud `--game-version` validation for classic (LoadFolders-less) mods:
/// exact version folder → ok; the game fallback (largest ≤ requested) → ok
/// with a warning; nothing at or below the request → refused. Flat mods fall
/// through to `resolve_game_version_root` (About/supportedVersions check,
/// same refusal as before the effective-view rework).
fn validate_requested_game_version(
    root: &std::path::Path,
    requested: &str,
) -> color_eyre::Result<()> {
    let want = {
        let r = requested.trim();
        r.strip_prefix('v')
            .or_else(|| r.strip_prefix('V'))
            .unwrap_or(r)
            .to_lowercase()
    };
    let want_parts: Vec<u64> = want.split('.').filter_map(|p| p.parse().ok()).collect();
    let tiers = rimloc_services::classic_version_dirs(root);
    if tiers.is_empty() {
        // Flat mod: the historical loud path (declared-versions check).
        resolve_game_version_root(root, Some(requested))?;
        return Ok(());
    }
    if tiers.iter().any(|(v, _)| *v == want) {
        return Ok(());
    }
    let fallback = tiers
        .iter()
        .filter(|(v, _)| {
            let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
            parts <= want_parts
        })
        .max_by_key(|(v, _)| {
            v.split('.')
                .filter_map(|p| p.parse::<u64>().ok())
                .collect::<Vec<_>>()
        });
    match fallback {
        Some((v, _)) => {
            tracing::warn!(
                event = "scan_version_fallback",
                requested = requested,
                resolved = v
            );
            Ok(())
        }
        None => {
            let available: Vec<&str> = tiers.iter().map(|(v, _)| v.as_str()).collect();
            color_eyre::eyre::bail!(
                "Requested version '{}' not found under {} (available: {})",
                requested,
                root.display(),
                available.join(", ")
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn run_scan(
    root: std::path::PathBuf,
    out_csv: Option<std::path::PathBuf>,
    out_json: Option<std::path::PathBuf>,
    lang: Option<String>,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    use_en_comments: Option<String>,
    defs_dir: Option<std::path::PathBuf>,
    defs_field: Vec<String>,
    defs_dict: Vec<std::path::PathBuf>,
    defs_type_schema: Option<std::path::PathBuf>,
    format: String,
    game_version: Option<String>,
    active_mods: Vec<String>,
    include_all_versions: bool,
    keyed_nested: bool,
    parallel: bool,
    no_inherit: bool,
    with_plugins: bool,
    with_patches: bool,
    patch_min_len: Option<usize>,
    patch_strict_xpath: bool,
    fuzzy: bool,
) -> color_eyre::Result<()> {
    // RimTransAI parity: --active-mods is the active-mod context resolving
    // LoadFolders IfModActive* branches. No flag → no guessing: conditional
    // content stays out of the scan and the view is reported as POTENTIAL.
    let active_mod_ctx = if active_mods.iter().all(|s| s.trim().is_empty()) {
        None
    } else {
        Some(rimloc_services::ActiveModContext::from_package_ids(
            active_mods,
        ))
    };
    tracing::debug!(
        event = "scan_args",
        root = ?root,
        out_csv = ?out_csv,
        out_json = ?out_json,
        lang = ?lang,
        format = %format,
        game_version = ?game_version,
        active_mods = ?active_mod_ctx.as_ref().map(|c| &c.active_package_ids),
        include_all_versions = include_all_versions
    );

    // Gate H: LoadFolders mods stay rooted at the mod root — the effective
    // view picks version content dirs; re-rooting to root/1.6 would lose the
    // Common content (root Languages etc.).
    // M1: a typo in the path must be a loud refusal, never an empty
    // "[]" report over nothing.
    if !root.is_dir() {
        color_eyre::eyre::bail!(
            "mod root `{}` does not exist or is not a directory; nothing to scan",
            root.display()
        );
    }
    let is_loadfolders_mod = root.join("LoadFolders.xml").is_file();
    // M1 continuity (wave-5 MUST_FIX): the effective-view pipeline resolves
    // versions silently, but a typo'd `--game-version` must stay a LOUD
    // refusal, never a quiet wrong-view scan. Classic mods validate against
    // their plain version folders: exact match passes, the game fallback
    // (largest ≤ requested) passes with a warning, anything else is an
    // error. Flat mods keep the About/supportedVersions check.
    if let Some(gv) = game_version.as_deref() {
        if !is_loadfolders_mod {
            validate_requested_game_version(&root, gv)?;
        }
    }
    let defs_abs = defs_dir.as_ref().map(|p| {
        if p.is_absolute() {
            p.clone()
        } else {
            root.join(p)
        }
    });
    // A user-supplied --defs-dir keeps the manual single-root pipeline:
    // the override IS the explicit scope, per-key version resolution does
    // not apply to it (LoadFolders mods already ignore the flag in the
    // effective-view pipeline — documented offline-superset policy).
    let legacy_defs_override = defs_abs.is_some() && !is_loadfolders_mod;
    let (scan_root, selected_version) = if legacy_defs_override && !include_all_versions {
        resolve_game_version_root(&root, game_version.as_deref())?
    } else {
        (root.clone(), None)
    };
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "scan_version_resolved", version = ver, path = %scan_root.display());
    }
    let auto = rimloc_services::autodiscover_defs_context(&scan_root)?;
    let cfg = rimloc_config::load_config().unwrap_or_default();

    let mut extra_fields: Vec<String> = auto.extra_fields.clone();
    if let Some(ref scan_cfg) = cfg.scan {
        if let Some(extra) = scan_cfg.defs_fields.clone() {
            extra_fields.extend(extra);
        }
    }
    extra_fields.extend(defs_field);
    extra_fields.sort();
    extra_fields.dedup();

    let mut dict_sets: HashMap<String, BTreeSet<String>> = auto
        .dict
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();

    let mut merge_dict = |map: HashMap<String, Vec<String>>| {
        for (k, v) in map {
            dict_sets.entry(k).or_default().extend(v);
        }
    };

    if let Some(scan_cfg) = cfg.scan.as_ref() {
        if let Some(paths) = scan_cfg.defs_dicts.as_ref() {
            for p in paths {
                let pp = if p.starts_with('/') || p.contains(':') {
                    std::path::PathBuf::from(p)
                } else {
                    scan_root.join(p)
                };
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
                    merge_dict(d.0);
                }
            }
        }
    }
    for p in &defs_dict {
        let pp = if p.is_absolute() {
            p.clone()
        } else {
            scan_root.join(p)
        };
        if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
            merge_dict(d.0);
        }
    }
    if let Some(schema) = defs_type_schema.as_ref() {
        let pp = if schema.is_absolute() {
            schema.clone()
        } else {
            scan_root.join(schema)
        };
        if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) {
            merge_dict(d.0);
        }
    }
    let merged: HashMap<String, Vec<String>> = dict_sets
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();

    // Gate inheritance via env var for parser crate
    // Apply ENV gates from CLI flags or config defaults
    if no_inherit
        || cfg
            .scan
            .as_ref()
            .and_then(|s| s.no_inherit)
            .unwrap_or(false)
    {
        std::env::set_var("RIMLOC_INHERIT", "0");
    }
    if cfg
        .scan
        .as_ref()
        .and_then(|s| s.keyed_nested)
        .unwrap_or(false)
    {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
    }
    // Gate nested keyed dotted keys via env var
    if keyed_nested {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
    }
    if parallel || cfg.scan.as_ref().and_then(|s| s.parallel).unwrap_or(false) {
        std::env::set_var("RIMLOC_PARALLEL", "1");
    }
    if keyed_nested {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
    }
    if fuzzy {
        std::env::set_var("RIMLOC_FUZZY", "1");
    }

    // Gate H: a mod is scanned as its EFFECTIVE view for the requested
    // version — version-scoped Defs roots, never the cross-version union —
    // for BOTH layouts: LoadFolders.xml tags and plain version folders
    // (game rule: the newest folder ≤ the version, plus Common, plus the
    // root; GAME_SOURCE_FINDINGS §1.3). Wave-5 MUST_FIX: the legacy
    // whole-root union on plain version folders let a 1.3 value win per key
    // where the disk ships 1.6 (`--include-all-versions` keeps the union
    // coverage but now resolves every key to its NEWEST defining version).
    // The FULL pipeline variants: the resolved view rides along so the
    // conditional-content warning below is grounded in what the resolver
    // actually did (typed state, not a LoadFolders.xml sniff).
    let effective_scan = if include_all_versions && !is_loadfolders_mod && !legacy_defs_override {
        rimloc_services::scan::scan_units_all_versions_full(
            &scan_root,
            game_version.as_deref(),
            &merged,
            &extra_fields,
            active_mod_ctx.as_ref(),
        )?
    } else if !legacy_defs_override && !include_all_versions {
        rimloc_services::scan::scan_units_effective_full(
            &scan_root,
            game_version.as_deref(),
            &merged,
            &extra_fields,
            active_mod_ctx.as_ref(),
        )?
    } else {
        let units = rimloc_services::scan_units_with_defs_and_dict(
            &scan_root,
            defs_abs.as_deref(),
            &merged,
            &extra_fields,
        )?;
        rimloc_services::scan::EffectiveScan {
            units,
            patch: Default::default(),
            view: None,
        }
    };
    let mut units = effective_scan.units;
    // Typed diagnostic (do NOT guess): with IfModActive content present but
    // no --active-mods, the scan is an honest partial — say so, loudly.
    if let Some(view) = effective_scan.view.as_ref() {
        if let Some(entries) = view
            .conditional_state
            .is_unresolved()
            .then_some(&view.unresolved_conditionals)
        {
            if !entries.is_empty() {
                let conditions: Vec<String> = entries
                    .iter()
                    .map(|e| e.display_condition())
                    .collect::<Vec<_>>();
                tracing::warn!(
                    event = "ifmodactive_unresolved",
                    skipped = entries.len(),
                    conditions = %conditions.join(", "),
                    "conditional LoadFolders content was NOT scanned: no --active-mods given; \
                     pass --active-mods <packageId,...> (e.g. Ludeon.RimWorld.Royalty) for the resolved view"
                );
                ui_warn!(
                    "scan-ifmodactive-potential",
                    count = entries.len(),
                    conditions = conditions.join(", ")
                );
            }
        }
    }

    // Optionally augment with plugin-derived units (e.g., XmlExtensions Settings/TKey)
    if with_plugins {
        rimloc_services::plugins::init_builtin();
        for p in rimloc_services::plugins::iter() {
            if let Ok(mut extra) = p.scan_units(&scan_root) {
                units.append(&mut extra);
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

    if with_patches {
        let min_len = patch_min_len.unwrap_or(1);
        // Gate H: patch units follow the same effective version view as Defs —
        // never the cross-version union (1.4 patches must not leak into a
        // 1.5 scan of a LoadFolders mod).
        let patch_root = if include_all_versions {
            scan_root.clone()
        } else {
            resolve_game_version_root(&scan_root, game_version.as_deref())
                .map(|(p, _)| p)
                .unwrap_or_else(|_| scan_root.clone())
        };
        if let Ok(mut extra) = scan_patches_as_units(&patch_root, min_len, patch_strict_xpath) {
            units.append(&mut extra);
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

    // Plugin execution is integrated in services::scan_units via registry. Future: expose dynamic loaders here.

    fn is_source_for_lang_dir(path: &std::path::Path, lang_dir: &str) -> bool {
        // Languages/<dir>
        let mut comps = path.components();
        while let Some(c) = comps.next() {
            let s = c.as_os_str().to_string_lossy();
            if s.eq_ignore_ascii_case("Languages") {
                if let Some(lang) = comps.next() {
                    let lang_s = lang.as_os_str().to_string_lossy();
                    if lang_s == lang_dir {
                        return true;
                    }
                }
                break;
            }
        }
        // English also includes Defs/*
        if lang_dir.eq_ignore_ascii_case("English") {
            let s = path.to_string_lossy();
            if rimloc_core::path_text::has_path_marker(&s, "Defs") {
                return true;
            }
        }
        false
    }

    // Apply optional EN comments override before filtering/sorting
    if let Some(prefix) = use_en_comments.as_deref() {
        let src_dir = source_lang_dir
            .clone()
            .or_else(|| {
                source_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "English".to_string());
        let _ =
            rimloc_services::scan::override_keyed_units_from_comments(&mut units, &src_dir, prefix);
    }

    let units = if let Some(dir) = source_lang_dir.clone() {
        let dir = rimloc_services::normalize_lang_dir(&dir);
        let before = units.len();
        let mut filtered: Vec<_> = units
            .into_iter()
            .filter(|u| is_source_for_lang_dir(&u.path, &dir))
            .collect();
        filtered.sort_by(|a, b| {
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
        tracing::info!(event = "scan_filtered_by_dir", before = before, after = filtered.len(), source_lang_dir = %dir);
        filtered
    } else if let Some(code) = source_lang.clone() {
        let dir = rimloc_import_po::rimworld_lang_dir(&code);
        let before = units.len();
        let mut filtered: Vec<_> = units
            .into_iter()
            .filter(|u| is_source_for_lang_dir(&u.path, &dir))
            .collect();
        filtered.sort_by(|a, b| {
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
        tracing::info!(event = "scan_filtered_by_code", source_lang = %code, source_dir = %dir, before = before, after = filtered.len());
        filtered
    } else if let Some(dir) = lang.as_deref().map(rimloc_import_po::rimworld_lang_dir) {
        // `--lang` filters the scan to one Languages/<dir> (English additionally
        // includes Defs/* as the translation source). Without it, every language
        // folder is collected and keys collide across translations.
        let before = units.len();
        let mut filtered: Vec<_> = units
            .into_iter()
            .filter(|u| is_source_for_lang_dir(&u.path, &dir))
            .collect();
        filtered.sort_by(|a, b| {
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
        tracing::info!(event = "scan_filtered_by_lang", lang = %dir, before = before, after = filtered.len());
        filtered
    } else {
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
        units
    };

    match format.as_str() {
        "csv" => {
            if out_json.is_some() {
                return Err(color_eyre::eyre::eyre!(
                    "--out-json is only supported when --format json"
                ));
            }
            if let Some(path) = out_csv {
                let file = std::fs::File::create(&path)?;
                rimloc_export_csv::write_csv(file, &units, lang.as_deref())?;
                ui_info!("scan-csv-saved", path = path.display().to_string());
            } else {
                if std::io::stdout().is_terminal() {
                    ui_info!("scan-csv-stdout");
                }
                let stdout = std::io::stdout();
                let lock = stdout.lock();
                rimloc_export_csv::write_csv(lock, &units, lang.as_deref())?;
            }
        }
        "json" => {
            #[derive(serde::Serialize)]
            struct JsonUnit<'a> {
                schema_version: u32,
                path: String,
                line: Option<usize>,
                key: &'a str,
                value: Option<&'a str>,
                #[serde(skip_serializing_if = "Option::is_none")]
                tkey: Option<&'a rimloc_core::TKeyMeta>,
            }
            let items: Vec<JsonUnit<'_>> = units
                .iter()
                .map(|u| JsonUnit {
                    schema_version: crate::OUTPUT_SCHEMA_VERSION,
                    path: u.path.display().to_string(),
                    line: u.line,
                    key: u.key.as_str(),
                    value: u.source.as_deref(),
                    tkey: u.tkey.as_ref(),
                })
                .collect();

            if let Some(path) = out_json {
                let file = std::fs::File::create(&path)?;
                serde_json::to_writer_pretty(file, &items)?;
                ui_info!("scan-json-saved", path = path.display().to_string());
            } else {
                if std::io::stdout().is_terminal() {
                    ui_info!("scan-json-stdout");
                }
                serde_json::to_writer(std::io::stdout().lock(), &items)?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
