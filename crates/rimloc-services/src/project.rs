//! Canonical project workflows (Gate I4): build a project from a mod,
//! apply an existing translation, and write RimWorld output — WITHOUT any
//! PO intermediate. PO import/export stays a separate adapter concern.

use crate::matching::{SourceMatcher, TKeyRegistry};
use crate::Result;
use rimloc_domain::canonical as dom;
use rimloc_domain::canonical::{
    EntryKind, InventoryContext, Origin, PatchStage, Project, SourceEntry, SourceEntryId, ViewLabel,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Build a canonical project snapshot for a mod (Gate I4 shared entry).
///
/// `active_mods` is the optional active-mod context ([`crate::modview::
/// ActiveModContext`], RimTransAI parity): when provided, conditional
/// `IfModActive*` LoadFolders branches resolve against it and satisfied
/// content joins the inventory; when `None`, conditional content is NOT
/// guessed — it stays out of the scan and the view is honestly POTENTIAL.
///
/// Provenance honesty (pre-freeze, Source Inspector mandate):
/// - patch coverage comes from the REAL patch report of the scan pipeline,
///   not from "a Patches dir exists";
/// - `version_selected` is the version actually RESOLVED (LoadFolders tag or
///   version dir), `None` for a flat mod — the requested game version stays
///   in `context.target_version`;
/// - the view is EXACT only when a version is known, no `IfModActive`
///   content is left UNRESOLVED (either absent or resolved against the
///   active-mod context), and patch coverage is not partial; otherwise it is
///   an honest POTENTIAL/CONDITIONAL view;
/// - winner reasons are per entry (stamped by the scan pipeline), so no
///   batch-level `selected_by` label is passed here.
pub fn build_project(
    mod_root: &Path,
    target_version: Option<&str>,
    active_mods: Option<&crate::modview::ActiveModContext>,
) -> Result<Project> {
    // Self-localization (wave B4): a directory carrying the app's own
    // generated UI catalog (`catalog.en.json`, schema "1") is its own SOURCE
    // kind. The catalog adapter produces the inventory through this SAME
    // canonical create-path contract (entries + M3 fingerprint) and NO
    // RimWorld scanner runs on catalog data — see `crate::ui_catalog`.
    // SF-09: recognition is TYPED — a broken catalog (marker present, source
    // unreadable/unparseable/foreign) is a refusal carrying the reason, NOT
    // a silent fall-through into the mod scan: a corrupted source must never
    // look like a legitimately empty mod.
    match crate::ui_catalog::recognize_catalog(mod_root) {
        crate::ui_catalog::CatalogStatus::NotCatalog => {} // ordinary mod pipeline
        crate::ui_catalog::CatalogStatus::Valid(_) => {
            return crate::ui_catalog::build_catalog_project(mod_root);
        }
        crate::ui_catalog::CatalogStatus::Invalid(reason) => {
            return Err(color_eyre::eyre::eyre!(
                "UI catalog source at `{}` is invalid and was not scanned: {reason}",
                mod_root.display()
            ));
        }
    }
    let auto = crate::autodiscover_defs_context(mod_root)?;
    // ONE effective pipeline for every layout: the modview resolver decides
    // between flat, classic version dirs and LoadFolders, so a version-only
    // mod is never scanned as a cross-version union.
    let scan = crate::scan::scan_units_effective_full(
        mod_root,
        target_version,
        &auto.dict,
        &auto.extra_fields,
        active_mods,
    )?;
    let mut units = scan.units;
    // The canonical source inventory is the ENGLISH source: units under
    // Languages/<other> are existing target packs, not source text.
    crate::scan::retain_source_language_units(&mut units);
    let stage = match scan.patch.coverage {
        Some(crate::patches_effect::PatchCoverage::Full) => PatchStage::Applied,
        Some(crate::patches_effect::PatchCoverage::Partial) => PatchStage::Partial,
        _ => PatchStage::None,
    };
    // The view stays POTENTIAL while conditional content exists but was not
    // resolved (no active-mod context). With a context the branches are
    // evaluated — satisfied content is as real as unconditional content.
    let conditional_roots = scan
        .view
        .as_ref()
        .is_some_and(|v| v.conditional_state.is_unresolved());
    // The version the RESOLUTION selected (LoadFolders tag or version dir);
    // for a flat mod nothing was version-selected.
    let resolved_version = scan.view.as_ref().and_then(|v| v.version.clone());
    // Exact view requires a known version, no unresolved conditional roots,
    // and full (or no) patch coverage; otherwise honest POTENTIAL.
    let view = if target_version.is_some() && !conditional_roots && stage != PatchStage::Partial {
        ViewLabel::Exact
    } else {
        ViewLabel::Potential
    };
    let context = InventoryContext {
        target_version: target_version.map(String::from),
        view,
        ..Default::default()
    };
    Ok(crate::canonical_bridge::project_from_inventory(
        &units,
        stage,
        resolved_version.as_deref(),
        None,
        context,
    ))
}

/// Scope-aware pack-key resolver over canonical entries — ONE shared
/// resolution path for pack import and PO interop (no second matcher).
/// A pack file's own scope (Keyed, or `DefInjected/<DefType>`) decides
/// which canonical entries are candidates, and [`crate::matching`] resolves
/// exact keys, proven aliases and TKey-suffix proofs WITHIN that scope —
/// never across kinds or def types.
struct ScopedPackResolver {
    keyed: Option<TypedScope>,
    typed: BTreeMap<String, TypedScope>,
}

/// One def-type bucket. Kind identity is preserved INSIDE the bucket: the
/// exact serialized DefInjected key and the TKey identity are separate maps
/// — an entry of one kind never overwrites the other when both share a
/// display key.
struct TypedScope {
    units: Vec<rimloc_core::TransUnit>,
    registry: TKeyRegistry,
    /// Exact serialized DefInjected elements (`{defName}.{field}`).
    definj: BTreeMap<String, SourceEntryId>,
    /// TKey logical identities (`{defName}.{TKey}`).
    tkey: BTreeMap<String, SourceEntryId>,
}

impl TypedScope {
    fn matcher(&self) -> SourceMatcher<'_> {
        SourceMatcher::new(&self.units, &self.registry)
    }
}

impl ScopedPackResolver {
    fn new(project: &Project) -> Self {
        let mut out = Self {
            keyed: None,
            typed: BTreeMap::new(),
        };
        for e in &project.entries {
            let unit = rimloc_core::TransUnit {
                key: e.id.key.clone(),
                source: Some(e.text.clone()),
                path: PathBuf::from(
                    e.contexts
                        .first()
                        .map(|c| c.file.clone())
                        .unwrap_or_default(),
                ),
                ..Default::default()
            };
            match e.id.kind {
                EntryKind::Keyed => {
                    let scope = out.keyed.get_or_insert_with(|| TypedScope {
                        units: Vec::new(),
                        registry: TKeyRegistry::default(),
                        definj: BTreeMap::new(),
                        tkey: BTreeMap::new(),
                    });
                    scope.units.push(unit);
                    scope.definj.insert(e.id.key.clone(), e.id.clone());
                }
                EntryKind::DefInjected | EntryKind::TKey => {
                    // Exact def-type buckets — no case folding that could
                    // merge two genuinely different types.
                    let Some(dt) = e.id.def_type.clone() else {
                        // Untyped entries cannot be scoped from a pack
                        // folder — never guessed into a match.
                        continue;
                    };
                    let scope = out.typed.entry(dt).or_insert_with(|| TypedScope {
                        units: Vec::new(),
                        registry: TKeyRegistry::default(),
                        definj: BTreeMap::new(),
                        tkey: BTreeMap::new(),
                    });
                    if e.tkey.is_some() {
                        scope.registry.identities.insert(e.id.key.clone());
                    }
                    scope.units.push(unit);
                    match e.id.kind {
                        EntryKind::DefInjected => {
                            scope.definj.insert(e.id.key.clone(), e.id.clone());
                        }
                        _ => {
                            scope.tkey.insert(e.id.key.clone(), e.id.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The canonical identity a pack line (path + key) addresses, if any.
    fn resolve(&self, pack_path: &Path, pack_key: &str) -> Option<SourceEntryId> {
        match self.resolve_report(pack_path, pack_key) {
            PackResolution::Matched(id) => Some(id),
            _ => None,
        }
    }

    /// FULL resolution report for the dry-run analyzer (W2): Matched /
    /// Ambiguous / Unmatched. The plain [`Self::resolve`] (application) is
    /// this filtered to `Matched`, so the analyzer's reusable set is
    /// EXACTLY what application applies — analysis and application can
    /// never disagree about what a pack line addresses.
    fn resolve_report(&self, pack_path: &Path, pack_key: &str) -> PackResolution {
        let s = pack_path.to_string_lossy().replace('\\', "/");
        if s.contains("/Keyed/") {
            let Some(scope) = self.keyed.as_ref() else {
                return PackResolution::Unmatched;
            };
            return match scope.definj.get(pack_key) {
                Some(id) => PackResolution::Matched(id.clone()),
                None => PackResolution::Unmatched,
            };
        }
        let typed = if let Some(i) = s.find("/DefInjected/") {
            let seg = s[i + "/DefInjected/".len()..]
                .split('/')
                .next()
                .unwrap_or_default();
            (!seg.is_empty()).then(|| seg.to_string())
        } else {
            None
        };
        let Some(dt) = typed else {
            return PackResolution::Unmatched;
        };
        let Some(scope) = self.typed.get(&dt) else {
            return PackResolution::Unmatched;
        };
        // 1) The real serialized native DefInjected element wins over any
        //    alias path.
        if let Some(id) = scope.definj.get(pack_key) {
            return PackResolution::Matched(id.clone());
        }
        // 2) The canonical matcher within the SAME scope: exact TKey
        //    identity, proven alias, known suffix — kinds are never swapped
        //    and an unresolved line stays unresolved (no guessed match).
        //    Ambiguity is REPORTED for review, never resolved to an
        //    arbitrary winner.
        match scope.matcher().resolve_target(pack_key) {
            crate::matching::Resolution::Matched { source_key, .. } => scope
                .tkey
                .get(&source_key)
                .or_else(|| scope.definj.get(&source_key))
                .map(|id| PackResolution::Matched(id.clone()))
                .unwrap_or(PackResolution::Unmatched),
            crate::matching::Resolution::Ambiguous { candidates, .. } => {
                PackResolution::Ambiguous(candidates)
            }
            crate::matching::Resolution::Unmatched { .. } => PackResolution::Unmatched,
        }
    }
}

/// Dry-run resolution outcome of one existing-pack line (W2).
#[derive(Debug, Clone, PartialEq, Eq)]
enum PackResolution {
    /// Addresses exactly one canonical identity.
    Matched(SourceEntryId),
    /// Could plausibly address more than one source identity (alias /
    /// exact-entry collision in the shared matcher). Reported for review,
    /// never auto-applied.
    Ambiguous(Vec<String>),
    /// The pack line addresses nothing in the project inventory
    /// (typically a key from an older source version).
    Unmatched,
}

/// Workflow C seed: import an existing translation pack into the project.
///
/// Resolution is SCOPE-AWARE over the canonical identity (identity fix):
/// the pack file's own scope — Keyed, or `DefInjected/<DefType>` — decides
/// which canonical entries are candidates, and the ONE shared matcher
/// ([`crate::matching`]) resolves exact keys, proven aliases and
/// TKey-suffix proofs WITHIN that scope. Matching never crosses kinds or
/// def types: a Keyed line maps to the Keyed entry, a `DefInjected/ThingDef`
/// element to the ThingDef entry — never to an AbilityDef entry sharing the
/// same logical key. Every match is recorded with origin=Imported; nothing
/// is overwritten silently (existing translations win only if the slot was
/// empty).
pub fn apply_existing_translation(
    project: &mut Project,
    pack_root: &Path,
    locale: &str,
) -> Result<usize> {
    let resolver = ScopedPackResolver::new(project);
    let pack_units = rimloc_parsers_xml::scan_keyed_xml(pack_root)?;
    let mut applied = 0usize;
    for u in &pack_units {
        // THE SAME text rule the dry-run analyzer applies (empty / TODO
        // placeholders are never reusable): analysis and application must
        // agree line by line, so a "TODO" marker can never land in an empty
        // slot as a fake "translation".
        let Some(text) = u
            .source
            .as_deref()
            .map(str::trim)
            .filter(|t| !crate::matching::is_todo(t))
        else {
            continue;
        };
        let Some(id) = resolver.resolve(&u.path, &u.key) else {
            continue;
        };
        if project.translation(&id, locale).is_none() {
            project.update_translation(id, locale, Some(text.to_string()), Origin::Imported);
            applied += 1;
        }
    }
    Ok(applied)
}

// ---------------------------------------------------------------------------
// Existing-pack dry-run analysis (W2, mandate "update an existing
// translation"): classify a translation pack against the canonical project
// BEFORE anything is written. ONE resolution path — the same
// [`ScopedPackResolver`] application uses — so the analysis's reusable set
// is by construction what `apply_existing_translation` applies.
// ---------------------------------------------------------------------------

/// Size cap for the per-category sample lists in [`ExistingPackAnalysis`].
/// Counts are always exact; the lists are capped samples for review.
pub const EXISTING_ANALYSIS_LIST_LIMIT: usize = 50;

/// One analyzed existing-pack line (sample list item).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingPackLine {
    /// The pack's serialization key.
    pub key: String,
    /// The canonical identity the line addresses (reusable / conflict only).
    pub target: Option<SourceEntryId>,
}

/// One ambiguous pack line: the matcher reported several candidate source
/// identities; a human decides, the application never auto-applies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingPackAmbiguousLine {
    pub key: String,
    /// Candidate source keys (sorted, from the shared matcher).
    pub candidates: Vec<String>,
}

/// Dry-run classification of a translation pack against the canonical
/// project (NEVER mutates anything):
///
/// - `reusable` — matched a project entry whose `<locale>` slot is empty:
///   exactly what `apply_existing_translation` would apply;
/// - `conflicts` — matched a project entry that ALREADY has a `<locale>`
///   translation: the existing (human/imported) work wins, never
///   overwritten;
/// - `obsolete` — pack lines addressing nothing in the inventory
///   (typically keys from an older source version); preserved in the pack,
///   never applied;
/// - `ambiguous` — the shared matcher reported several candidate
///   identities; reported for review, never auto-applied;
/// - `invalid` — empty or TODO-only pack lines (nothing to reuse);
/// - `new_uncovered` — project entries that stay untranslated after the
///   merge: the pack has no line for them ("new" source strings).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExistingPackAnalysis {
    pub scanned_files: usize,
    pub scanned_keys: usize,
    /// EXACT totals over every scanned line ...
    pub reusable_count: usize,
    pub conflict_count: usize,
    pub obsolete_count: usize,
    pub ambiguous_count: usize,
    pub invalid_count: usize,
    /// ... and size-capped sample lists (see
    /// [`EXISTING_ANALYSIS_LIST_LIMIT`]) for review UIs.
    pub reusable: Vec<ExistingPackLine>,
    pub conflicts: Vec<ExistingPackLine>,
    pub obsolete: Vec<ExistingPackLine>,
    pub ambiguous: Vec<ExistingPackAmbiguousLine>,
    pub invalid: Vec<ExistingPackLine>,
    /// Project entries that stay untranslated after the merge: the pack
    /// has no line for them ("new" source strings).
    pub new_uncovered: usize,
}

impl ExistingPackAnalysis {
    /// Cap a category list to the sample limit, keeping the EXACT total.
    fn capped(lines: Vec<ExistingPackLine>) -> (usize, Vec<ExistingPackLine>) {
        let total = lines.len();
        (
            total,
            lines
                .into_iter()
                .take(EXISTING_ANALYSIS_LIST_LIMIT)
                .collect(),
        )
    }
}

/// Dry-run analyzer over an existing translation pack (READ-ONLY on both
/// sides: the pack is scanned, the project is borrowed). The reusable set
/// is defined through the SAME resolution application uses, so a passing
/// analysis followed by `apply_existing_translation` yields exactly the
/// reusable lines and nothing else.
pub fn analyze_existing_translation(
    project: &Project,
    pack_root: &Path,
    locale: &str,
) -> Result<ExistingPackAnalysis> {
    let resolver = ScopedPackResolver::new(project);
    let pack_units = rimloc_parsers_xml::scan_keyed_xml(pack_root)?;

    let mut reusable = Vec::new();
    let mut conflicts = Vec::new();
    let mut obsolete = Vec::new();
    let mut ambiguous = Vec::new();
    let mut invalid = Vec::new();
    let mut covered: std::collections::BTreeSet<SourceEntryId> = Default::default();
    let mut files: std::collections::BTreeSet<PathBuf> = Default::default();
    for u in &pack_units {
        files.insert(
            u.path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| u.path.clone()),
        );
        let text = u.source.as_deref().map(str::trim).unwrap_or_default();
        // ONE text rule shared with `apply_existing_translation` (empty /
        // TODO placeholders): the reusable set is EXACTLY what application
        // applies, line by line.
        if crate::matching::is_todo(text) {
            invalid.push(ExistingPackLine {
                key: u.key.clone(),
                target: None,
            });
            continue;
        }
        match resolver.resolve_report(&u.path, &u.key) {
            PackResolution::Matched(id) => {
                covered.insert(id.clone());
                let line = ExistingPackLine {
                    key: u.key.clone(),
                    target: Some(id),
                };
                match project.translation(&line.target.clone().unwrap(), locale) {
                    Some(_) => conflicts.push(line),
                    None => reusable.push(line),
                }
            }
            PackResolution::Ambiguous(candidates) => {
                ambiguous.push(ExistingPackAmbiguousLine {
                    key: u.key.clone(),
                    candidates,
                });
            }
            PackResolution::Unmatched => {
                obsolete.push(ExistingPackLine {
                    key: u.key.clone(),
                    target: None,
                });
            }
        }
    }

    let mut out = ExistingPackAnalysis {
        scanned_files: files.len(),
        scanned_keys: pack_units.len(),
        new_uncovered: 0,
        ..Default::default()
    };
    (out.reusable_count, out.reusable) = ExistingPackAnalysis::capped(reusable);
    (out.conflict_count, out.conflicts) = ExistingPackAnalysis::capped(conflicts);
    (out.obsolete_count, out.obsolete) = ExistingPackAnalysis::capped(obsolete);
    (out.ambiguous_count, out.ambiguous) = {
        let total = ambiguous.len();
        (
            total,
            ambiguous
                .into_iter()
                .take(EXISTING_ANALYSIS_LIST_LIMIT)
                .collect(),
        )
    };
    (out.invalid_count, out.invalid) = ExistingPackAnalysis::capped(invalid);

    // "New" = inventory entries that stay untranslated after the merge:
    // no `<locale>` translation now AND no pack line covered them.
    for entry in &project.entries {
        if project.translation(&entry.id, locale).is_some() {
            continue;
        }
        if !covered.contains(&entry.id) {
            out.new_uncovered += 1;
        }
    }
    Ok(out)
}

/// Workflow A final step: write RimWorld translation output straight from
/// the canonical project — NO PO intermediate anywhere.
///
/// Layout mirrors the official pack conventions:
/// - Keyed entries → `Keyed/<locale>.xml` (flat elements, key = entry key);
/// - DefInjected/TKey entries → `DefInjected/<DefType>/<defName>.xml`,
///   element name = key (+ TKey suffix when the kind is TKey).
///
/// Result of a native output write: the output mod root plus the
/// identities that were SKIPPED because their def type could not be
/// resolved from any provenance. A skipped entry is never guessed into a
/// "Misc" catalog — the identity stays unknown by contract, and the report
/// names it for review/rescan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub out_mod: PathBuf,
    /// Display identities skipped as unknown-def-type (sorted, unique).
    pub skipped_unknown_type: Vec<String>,
    /// Leaf keys actually written into the Languages tree (unique Keyed
    /// keys + DefInjected elements). This is the WRITER's own acceptance
    /// accounting; the export reparse guard compares the scanner output
    /// against it instead of re-deriving the acceptance filters
    /// (re-derivation drifts from the writer and yields false mismatches).
    pub keys_written: usize,
}

pub fn write_rimworld_translation(
    project: &Project,
    out_mod: &Path,
    lang_dir: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> Result<WriteReport> {
    use std::fmt::Write as _;

    // H1: the deepest write choke point — every caller (session export,
    // CLI build paths) gets the strict form + containment guard.
    crate::util::ensure_lang_write_target(out_mod, lang_dir)?;

    let base = out_mod.join("Languages").join(lang_dir);
    // defName -> file; collected per def type from entry keys/contexts.
    let mut keyed: BTreeMap<String, String> = BTreeMap::new();
    let mut definj: BTreeMap<(String, String, String), BTreeMap<String, String>> = BTreeMap::new(); // (def_type, def_name, file) -> (element, text)
    let mut skipped_unknown_type: Vec<String> = Vec::new();

    for t in &project.translations {
        if t.locale != lang_dir && !t.locale.is_empty() {
            // Locale is the target folder name; keep entries for it only.
        }
        if t.locale != lang_dir {
            continue;
        }
        let Some(text) = t.text.as_deref().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let Some(entry) = project.entries.iter().find(|e| e.id == t.source_id) else {
            continue;
        };
        if t.lifecycle == dom::Lifecycle::Obsolete {
            continue;
        }
        match entry.id.kind {
            EntryKind::Keyed => {
                keyed.insert(entry.id.key.clone(), text.to_string());
            }
            EntryKind::TKey | EntryKind::DefInjected => {
                // The identity discriminator is authoritative for output
                // grouping: each def type lands in its own DefInjected
                // catalog even when defName/field keys collide across types.
                // An entry with NO resolvable type is SKIPPED with a
                // diagnostic — never guessed into a functional "Misc"
                // catalog (unknown stays unknown by contract).
                let Some(def_type) = entry
                    .id
                    .def_type
                    .clone()
                    .or_else(|| entry.tkey.as_ref().map(|m| m.def_type.clone()))
                    .or_else(|| def_type_from_contexts(entry))
                else {
                    let identity = entry.id.display_identity();
                    if !skipped_unknown_type.contains(&identity) {
                        skipped_unknown_type.push(identity);
                    }
                    continue;
                };
                let def_name = entry
                    .id
                    .key
                    .split('.')
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let element = match &entry.tkey {
                    Some(m) => format!("{}{}", entry.id.key, m.suffix),
                    None => entry.id.key.clone(),
                };
                let file = format!("{def_name}.xml");
                definj
                    .entry((def_type, def_name, file))
                    .or_default()
                    .insert(element, text.to_string());
            }
            _ => {
                // Strings/Backstories/PatchDerived writers land with their
                // gates; not part of this acceptance.
            }
        }
    }

    // About.xml
    let about = out_mod.join("About");
    std::fs::create_dir_all(&about)?;
    let mut about_xml = String::new();
    // H4: manifest fields are user-controlled text — escaped, never raw
    // (& < > in a folder name used to produce invalid XML).
    let _ = writeln!(
        about_xml,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<RimWorldManifest>\n  <name>{}</name>\n  <packageId>{}</packageId>\n  <supportedVersions>\n    <li>{}</li>\n  </supportedVersions>\n</RimWorldManifest>",
        rimloc_core::xml_chars::escape_text(mod_name),
        rimloc_core::xml_chars::escape_text(package_id),
        rimloc_core::xml_chars::escape_text(rw_version)
    );
    crate::write_atomic(&about.join("About.xml"), about_xml.as_bytes())?;

    // Keyed
    if !keyed.is_empty() {
        let dir = base.join("Keyed");
        std::fs::create_dir_all(&dir)?;
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<LanguageData>\n");
        for (k, v) in &keyed {
            let _ = writeln!(xml, "  <{k}>{}</{k}>", escape_xml(v));
        }
        xml.push_str("</LanguageData>\n");
        crate::write_atomic(&dir.join("Translation.xml"), xml.as_bytes())?;
    }

    // DefInjected
    for ((def_type, _def_name, file), items) in &definj {
        let dir = base.join("DefInjected").join(def_type);
        std::fs::create_dir_all(&dir)?;
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<LanguageData>\n");
        for (element, text) in items {
            let _ = writeln!(xml, "  <{element}>{}</{element}>", escape_xml(text));
        }
        xml.push_str("</LanguageData>\n");
        crate::write_atomic(&dir.join(file), xml.as_bytes())?;
    }

    let keys_written = keyed.len() + definj.values().map(|items| items.len()).sum::<usize>();
    Ok(WriteReport {
        out_mod: out_mod.to_path_buf(),
        skipped_unknown_type,
        keys_written,
    })
}

/// Game-loadable `About/About.xml` in the CLI build-mod shape: a
/// `<ModMetaData>` root with packageId/name/description/supportedVersions —
/// exactly what `rimloc_import_po::write_about_xml` (behind
/// `build_from_po_execute`) emits for `rimloc build-mod`, so the package
/// this writer produces drops straight into the game's Mods folder. The
/// export writer's own `<RimWorldManifest>` stays untouched (its semantics
/// are frozen); this is the build-mod counterpart. H4: manifest fields are
/// user-controlled text — escaped, never raw.
pub fn write_modmetadata_about(
    out_mod: &Path,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> Result<()> {
    use std::fmt::Write as _;
    let about = out_mod.join("About");
    std::fs::create_dir_all(&about)?;
    let mut about_xml = String::new();
    let _ = write!(
        about_xml,
        "<ModMetaData>\n  <packageId>{}</packageId>\n  <name>{}</name>\n  <description>Translation mod (generated by RimLoc)</description>\n  <supportedVersions>\n    <li>{}</li>\n  </supportedVersions>\n</ModMetaData>\n",
        rimloc_core::xml_chars::escape_text(package_id),
        rimloc_core::xml_chars::escape_text(mod_name),
        rimloc_core::xml_chars::escape_text(rw_version)
    );
    crate::write_atomic(&about.join("About.xml"), about_xml.as_bytes())?;
    Ok(())
}

pub(crate) fn def_type_from_contexts(
    entry: &rimloc_domain::canonical::SourceEntry,
) -> Option<String> {
    for c in &entry.contexts {
        let s = c.file.replace('\\', "/");
        if let Some(i) = s.find("/DefInjected/") {
            let rest = &s[i + "/DefInjected/".len()..];
            return Some(rest.split('/').next().unwrap_or_default().to_string());
        }
    }
    None
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Source provenance stays reachable for diagnostics without per-entry noise.
pub fn provenance_summary(project: &Project) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for e in &project.entries {
        let key = format!(
            "version={};conditional={};patch={}",
            e.provenance.version_selected.as_deref().unwrap_or("-"),
            e.provenance.conditional_branch,
            match e.provenance.patch_stage {
                PatchStage::None => "none",
                PatchStage::Applied => "applied",
                PatchStage::Partial => "partial",
            }
        );
        *out.entry(key).or_default() += 1;
    }
    out
}

#[cfg(test)]
mod gate_i_tests {
    use super::*;
    use rimloc_domain::canonical::{Completeness, SourceProvenance};
    use rimloc_domain::canonical::{SourceEntry, Translation};

    fn project_with_translation(
        key: &str,
        kind: EntryKind,
        text: &str,
        tkey: Option<rimloc_core::TKeyMeta>,
    ) -> Project {
        let id = SourceEntryId {
            kind,
            key: key.into(),
            def_type: None,
        };
        Project {
            entries: vec![SourceEntry {
                id: id.clone(),
                text: "source".into(),
                source_locale: "en".into(),
                contexts: vec![],
                provenance: SourceProvenance::default(),
                tkey,
                source_ref: None,
            }],
            translations: vec![Translation {
                source_id: id,
                locale: "Russian".into(),
                text: Some(text.into()),
                completeness: Completeness::Translated,
                review: dom::Review::None,
                validation: dom::ValidationState::Unknown,
                lifecycle: dom::Lifecycle::Active,
                origin: Origin::Human,
                notes: String::new(),
                source_changed: None,
            }],
            ..Default::default()
        }
    }

    /// Workflow A core proof: project -> RimWorld output WITHOUT any PO.
    #[test]
    fn write_rimworld_emits_definjected_and_keyed_from_project() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("mod");
        let tk = rimloc_core::TKeyMeta {
            strategy: "slate_ref".into(),
            suffix: ".slateRef".into(),
            def_type: "QuestScriptDef".into(),
            contexts: 1,
            locations: Vec::new(),
        };
        let mut p = project_with_translation(
            "SampleQuest.LetterLabel",
            EntryKind::TKey,
            "Метка",
            Some(tk.clone()),
        );
        p.translations.push(Translation {
            source_id: SourceEntryId {
                kind: EntryKind::Keyed,
                key: "Greeting".into(),
                def_type: None,
            },
            locale: "Russian".into(),
            text: Some("Привет".into()),
            completeness: Completeness::Translated,
            review: dom::Review::None,
            validation: dom::ValidationState::Unknown,
            lifecycle: dom::Lifecycle::Active,
            origin: Origin::Human,
            notes: String::new(),
            source_changed: None,
        });
        p.entries.push(SourceEntry {
            id: SourceEntryId {
                kind: EntryKind::Keyed,
                key: "Greeting".into(),
                def_type: None,
            },
            text: "hello".into(),
            source_locale: "en".into(),
            contexts: vec![],
            provenance: SourceProvenance::default(),
            tkey: None,
            source_ref: None,
        });
        let report = write_rimworld_translation(&p, &out, "Russian", "T", "t.test", "1.6").unwrap();
        let out = report.out_mod;
        let definj = std::fs::read_to_string(
            out.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
        )
        .unwrap();
        assert!(
            definj.contains("<SampleQuest.LetterLabel.slateRef>Метка</"),
            "{definj}"
        );
        let keyed =
            std::fs::read_to_string(out.join("Languages/Russian/Keyed/Translation.xml")).unwrap();
        assert!(keyed.contains("<Greeting>Привет</Greeting>"), "{keyed}");
    }
}

#[cfg(test)]
mod gate_i4_acceptance {
    use super::*;
    use rimloc_domain::canonical::EntryKind;

    const FIXTURE: &str = "test/TKeyMod";

    fn fixture_path() -> PathBuf {
        // Integration-style: run against the repo fixture (crate dir -> repo
        // root -> test/).
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(FIXTURE)
            .canonicalize()
            .expect("fixture exists")
    }

    /// THE Gate I acceptance: all three workflows over ONE canonical model.
    ///
    /// A. native/no-PO:  source -> project -> apply existing RU -> write.
    /// B. PO interop:    project -> PO file -> import -> project -> write.
    /// C. existing pack: covered by A's import step (preserve; the shared
    ///    text rule refuses TODO placeholders on BOTH the analyzer and the
    ///    application, so a TODO marker never lands in a slot as a fake
    ///    translation), plus reopen via the persistence store between
    ///    workflows.
    #[test]
    fn three_workflows_over_one_canonical_project() {
        let root = fixture_path();
        let tmp = tempfile::tempdir().unwrap();

        // ---------- Workflow A: build + import existing RU + write ----------
        let mut project = build_project(&root, Some("1.6"), None).unwrap();
        // The view is EXACT for a known version; patches absent -> stage None.
        assert_eq!(project.context.view, ViewLabel::Exact);
        let ru_dir = root.join("Languages/Russian");
        let applied = apply_existing_translation(&mut project, &ru_dir, "Russian").unwrap();
        assert!(applied >= 4, "existing RU entries must map, got {applied}");

        // Save/reopen between workflows (project persistence, Gate I3).
        let project_file = tmp.path().join("project.rimloc.json");
        crate::project_store::save_project(&project, &project_file).unwrap();
        let project = crate::project_store::load_project(&project_file).unwrap();

        let out_a = tmp.path().join("out-a");
        write_rimworld_translation(&project, &out_a, "Russian", "T", "t.a", "1.6").unwrap();
        let quest_a = std::fs::read_to_string(
            out_a.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
        )
        .unwrap();
        assert!(
            quest_a.contains("<SampleQuest.LetterLabelFavorReceiver.slateRef>Метка услуги<"),
            "{quest_a}"
        );
        assert!(
            quest_a.contains("<SampleQuest.LetterTextParms.value.slateRef>Пармс-текст.<"),
            "{quest_a}"
        );
        // The pack's TODO placeholder is refused by the shared text rule
        // (analyze AND apply): the slot stays empty, so the entry is not
        // written at all — it stays on the to-translate list instead of
        // exporting a literal "TODO" as if it were a translation.
        assert!(
            !quest_a.contains("ExpiryTip"),
            "TODO placeholder must not be imported as a translation: {quest_a}"
        );
        let tips_a = std::fs::read_to_string(
            out_a.join("Languages/Russian/DefInjected/TipSetDef/SampleTips.xml"),
        )
        .unwrap();
        assert!(
            tips_a.contains("<SampleTips.DismissLetters>Подсказки"),
            "{tips_a}"
        );

        // ---------- Workflow B: PO interoperability adapter ----------
        // project -> PO file (adapter export over canonical translations)...
        // Synthesized reference paths carry the DefInjected/<DefType>/
        // scope so the import resolves through the same identity rules.
        let po_path = tmp.path().join("interop.po");
        let po_units: Vec<rimloc_core::TransUnit> = project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .filter_map(|t| {
                project
                    .entries
                    .iter()
                    .find(|e| e.id == t.source_id)
                    .map(|e| {
                        let def_type =
                            e.id.def_type
                                .clone()
                                .or_else(|| e.tkey.as_ref().map(|m| m.def_type.clone()))
                                .unwrap_or_else(|| "Misc".into());
                        rimloc_core::TransUnit {
                            key: match (&e.id.kind, &e.tkey) {
                                (EntryKind::TKey, Some(m)) => format!("{}{}", e.id.key, m.suffix),
                                _ => e.id.key.clone(),
                            },
                            source: Some(e.text.clone()),
                            path: PathBuf::from(format!(
                                "Languages/Russian/DefInjected/{def_type}/interop.xml"
                            )),
                            ..Default::default()
                        }
                    })
            })
            .collect();
        // msgstr filled from the project's own translations (as if a
        // translator had completed them in an external PO editor).
        let tm_map: std::collections::HashMap<String, String> = project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .filter_map(|t| {
                project
                    .entries
                    .iter()
                    .find(|e| e.id == t.source_id)
                    .and_then(|e| {
                        t.text.clone().map(|text| {
                            let key = match (&e.id.kind, &e.tkey) {
                                (EntryKind::TKey, Some(m)) => {
                                    format!("{}{}", e.id.key, m.suffix)
                                }
                                _ => e.id.key.clone(),
                            };
                            (key, text)
                        })
                    })
            })
            .collect();
        rimloc_export_po::write_po_with_tm(&po_path, &po_units, Some("ru"), Some(&tm_map)).unwrap();
        // ...external-like import into a FRESH project (same source scan),
        // through the SAME scope-aware resolver as pack import — the
        // identity fix has no separate PO matching path...
        let mut project_b = build_project(&root, Some("1.6"), None).unwrap();
        let entries = rimloc_import_po::read_po_entries(&po_path).unwrap();
        let mut merged = 0;
        for e in &entries {
            let v = e.value.trim();
            if v.is_empty() {
                continue;
            }
            // Reconstruct the scoped reference path: the PO key carries the
            // identity (TKey suffix included); the scope comes from the
            // def type of whichever entry the PO was exported from — which
            // the fresh project shares, so resolve by scanning its own
            // DefInjected/TKey entries for the matching serialization key.
            let Some(id) = project_b
                .entries
                .iter()
                .find(|en| {
                    let serialization_key = match (&en.id.kind, &en.tkey) {
                        (EntryKind::TKey, Some(m)) => format!("{}{}", en.id.key, m.suffix),
                        _ => en.id.key.clone(),
                    };
                    serialization_key == e.key
                })
                .map(|en| en.id.clone())
            else {
                continue;
            };
            project_b.update_translation(id, "Russian", Some(v.to_string()), dom::Origin::Imported);
            merged += 1;
        }
        assert!(
            merged >= 4,
            "PO interop must restore translations, got {merged}"
        );
        // ...and the PO round trip writes the SAME RimWorld output.
        let out_b = tmp.path().join("out-b");
        write_rimworld_translation(&project_b, &out_b, "Russian", "T", "t.b", "1.6").unwrap();
        let quest_b = std::fs::read_to_string(
            out_b.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
        )
        .unwrap();
        assert!(quest_b.contains("<SampleQuest.LetterLabelFavorReceiver.slateRef>Метка услуги<"));
    }
}

#[cfg(test)]
mod identity_regression {
    //! Approved architectural identity fix (pre-freeze blocker): two def
    //! types sharing one logical key are TWO identities end-to-end — build,
    //! scoped pack import, save/reload, native output and source-change
    //! detection. Keyed import is fixed by the same scope work.
    use super::*;
    use rimloc_domain::canonical::SourceEntry;
    use rimloc_domain::canonical::{ContextRole, SourceProvenance};

    fn write_def(path: &std::path::Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn collision_mod(dir: &std::path::Path) {
        write_def(
            &dir.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing label</label></ThingDef></Defs>"#,
        );
        write_def(
            &dir.join("Defs/B_Ability.xml"),
            r#"<Defs><AbilityDef><defName>Dup</defName><label>ability label</label></AbilityDef></Defs>"#,
        );
    }

    fn entry_by_type<'a>(p: &'a Project, key: &str, def_type: &str) -> &'a SourceEntry {
        p.entries
            .iter()
            .find(|e| {
                e.id.key == key
                    && e.id
                        .def_type
                        .as_deref()
                        .is_some_and(|dt| dt.eq_ignore_ascii_case(def_type))
            })
            .unwrap_or_else(|| {
                panic!(
                    "entry {} missing; have {:?}",
                    key,
                    p.entries
                        .iter()
                        .map(|e| e.id.display_identity())
                        .collect::<Vec<_>>()
                )
            })
    }

    /// Full pipeline: two def types, one key -> two identities survive
    /// build -> different translations -> save/reload -> native output in
    /// DIFFERENT DefInjected catalogs.
    #[test]
    fn collision_survives_build_translate_save_and_native_output() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mod");
        collision_mod(&root);
        let mut p = build_project(&root, Some("1.6"), None).unwrap();
        let thing = entry_by_type(&p, "Dup.label", "ThingDef").id.clone();
        let ability = entry_by_type(&p, "Dup.label", "AbilityDef").id.clone();
        p.update_translation(thing.clone(), "Russian", Some("вещь".into()), Origin::Human);
        p.update_translation(
            ability.clone(),
            "Russian",
            Some("способность".into()),
            Origin::Human,
        );

        let file = dir.path().join("p.rimloc.json");
        crate::project_store::save_project(&p, &file).unwrap();
        let reloaded = crate::project_store::load_project(&file).unwrap();
        assert_eq!(
            reloaded
                .translation(&thing, "Russian")
                .unwrap()
                .text
                .as_deref(),
            Some("вещь")
        );
        assert_eq!(
            reloaded
                .translation(&ability, "Russian")
                .unwrap()
                .text
                .as_deref(),
            Some("способность")
        );

        let out = dir.path().join("out");
        write_rimworld_translation(&reloaded, &out, "Russian", "T", "t.id", "1.6").unwrap();
        let thing_xml =
            std::fs::read_to_string(out.join("Languages/Russian/DefInjected/ThingDef/Dup.xml"))
                .unwrap();
        let ability_xml =
            std::fs::read_to_string(out.join("Languages/Russian/DefInjected/AbilityDef/Dup.xml"))
                .unwrap();
        assert!(thing_xml.contains("вещь"), "{thing_xml}");
        assert!(!thing_xml.contains("способность"));
        assert!(ability_xml.contains("способность"), "{ability_xml}");
        assert!(!ability_xml.contains("вещь"));
    }

    /// Scoped pack import: the same logical key under Keyed + two def types
    /// maps each pack line to ITS OWN entry (previously the Keyed line was
    /// silently imported as DefInjected and types collided).
    #[test]
    fn scoped_pack_import_matches_same_key_across_kinds_and_types() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mod");
        collision_mod(&root);
        write_def(
            &root.join("Languages/English/Keyed/K.xml"),
            "<LanguageData>\n  <Greeting>hello</Greeting>\n</LanguageData>\n",
        );
        let pack = dir.path().join("pack/Languages/Russian");
        write_def(
            &pack.join("Keyed/Keys.xml"),
            "<LanguageData>\n  <Greeting>Привет</Greeting>\n</LanguageData>\n",
        );
        write_def(
            &pack.join("DefInjected/ThingDef/Dup.xml"),
            "<LanguageData>\n  <Dup.label>вещь</Dup.label>\n</LanguageData>\n",
        );
        write_def(
            &pack.join("DefInjected/AbilityDef/Dup.xml"),
            "<LanguageData>\n  <Dup.label>способность</Dup.label>\n</LanguageData>\n",
        );
        let mut p = build_project(&root, Some("1.6"), None).unwrap();
        let applied = apply_existing_translation(&mut p, &pack, "Russian").unwrap();
        assert_eq!(applied, 3, "Keyed + ThingDef + AbilityDef all scoped-match");
        let thing_t = p
            .translation(&entry_by_type(&p, "Dup.label", "ThingDef").id, "Russian")
            .unwrap();
        assert_eq!(thing_t.text.as_deref(), Some("вещь"));
        let ability_t = p
            .translation(&entry_by_type(&p, "Dup.label", "AbilityDef").id, "Russian")
            .unwrap();
        assert_eq!(ability_t.text.as_deref(), Some("способность"));
        let keyed = p
            .entries
            .iter()
            .find(|e| e.id.kind == EntryKind::Keyed && e.id.key == "Greeting")
            .unwrap();
        assert_eq!(
            p.translation(&keyed.id, "Russian").unwrap().text.as_deref(),
            Some("Привет"),
            "Keyed import works through the same scoped resolver"
        );
    }

    /// TKey suffix proof stays WITHIN the def-type scope: the same
    /// defName+TKey under two def types are two identities; each pack
    /// folder maps to its own serialization.
    #[test]
    fn tkey_suffix_resolution_stays_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mod");
        write_def(
            &root.join("Defs/Q.xml"),
            r#"<Defs><QuestScriptDef><defName>Sample</defName><label TKey="LetterLabel">quest text</label></QuestScriptDef></Defs>"#,
        );
        write_def(
            &root.join("Defs/T.xml"),
            r#"<Defs><TipSetDef><defName>Sample</defName><tips><li TKey="LetterLabel">tip text</li></tips></TipSetDef></Defs>"#,
        );
        let pack = dir.path().join("pack/Languages/Russian");
        write_def(
            &pack.join("DefInjected/QuestScriptDef/Sample.xml"),
            "<LanguageData>\n  <Sample.LetterLabel.slateRef>квест</Sample.LetterLabel.slateRef>\n</LanguageData>\n",
        );
        write_def(
            &pack.join("DefInjected/TipSetDef/Sample.xml"),
            "<LanguageData>\n  <Sample.LetterLabel>подсказка</Sample.LetterLabel>\n</LanguageData>\n",
        );
        let mut p = build_project(&root, Some("1.6"), None).unwrap();
        let quest_id = entry_by_type(&p, "Sample.LetterLabel", "QuestScriptDef")
            .id
            .clone();
        let tip_id = entry_by_type(&p, "Sample.LetterLabel", "TipSetDef")
            .id
            .clone();
        assert_eq!(
            entry_by_type(&p, "Sample.LetterLabel", "QuestScriptDef").text,
            "quest text"
        );
        assert_eq!(
            entry_by_type(&p, "Sample.LetterLabel", "TipSetDef").text,
            "tip text"
        );
        let applied = apply_existing_translation(&mut p, &pack, "Russian").unwrap();
        assert_eq!(applied, 2);
        assert_eq!(
            p.translation(&quest_id, "Russian").unwrap().text.as_deref(),
            Some("квест")
        );
        assert_eq!(
            p.translation(&tip_id, "Russian").unwrap().text.as_deref(),
            Some("подсказка")
        );
    }

    /// Source-change detection compares FULL identities: a change to one
    /// def type must not flag or transplant the other's translation, and a
    /// vanished def type only obsoletes its own work.
    #[test]
    fn source_change_never_transplants_across_types() {
        let dir = tempfile::tempdir().unwrap();
        let v1 = dir.path().join("v1");
        let v2 = dir.path().join("v2");
        write_def(
            &v1.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing v1</label></ThingDef></Defs>"#,
        );
        write_def(
            &v1.join("Defs/B_Ability.xml"),
            r#"<Defs><AbilityDef><defName>Dup</defName><label>ability v1</label></AbilityDef></Defs>"#,
        );
        write_def(
            &v2.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing v2</label></ThingDef></Defs>"#,
        );
        write_def(
            &v2.join("Defs/B_Ability.xml"),
            r#"<Defs><AbilityDef><defName>Dup</defName><label>ability v1</label></AbilityDef></Defs>"#,
        );
        let mut p = build_project(&v1, Some("1.6"), None).unwrap();
        let ability_id = entry_by_type(&p, "Dup.label", "AbilityDef").id.clone();
        p.update_translation(
            ability_id.clone(),
            "Russian",
            Some("способность".into()),
            Origin::Human,
        );

        let report = detect_source_changes(&mut p, &v2, Some("1.6")).unwrap();
        assert_eq!(
            report.source_changed, 0,
            "the ThingDef change must not flag the AbilityDef translation: {report:?}"
        );
        assert_eq!(report.obsolete, 0);
        assert_eq!(
            p.translation(&ability_id, "Russian")
                .unwrap()
                .text
                .as_deref(),
            Some("способность")
        );

        // Now the AbilityDef type disappears entirely: only its own
        // translation becomes obsolete; the ThingDef work is untouched.
        std::fs::remove_file(v2.join("Defs/B_Ability.xml")).unwrap();
        let report = detect_source_changes(&mut p, &v2, Some("1.6")).unwrap();
        assert_eq!(report.obsolete, 1, "{report:?}");
        assert_eq!(
            p.translation(&ability_id, "Russian").unwrap().lifecycle,
            dom::Lifecycle::Obsolete
        );
        assert_eq!(
            p.translation(&ability_id, "Russian")
                .unwrap()
                .text
                .as_deref(),
            Some("способность")
        );
    }
    /// P2-1: an entry with NO resolvable def type (post-v1-migration
    /// "unknown stays None") is SKIPPED from native output and named in the
    /// write report — never guessed into a functional
    /// `DefInjected/Misc/...` catalog.
    #[test]
    fn unknown_def_type_entry_is_skipped_not_misc() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mod");
        write_def(
            &root.join("Defs/Widget.xml"),
            r#"<Defs><ThingDef><defName>Widget</defName><label>real label</label></ThingDef></Defs>"#,
        );
        let mut p = build_project(&root, Some("1.6"), None).unwrap();
        // A legacy entry whose type is genuinely unknown everywhere.
        let unknown = SourceEntryId {
            kind: EntryKind::DefInjected,
            key: "Ghost.label".into(),
            def_type: None,
        };
        p.entries.push(SourceEntry {
            id: unknown.clone(),
            text: "ghost text".into(),
            source_locale: "en".into(),
            contexts: vec![rimloc_domain::canonical::SourceContext {
                file: "legacy/Unknown.xml".into(),
                line: None,
                def_type: None,
                role: ContextRole::Effective,
            }],
            provenance: SourceProvenance::default(),
            tkey: None,
            source_ref: None,
        });
        p.update_translation(
            unknown.clone(),
            "Russian",
            Some("призрак".into()),
            Origin::Human,
        );
        let widget_id = SourceEntryId {
            kind: EntryKind::DefInjected,
            key: "Widget.label".into(),
            def_type: Some("ThingDef".into()),
        };
        p.update_translation(widget_id, "Russian", Some("метка".into()), Origin::Human);

        let out = dir.path().join("out");
        let report = write_rimworld_translation(&p, &out, "Russian", "T", "t.p2", "1.6").unwrap();
        assert_eq!(
            report.skipped_unknown_type,
            vec!["def_injected·Ghost.label"],
            "the unknown entry is named, not guessed"
        );
        // The real entry is written; no Misc catalog exists at all.
        let widget =
            std::fs::read_to_string(out.join("Languages/Russian/DefInjected/ThingDef/Widget.xml"))
                .unwrap();
        assert!(
            widget.contains("метка") || widget.contains("label"),
            "{widget}"
        );
        assert!(
            !out.join("Languages/Russian/DefInjected/Misc").exists(),
            "no invented Misc catalog"
        );
    }

    /// 028-2: same DefType AND same display key for a plain DefInjected
    /// sidecar AND a TKey identity — both entries exist, and scoped pack
    /// import lands each pack line on its OWN kind (native element exact
    /// first, TKey-suffix proof second), never one overwriting the other.
    #[test]
    fn same_key_tkey_and_definjected_both_kept_and_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mod");
        write_def(
            &root.join("Defs/Q.xml"),
            r#"<Defs><QuestScriptDef><defName>Sample</defName><label TKey="LetterLabel">tkey text</label></QuestScriptDef></Defs>"#,
        );
        write_def(
            &root.join("Languages/English/DefInjected/QuestScriptDef/Sample.xml"),
            "<LanguageData>\n  <Sample.LetterLabel>sidecar text</Sample.LetterLabel>\n</LanguageData>\n",
        );
        let pack = dir.path().join("pack/Languages/Russian");
        write_def(
            &pack.join("DefInjected/QuestScriptDef/Sample.xml"),
            "<LanguageData>\n  <Sample.LetterLabel>сайдкар</Sample.LetterLabel>\n  <Sample.LetterLabel.slateRef>ткей</Sample.LetterLabel.slateRef>\n</LanguageData>\n",
        );
        let mut p = build_project(&root, Some("1.6"), None).unwrap();
        let definj_id = p
            .entries
            .iter()
            .find(|e| e.id.kind == EntryKind::DefInjected && e.id.key == "Sample.LetterLabel")
            .expect("plain DefInjected identity kept")
            .id
            .clone();
        let tkey_id = p
            .entries
            .iter()
            .find(|e| e.id.kind == EntryKind::TKey && e.id.key == "Sample.LetterLabel")
            .expect("TKey identity kept alongside the same-key sidecar")
            .id
            .clone();
        assert_eq!(
            p.entries.iter().find(|e| e.id == definj_id).unwrap().text,
            "sidecar text"
        );
        assert_eq!(
            p.entries.iter().find(|e| e.id == tkey_id).unwrap().text,
            "tkey text"
        );
        let applied = apply_existing_translation(&mut p, &pack, "Russian").unwrap();
        assert_eq!(applied, 2, "each pack line maps to its OWN kind");
        assert_eq!(
            p.translation(&definj_id, "Russian")
                .unwrap()
                .text
                .as_deref(),
            Some("сайдкар")
        );
        assert_eq!(
            p.translation(&tkey_id, "Russian").unwrap().text.as_deref(),
            Some("ткей")
        );
    }

    /// 028-5: detect_source_changes keeps the RESOLVED version on new
    /// entries — a requested 1.6 against a mod that resolves to 1.5 must
    /// not stamp 1.6 onto fresh provenance.
    #[test]
    fn rescan_preserves_resolved_version_on_new_entries() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::write(
            root.join("LoadFolders.xml"),
            "<loadFolders><v1.5><li>/</li><li>1.5</li></v1.5></loadFolders>",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("1.5/Defs")).unwrap();
        std::fs::write(
            root.join("1.5/Defs/A.xml"),
            r#"<Defs><ThingDef><defName>F</defName><label>v15</label></ThingDef></Defs>"#,
        )
        .unwrap();
        let mut p = build_project(root, Some("1.6"), None).unwrap();
        // Later rescan with a NEW source entry in the same 1.5 root.
        std::fs::write(
            root.join("1.5/Defs/B.xml"),
            r#"<Defs><ThingDef><defName>G</defName><label>brand new</label></ThingDef></Defs>"#,
        )
        .unwrap();
        let report = detect_source_changes(&mut p, root, Some("1.6")).unwrap();
        assert_eq!(report.new_source, 1, "{report:?}");
        let g = p.entries.iter().find(|e| e.id.key == "G.label").unwrap();
        assert_eq!(
            g.provenance.version_selected.as_deref(),
            Some("1.5"),
            "resolved 1.5 must survive the rescan (not the requested 1.6)"
        );
    }
}

/// Maintenance pipeline (Gate K): after the source mod updated, rebuild the
/// effective inventory and compare per canonical identity. Translations whose
/// source text changed are flagged (sourceChanged + Pending review); new
/// source identities become plain untranslated entries; vanished identities
/// keep their translations but are marked obsolete in diagnostics count.
/// Nothing is deleted — user work is preserved (mandate 4 §3).
pub fn detect_source_changes(
    project: &mut Project,
    updated_mod_root: &Path,
    target_version: Option<&str>,
) -> Result<SourceChangeReport> {
    let fresh = build_project(updated_mod_root, target_version, None)?;
    let mut report = SourceChangeReport::default();

    // FULL STRUCTURAL identity compare (the SourceEntryId itself — never a
    // concatenated string): a shared display key across two def types or
    // kinds must never transplant a translation from one identity to
    // another.
    let fresh_by_id: std::collections::HashMap<SourceEntryId, &SourceEntry> =
        fresh.entries.iter().map(|e| (e.id.clone(), e)).collect();
    let old_by_id: std::collections::HashMap<SourceEntryId, &SourceEntry> =
        project.entries.iter().map(|e| (e.id.clone(), e)).collect();

    // 1. Changed source text for translated entries.
    for t in &mut project.translations {
        if t.text.is_none() {
            continue;
        }
        if let Some(old_entry) = old_by_id.get(&t.source_id) {
            match fresh_by_id.get(&t.source_id) {
                Some(new_entry) => {
                    if new_entry.text != old_entry.text && t.source_changed.is_none() {
                        t.source_changed = Some("source text changed".into());
                        t.review = dom::Review::Pending;
                        report.source_changed += 1;
                    }
                }
                None => {
                    t.lifecycle = dom::Lifecycle::Obsolete;
                    report.obsolete += 1;
                }
            }
        }
    }

    // 2. New source identities -> fresh untranslated entries in the project.
    // Owned ids end the borrow of project.entries before we push into it.
    let old_ids: std::collections::HashSet<SourceEntryId> = old_by_id.keys().cloned().collect();
    for e in &fresh.entries {
        if !old_ids.contains(&e.id) {
            // Fresh provenance is AUTHORITATIVE: the scan already recorded
            // the RESOLVED version — a requested target_version never
            // overwrites it (version-fallback contract).
            project.entries.push(e.clone());
            report.new_source += 1;
        }
    }

    // 3. Reusable: translations whose source text is unchanged stay as-is.
    report.reusable = project
        .translations
        .iter()
        .filter(|t| t.text.is_some() && t.source_changed.is_none())
        .count();

    Ok(report)
}

#[cfg(test)]
mod provenance_regression {
    //! Pre-freeze provenance regressions (Source Inspector mandate §1/§7/§14).
    //! Fixture: test/ProvenanceMod — a LoadFolders mod exercising every
    //! winner-reason family on ONE inventory: Defs first-file across content
    //! dirs, Keyed last-file/in-file-first, DefInjected SetOrAdd, patch
    //! application, an IfModActive conditional dir and a foreign target pack.
    use super::*;
    use rimloc_domain::canonical::{ContextRole, SourceEntry};

    const FIXTURE: &str = "test/ProvenanceMod";

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(FIXTURE)
            .canonicalize()
            .expect("fixture exists")
    }

    fn entry<'a>(p: &'a Project, key: &str) -> &'a SourceEntry {
        p.entries
            .iter()
            .find(|e| e.id.key == key)
            .unwrap_or_else(|| {
                panic!(
                    "entry {key} missing; have {:?}",
                    p.entries.iter().map(|e| &e.id.key).collect::<Vec<_>>()
                )
            })
    }

    /// Per-entry winner reasons: the effective occurrence must say WHY it won
    /// (family rule that decided it), and Defs entries must carry the REAL
    /// source file — not the virtual DefInjected output path. The fixture's
    /// IfModActive dir resolves against an active-mod context naming Odyssey
    /// (its LoadFolders condition); without that context the content is
    /// honestly absent — see the exactness test below.
    #[test]
    fn per_entry_winner_reasons_and_real_source_file() {
        let root = fixture_path();
        let odyssey = crate::modview::ActiveModContext::from_package_ids(vec![
            "ludeon.rimworld.odyssey".into(),
        ]);
        let p = build_project(&root, Some("1.6"), Some(&odyssey)).unwrap();

        // Defs duplicate identity across content dirs: the FIRST registration
        // wins (LoadFolders li order: root before 1.6) — not last-file.
        let dup = entry(&p, "Dup.label");
        assert_eq!(dup.text, "root-wins", "{:?}", dup.contexts);
        assert_eq!(
            dup.provenance.selected_by.as_deref(),
            Some("first-file-wins")
        );
        let ctx = &dup.contexts[0];
        assert_eq!(ctx.role, ContextRole::Effective);
        // `file` is a display String with the native separator — normalize
        // before matching the fixture-relative suffix (Windows `\`).
        assert!(
            ctx.file.replace('\\', "/").ends_with("Defs/A_Root.xml"),
            "real effective source file expected, got {}",
            ctx.file
        );
        assert!(
            !ctx.file.contains("DefInjected"),
            "source context must not point at the virtual output path: {}",
            ctx.file
        );
        assert!(ctx.line.is_some(), "parser-guaranteed line must survive");
        assert_eq!(ctx.def_type.as_deref(), Some("ThingDef"));

        // Keyed cross-file duplicate: last loaded file wins.
        let g = entry(&p, "Greeting");
        assert_eq!(g.text, "last-file");
        assert_eq!(g.provenance.selected_by.as_deref(), Some("keyed-last-wins"));

        // Keyed same-file duplicate: the FIRST value wins, the duplicate is
        // kept as an Overridden context (mandate §14: primary + other usages).
        let dupk = entry(&p, "DupKey");
        assert_eq!(dupk.text, "one");
        assert_eq!(
            dupk.provenance.selected_by.as_deref(),
            Some("keyed-first-in-file")
        );
        assert_eq!(dupk.contexts.len(), 2);
        assert_eq!(dupk.contexts[0].role, ContextRole::Effective);
        assert_eq!(dupk.contexts[1].role, ContextRole::Overridden);
        assert_eq!(dupk.contexts[1].file, dupk.contexts[0].file);

        // DefInjected sidecar SetOrAdd: the last file overwrites.
        let sidecar = entry(&p, "Solo.label");
        assert_eq!(sidecar.text, "sidecar-z");
        assert_eq!(
            sidecar.provenance.selected_by.as_deref(),
            Some("definjected-setoradd")
        );

        // Patched content: the patch operation is why this value won.
        let patched = entry(&p, "Patched.label");
        assert_eq!(patched.text, "patched by op");
        assert_eq!(
            patched.provenance.selected_by.as_deref(),
            Some("patch-applied")
        );

        // IfModActive content is marked per-entry, with the resolved version.
        let cond = entry(&p, "CondD.label");
        assert!(cond.provenance.conditional_branch);
        assert_eq!(cond.provenance.version_selected.as_deref(), Some("1.6"));
        assert!(
            cond.contexts[0]
                .file
                .replace('\\', "/")
                .contains("1.6/Cond/Defs/C.xml"),
            "{}",
            cond.contexts[0].file
        );
    }

    /// Exact is honest: UNRESOLVED conditional content and partial patch
    /// coverage downgrade the view; full supported coverage stays Applied
    /// (not the old "Patches dir exists → Partial" guess). With an
    /// active-mod context resolving the fixture's only IfModActive branch,
    /// the view is Exact; without the context the conditional content is
    /// never guessed in — it is absent AND the view stays Potential.
    #[test]
    fn exact_requires_no_unresolved_conditionals_and_full_patch_coverage() {
        let root = fixture_path();
        let odyssey = crate::modview::ActiveModContext::from_package_ids(vec![
            "ludeon.rimworld.odyssey".into(),
        ]);
        let p = build_project(&root, Some("1.6"), Some(&odyssey)).unwrap();
        assert_eq!(p.context.target_version.as_deref(), Some("1.6"));
        // The branch resolved against the context — exact runtime truth.
        assert_eq!(p.context.view, ViewLabel::Exact);
        assert_eq!(
            entry(&p, "Patched.label").provenance.patch_stage,
            PatchStage::Applied,
            "the only patch op is supported and hit its target"
        );
        assert!(entry(&p, "CondD.label").provenance.conditional_branch);

        // Without the context the conditional content is NOT included
        // (no guessing) and the view honestly stays Potential.
        let p2 = build_project(&root, Some("1.6"), None).unwrap();
        assert_eq!(p2.context.view, ViewLabel::Potential);
        assert!(!p2.entries.iter().any(|e| e.id.key == "CondD.label"));
    }

    /// A foreign target pack (Languages/Russian) is TARGET content, never
    /// English source: neither entry text nor a phantom context, and a
    /// key that exists only in the target pack is not a source entry.
    #[test]
    fn foreign_target_pack_is_not_english_source() {
        let root = fixture_path();
        let p = build_project(&root, Some("1.6"), None).unwrap();
        assert!(
            !p.entries.iter().any(|e| e.text == "Привет"),
            "Russian pack text must not enter the EN source inventory"
        );
        assert!(
            !p.entries.iter().any(|e| e.id.key == "OnlyRussian"),
            "a key existing only in the target pack is not English source"
        );
        assert!(
            p.entries.iter().all(|e| {
                e.contexts
                    .iter()
                    .all(|c| !c.file.replace('\\', "/").contains("/Russian/"))
            }),
            "target-pack occurrences must not appear as contexts"
        );
        assert!(p.entries.iter().any(|e| e.text == "last-file"));
        assert!(p.entries.iter().all(|e| e.source_locale == "en"));
    }

    /// Flat mod without LoadFolders/version dirs: nothing was
    /// version-selected, so version_selected stays None even when a game
    /// version is requested; the view is still Exact (no conditionals, no
    /// patches).
    #[test]
    fn flat_mod_records_no_version_selection() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/TKeyMod")
            .canonicalize()
            .unwrap();
        let p = build_project(&root, Some("1.6"), None).unwrap();
        assert_eq!(p.context.view, ViewLabel::Exact);
        assert!(
            p.entries
                .iter()
                .all(|e| e.provenance.version_selected.is_none()),
            "flat mod: no version root was selected"
        );
    }

    /// Unsupported patch op → Partial coverage → POTENTIAL view even with a
    /// known version (patch coverage must come from the real report, not
    /// from directory existence).
    #[test]
    fn partial_patch_coverage_blocks_exact_view() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::write(
            root.join("Defs/T.xml"),
            r#"<Defs><ThingDef><defName>Widget</defName><label>a label</label></ThingDef></Defs>"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("Patches")).unwrap();
        std::fs::write(
            root.join("Patches/P.xml"),
            r#"<Patch><Operation Class="PatchOperationUnknownThing"><xpath>/Defs/ThingDef[defName="Widget"]/label</xpath><value>x</value></Operation></Patch>"#,
        )
        .unwrap();
        let p = build_project(root, Some("1.6"), None).unwrap();
        assert_eq!(
            entry(&p, "Widget.label").provenance.patch_stage,
            PatchStage::Partial
        );
        assert_eq!(p.context.view, ViewLabel::Potential);
    }

    /// LoadFolders version fallback: when the requested game version exceeds
    /// the mod's tags, the RESOLVED version (largest ≤ requested) is what was
    /// selected — provenance must record 1.5, not the requested 1.6.
    #[test]
    fn version_fallback_records_resolved_version() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("1.5/Defs")).unwrap();
        std::fs::write(
            root.join("LoadFolders.xml"),
            "<loadFolders><v1.5><li>1.5</li></v1.5></loadFolders>",
        )
        .unwrap();
        std::fs::write(
            root.join("1.5/Defs/D.xml"),
            r#"<Defs><ThingDef><defName>F</defName><label>fallback label</label></ThingDef></Defs>"#,
        )
        .unwrap();
        let p = build_project(root, Some("1.6"), None).unwrap();
        assert_eq!(p.context.target_version.as_deref(), Some("1.6"));
        assert_eq!(
            entry(&p, "F.label").provenance.version_selected.as_deref(),
            Some("1.5"),
            "provenance must record the resolved 1.5 root, not the requested 1.6"
        );
    }

    /// TKey shares Defs semantics: a duplicate identity in a later content
    /// dir is rejected — one entry, first file's text, first-file reason.
    #[test]
    fn tkey_duplicate_across_content_dirs_first_file_wins() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::create_dir_all(root.join("1.6/Defs")).unwrap();
        std::fs::write(
            root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>/</li><li>1.6</li></v1.6></loadFolders>",
        )
        .unwrap();
        std::fs::write(
            root.join("Defs/Q.xml"),
            r#"<Defs><QuestScriptDef><defName>Sample</defName><label TKey="LetterLabel">root tkey</label></QuestScriptDef></Defs>"#,
        )
        .unwrap();
        std::fs::write(
            root.join("1.6/Defs/Q2.xml"),
            r#"<Defs><QuestScriptDef><defName>Sample</defName><label TKey="LetterLabel">version tkey</label></QuestScriptDef></Defs>"#,
        )
        .unwrap();
        let p = build_project(root, Some("1.6"), None).unwrap();
        let tk = entry(&p, "Sample.LetterLabel");
        assert_eq!(tk.id.kind, EntryKind::TKey);
        assert_eq!(tk.text, "root tkey");
        assert_eq!(tk.contexts.len(), 1, "{:?}", tk.contexts);
        assert_eq!(
            tk.provenance.selected_by.as_deref(),
            Some("first-file-wins")
        );
    }

    /// Mandate §14 (TKey/multi-context): an identity shared by several
    /// same-file nodes keeps Primary location + Other usages. The LAST
    /// field assignment in document order is the effective text; the
    /// winner reason is the actual same-file re-assignment, not
    /// first-file-wins. Locations are parser-guaranteed (captured in the
    /// same parser pass) and survive the project persistence roundtrip.
    #[test]
    fn tkey_same_file_shared_nodes_primary_plus_other_usages() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::write(
            root.join("Defs/Q.xml"),
            r#"<Defs>
  <QuestScriptDef>
    <defName>Sample</defName>
    <label TKey="LetterLabel">first text</label>
    <description TKey="LetterLabel">second text</description>
  </QuestScriptDef>
</Defs>"#,
        )
        .unwrap();
        let p = build_project(root, None, None).unwrap();
        let tk = entry(&p, "Sample.LetterLabel");
        assert_eq!(tk.id.kind, EntryKind::TKey);
        assert_eq!(tk.text, "second text", "last same-file assignment wins");
        assert_eq!(
            tk.provenance.selected_by.as_deref(),
            Some("tkey-last-assignment")
        );
        assert_eq!(tk.tkey.as_ref().unwrap().contexts, 2);
        assert_eq!(tk.tkey.as_ref().unwrap().locations.len(), 2);
        // Primary (effective) leads; the overwritten earlier node follows.
        assert_eq!(tk.contexts.len(), 2, "{:?}", tk.contexts);
        assert_eq!(tk.contexts[0].role, ContextRole::Effective);
        assert_eq!(tk.contexts[1].role, ContextRole::Overridden);
        for c in &tk.contexts {
            assert!(
                c.file.replace('\\', "/").ends_with("Defs/Q.xml"),
                "real source file expected, got {}",
                c.file
            );
            assert!(c.line.is_some(), "parser-guaranteed line required");
            assert_eq!(c.def_type.as_deref(), Some("QuestScriptDef"));
        }
        assert!(
            tk.contexts[0].line > tk.contexts[1].line,
            "effective node must be the LATER one: {:?}",
            tk.contexts
        );
        // Persistence roundtrip keeps primary + other usages.
        let file = dir.path().join("project.rimloc.json");
        crate::project_store::save_project(&p, &file).unwrap();
        let reloaded = crate::project_store::load_project(&file).unwrap();
        let tk2 = entry(&reloaded, "Sample.LetterLabel");
        assert_eq!(tk2.contexts.len(), 2);
        assert_eq!(tk2.contexts[0].role, ContextRole::Effective);
        assert_eq!(tk2.contexts[0].line, tk.contexts[0].line);
        assert_eq!(tk2.contexts[1].role, ContextRole::Overridden);
        assert_eq!(tk2.contexts[1].line, tk.contexts[1].line);
    }

    /// A mod that ships ONLY a target pack has no English source at all —
    /// the canonical project must stay empty rather than label foreign
    /// target text as `en`.
    #[test]
    fn target_only_pack_is_never_english_source() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Languages/Russian/Keyed")).unwrap();
        std::fs::write(
            root.join("Languages/Russian/Keyed/K.xml"),
            "<LanguageData>\n  <Greeting>Привет</Greeting>\n</LanguageData>\n",
        )
        .unwrap();
        let p = build_project(root, Some("1.6"), None).unwrap();
        assert!(
            p.entries.is_empty(),
            "target-only pack must not become source: {:?}",
            p.entries.iter().map(|e| &e.id.key).collect::<Vec<_>>()
        );
    }

    /// TKey-only Defs source + a Russian sidecar: the TKey entry stays, the
    /// foreign pack never leaks into source text or contexts.
    #[test]
    fn tkey_only_defs_with_target_sidecar_has_no_foreign_source() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::create_dir_all(root.join("Languages/Russian/Keyed")).unwrap();
        std::fs::write(
            root.join("Defs/Q.xml"),
            r#"<Defs><QuestScriptDef><defName>Sample</defName><label TKey="LetterLabel">quest text</label></QuestScriptDef></Defs>"#,
        )
        .unwrap();
        std::fs::write(
            root.join("Languages/Russian/Keyed/K.xml"),
            "<LanguageData>\n  <Greeting>Привет</Greeting>\n</LanguageData>\n",
        )
        .unwrap();
        let p = build_project(root, None, None).unwrap();
        assert_eq!(p.entries.len(), 1, "{:?}", p.entries);
        assert_eq!(p.entries[0].id.key, "Sample.LetterLabel");
        assert_eq!(p.entries[0].text, "quest text");
        assert!(p.entries.iter().all(|e| e.source_locale == "en"));
        assert!(p.entries.iter().all(|e| e
            .contexts
            .iter()
            .all(|c| !c.file.replace('\\', "/").contains("/Russian/"))));
    }

    /// Version-only layouts (no LoadFolders.xml): the SAME modview resolver
    /// picks the version dir, so content is version-scoped (never a 1.5+1.6
    /// union) and the selected version is recorded — no false Exact.
    #[test]
    fn version_only_layout_uses_the_version_resolver() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("1.5/Defs")).unwrap();
        std::fs::create_dir_all(root.join("1.6/Defs")).unwrap();
        std::fs::write(
            root.join("1.5/Defs/A.xml"),
            r#"<Defs><ThingDef><defName>F</defName><label>v15 label</label></ThingDef></Defs>"#,
        )
        .unwrap();
        std::fs::write(
            root.join("1.6/Defs/B.xml"),
            r#"<Defs><ThingDef><defName>E</defName><label>v16 label</label></ThingDef></Defs>"#,
        )
        .unwrap();
        // Requested 1.5 → only 1.5 content, selected version recorded.
        let p = build_project(root, Some("1.5"), None).unwrap();
        assert_eq!(entry(&p, "F.label").text, "v15 label");
        assert!(p.entries.iter().all(|e| e.id.key != "E.label"));
        assert_eq!(
            entry(&p, "F.label").provenance.version_selected.as_deref(),
            Some("1.5")
        );
        assert_eq!(p.context.view, ViewLabel::Exact);
        // No request → the highest version dir is selected.
        let p = build_project(root, None, None).unwrap();
        assert_eq!(entry(&p, "E.label").text, "v16 label");
        assert!(p.entries.iter().all(|e| e.id.key != "F.label"));
        assert_eq!(
            entry(&p, "E.label").provenance.version_selected.as_deref(),
            Some("1.6")
        );
    }

    /// Identity fix (approved architectural decision): two DEF TYPES
    /// sharing one `{defName}.{field}` key are TWO distinct entries — each
    /// with its own def-type discriminator, its own text, its own context —
    /// never one collapsed entry with an Overridden fiction.
    #[test]
    fn def_type_key_collision_keeps_both_identities() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::write(
            root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing label</label></ThingDef></Defs>"#,
        )
        .unwrap();
        std::fs::write(
            root.join("Defs/B_Ability.xml"),
            r#"<Defs><AbilityDef><defName>Dup</defName><label>ability label</label></AbilityDef></Defs>"#,
        )
        .unwrap();
        let p = build_project(root, Some("1.6"), None).unwrap();
        let mut matches: Vec<_> = p
            .entries
            .iter()
            .filter(|e| e.id.key == "Dup.label")
            .collect();
        matches.sort_by(|a, b| a.id.def_type.cmp(&b.id.def_type));
        assert_eq!(matches.len(), 2, "both def types survive as entries");
        assert_eq!(matches[0].id.def_type.as_deref(), Some("AbilityDef"));
        assert_eq!(matches[0].text, "ability label");
        assert_eq!(matches[0].contexts.len(), 1);
        assert_eq!(matches[1].id.def_type.as_deref(), Some("ThingDef"));
        assert_eq!(matches[1].text, "thing label");
        assert_eq!(matches[1].contexts.len(), 1);
        assert_ne!(matches[0].id, matches[1].id);
        let expected_file = |def_type: &str| {
            if def_type.eq_ignore_ascii_case("ThingDef") {
                "A_Thing.xml"
            } else {
                "B_Ability.xml"
            }
        };
        for e in &matches {
            assert!(
                e.contexts[0]
                    .file
                    .ends_with(expected_file(e.id.def_type.as_deref().unwrap_or_default())),
                "each identity points at its OWN source file: {:?}",
                e.contexts[0]
            );
        }
    }
}

/// Classification result of a source update (mandate 4 §3).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SourceChangeReport {
    pub reusable: usize,
    pub source_changed: usize,
    pub new_source: usize,
    pub obsolete: usize,
}

#[cfg(test)]
mod gate_k_tests {
    use super::*;
    use rimloc_domain::canonical::{
        EntryKind, Origin, Project, SourceEntry, SourceEntryId, SourceProvenance,
    };

    /// The OLD project's entries must mirror the identities the fresh scan
    /// produces (DefInjected, ThingDef scope) so the comparison is between
    /// like identities — the identity fix made cross-kind/cross-type key
    /// matches distinct.
    fn entry(key: &str, text: &str) -> SourceEntry {
        SourceEntry {
            id: SourceEntryId {
                kind: EntryKind::DefInjected,
                key: key.into(),
                def_type: Some("ThingDef".into()),
            },
            text: text.into(),
            source_locale: "en".into(),
            contexts: vec![],
            provenance: SourceProvenance::default(),
            tkey: None,
            source_ref: None,
        }
    }

    fn id_of(key: &str) -> SourceEntryId {
        SourceEntryId {
            kind: EntryKind::DefInjected,
            key: key.into(),
            def_type: Some("ThingDef".into()),
        }
    }

    /// A source text change flags the translation for review; unchanged work
    /// stays reusable; a vanished identity becomes obsolete; nothing is lost.
    #[test]
    fn source_update_classifies_translations() {
        let updated = tempfile::tempdir().unwrap();
        let defs = updated.path().join("Defs");
        std::fs::create_dir_all(&defs).unwrap();
        std::fs::write(
            defs.join("K.xml"),
            r#"<Defs><ThingDef><defName>A</defName><label>CHANGED text</label></ThingDef>
  <ThingDef><defName>C</defName><label>brand new</label></ThingDef>
  <ThingDef><defName>D</defName><label>untouched</label></ThingDef></Defs>"#,
        )
        .unwrap();
        // Disable patches dir interference: none exists.

        let mut p = Project::default();
        // A: translated, source changed now.
        p.entries.push(entry("A.label", "old text"));
        // B: translated, source vanished.
        p.entries.push(entry("B.label", "gone source"));
        // D: translated, source unchanged.
        p.entries.push(entry("D.label", "untouched"));

        p.update_translation(
            id_of("A.label"),
            "Russian",
            Some("старый перевод".into()),
            Origin::Human,
        );
        p.update_translation(
            id_of("B.label"),
            "Russian",
            Some("перевод осиротел".into()),
            Origin::Human,
        );
        p.update_translation(
            id_of("D.label"),
            "Russian",
            Some("не трогать".into()),
            Origin::Human,
        );

        let report = detect_source_changes(&mut p, updated.path(), None).unwrap();
        assert_eq!(report.source_changed, 1, "{report:?}");
        assert_eq!(report.new_source, 1, "{report:?}");
        assert_eq!(report.obsolete, 1, "{report:?}");
        assert!(report.reusable >= 1, "{report:?}");

        let a = p.translation(&id_of("A.label"), "Russian").unwrap();
        assert!(a.source_changed.is_some());
        assert_eq!(a.review, dom::Review::Pending);
        assert_eq!(
            p.translation(&id_of("B.label"), "Russian")
                .unwrap()
                .lifecycle,
            dom::Lifecycle::Obsolete
        );
        // Old translation text preserved even though source changed.
        assert_eq!(a.text.as_deref(), Some("старый перевод"));
    }
}
