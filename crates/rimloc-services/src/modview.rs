//! Effective mod view (§B): model the content folders RimWorld would actually
//! load for a game version, including LoadFolders.xml semantics.
//!
//! RimWorld's rules (verified against 1.6 loadFolders usage in the wild and
//! GAME_SOURCE_FINDINGS.md): `<loadFolders><v1.6><li>/</li><li>1.6</li>
//! <li IfModActive="pkg">1.6/Mods/X</li></v1.6></loadFolders>`. `/` means the
//! mod root; `IfModActive` entries are game-state dependent: they resolve
//! against the ACTIVE MOD list ([`ActiveModContext`], RimTransAI parity).
//! Without that context the conditional dirs are NOT guessed into the view —
//! they are reported as unresolved and the view stays honestly POTENTIAL.

use std::path::{Path, PathBuf};

use crate::Result;

/// Which game-state condition guarded a conditional LoadFolders `<li>` entry.
/// RimWorld 1.6 semantics (decompile, LoadFoldersModDef): attribute value is a
/// COMMA-SEPARATED packageId list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalKind {
    /// `IfModActive="a,b"` — loads when AT LEAST ONE listed package is active.
    IfModActive,
    /// `IfModActiveAll="a,b"` — loads when ALL listed packages are active.
    IfModActiveAll,
    /// `IfModNotActive="a,b"` — loads when NONE of the listed packages is active.
    IfModNotActive,
}

impl ConditionalKind {
    /// The attribute name as written in LoadFolders.xml.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionalKind::IfModActive => "IfModActive",
            ConditionalKind::IfModActiveAll => "IfModActiveAll",
            ConditionalKind::IfModNotActive => "IfModNotActive",
        }
    }
}

/// One conditional `<li>` entry as written in LoadFolders.xml: the guard kind,
/// the packageIds it names, and the folder it would load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalEntry {
    pub kind: ConditionalKind,
    /// packageIds named by the attribute, in written order, whitespace-trimmed.
    pub packages: Vec<String>,
    /// The `<li>` text — the content folder relative to the mod root (`/` = root).
    pub folder: String,
}

impl ConditionalEntry {
    /// RimWorld's load decision for this entry under the given active-mod
    /// context. packageIds compare CASE-INSENSITIVELY: the game normalizes
    /// packageId to lowercase on parse (ModMetaData.PackageId).
    pub fn is_satisfied(&self, ctx: &ActiveModContext) -> bool {
        let active = |pkg: &str| ctx.is_active(pkg);
        match self.kind {
            ConditionalKind::IfModActive => self.packages.iter().any(|p| active(p)),
            ConditionalKind::IfModActiveAll => self.packages.iter().all(|p| active(p)),
            ConditionalKind::IfModNotActive => !self.packages.iter().any(|p| active(p)),
        }
    }

    /// Human-readable rendering for diagnostics (`IfModActive(a|b)`).
    pub fn display_condition(&self) -> String {
        format!("{}({})", self.kind.as_str(), self.packages.join("|"))
    }
}

/// The player's active-mod state — the context RimWorld resolves conditional
/// LoadFolders branches against (RimTransAI `ActivePackageIds` parity).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveModContext {
    /// packageIds of the active mods (Workshop + local), e.g.
    /// `ludeon.rimworld.royalty`, `grassypefire.medieval.backstoriespatch`.
    pub active_package_ids: Vec<String>,
    /// packageIds of the active DLC. DLC are mods too (`Ludeon.RimWorld.Royalty`);
    /// kept as a separate input so callers can pass them as reported by the game.
    pub active_dlc: Vec<String>,
}

impl ActiveModContext {
    /// Context from a flat active-mod list (the CLI `--active-mods` spelling).
    pub fn from_package_ids(ids: Vec<String>) -> Self {
        Self {
            active_package_ids: ids,
            active_dlc: Vec::new(),
        }
    }

    /// True when `package_id` names an active mod or DLC (case-insensitive,
    /// whitespace-trimmed; the game's packageId comparison is lowercase).
    pub fn is_active(&self, package_id: &str) -> bool {
        let want = package_id.trim().to_ascii_lowercase();
        if want.is_empty() {
            return false;
        }
        self.active_package_ids
            .iter()
            .chain(self.active_dlc.iter())
            .any(|have| have.trim().eq_ignore_ascii_case(&want))
    }

    pub fn is_empty(&self) -> bool {
        self.active_package_ids.is_empty() && self.active_dlc.is_empty()
    }
}

/// Typed resolution state of the conditional entries of the selected version.
/// The view NEVER guesses: either the conditions were evaluated against a
/// provided context, or the conditional dirs stay OUT of the view and the
/// state says POTENTIAL (no fabricated inclusion, no silent drop).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConditionalState {
    /// The selected version carries no conditional entries at all.
    #[default]
    None,
    /// An active-mod context was provided: every conditional entry was
    /// evaluated — `included` dirs satisfied their condition (and exist on
    /// disk), `excluded` failed theirs.
    Resolved { included: usize, excluded: usize },
    /// No active-mod context: the conditional entries were not guessed —
    /// their dirs are excluded from the view and listed in
    /// [`EffectiveModView::unresolved_conditionals`]. The view is POTENTIAL.
    Unresolved { entries: usize },
}

impl ConditionalState {
    /// True when conditional content exists but was not resolved (no context).
    pub fn is_unresolved(&self) -> bool {
        matches!(self, ConditionalState::Unresolved { .. })
    }
}

#[derive(Debug, Clone, Default)]
pub struct EffectiveModView {
    /// Resolved game version ("1.6"), when determinable.
    pub version: Option<String>,
    /// Folders whose content the game loads unconditionally for this version.
    pub content_dirs: Vec<PathBuf>,
    /// Conditional folders whose condition RESOLVED to true (active-mod
    /// context provided and satisfied, and the dir exists). Without a context
    /// this is always empty — see [`Self::conditional_state`].
    pub conditional_dirs: Vec<PathBuf>,
    /// Typed state of the conditional entries for this version (resolved
    /// against the context, or unresolved-potential without one).
    pub conditional_state: ConditionalState,
    /// Conditional entries NOT loaded because no active-mod context was
    /// provided. Diagnostic evidence only — never scanned, never guessed in.
    pub unresolved_conditionals: Vec<ConditionalEntry>,
    /// Translations live at the mod root for every layout style.
    pub languages_dir: PathBuf,
    /// RimWorld collects Languages/ from EVERY content folder (root, Common/,
    /// 1.6/, IfModActive dirs — verified in 1.6 decompile). Existing RU packs
    /// shipping 1.6/Languages/Russian are invisible to a root-only lookup.
    pub languages_dirs: Vec<PathBuf>,
}

impl EffectiveModView {
    /// All Defs roots the extractor should consider.
    pub fn defs_roots(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for dir in self.content_dirs.iter().chain(self.conditional_dirs.iter()) {
            let d = dir.join("Defs");
            if d.is_dir() {
                out.push(d);
            }
        }
        out
    }

    /// All Languages dirs the game would read (root + each content folder).
    pub fn languages_dirs(&self) -> Vec<PathBuf> {
        let mut out = vec![self.languages_dir.clone()];
        for dir in self.content_dirs.iter().chain(self.conditional_dirs.iter()) {
            let d = dir.join("Languages");
            if d.is_dir() && !out.contains(&d) {
                out.push(d);
            }
        }
        out
    }
}

/// Latest `v1.x` tag inside LoadFolders.xml, compared component-wise.
fn latest_version_tag(tags: &[String]) -> Option<String> {
    tags.iter()
        .max_by_key(|t| {
            t.trim_start_matches('v')
                .split('.')
                .filter_map(|p| p.parse::<u64>().ok())
                .collect::<Vec<_>>()
        })
        .cloned()
}

/// A game version string with the optional `v`/`V` prefix stripped and
/// lowercased (`"v1.6"` → `"1.6"`). Both flag spellings are documented
/// (`--game-version 1.6 | v1.6`); the LoadFolders resolver used to build
/// `vv1.6` from the prefixed form and silently fell back to the root view.
pub fn normalize_version_str(raw: &str) -> String {
    raw.trim().trim_start_matches(['v', 'V']).to_lowercase()
}

/// Component-wise version key for ordering (`"1.6.1"` → `[1, 6, 1]`).
fn version_key(v: &str) -> Vec<u64> {
    v.split('.').filter_map(|p| p.parse::<u64>().ok()).collect()
}

/// Classic (LoadFolders-less) version directories of a mod root, ASCENDING by
/// version: `(normalized version, dir path)`. Names may carry a `v`/`V`
/// prefix (`v1.4`); non-numeric or dot-less names never match.
pub fn classic_version_dirs(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = root
        .read_dir()
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|e| {
                    let name = e.file_name().to_str()?.to_string();
                    let normalized = normalize_version_str(&name);
                    if normalized.contains('.')
                        && normalized.split('.').all(|p| p.parse::<u64>().is_ok())
                    {
                        Some((normalized, root.join(&name)))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|(v, _)| version_key(v));
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// The classic-layout content folder for a requested game version: an exact
/// match, else the LARGEST version ≤ requested (the game's fallback), else
/// none. Without a request the newest available version is the stand-in for
/// the running game version.
fn pick_classic_version(tiers: &[(String, PathBuf)], requested: Option<&str>) -> Option<String> {
    match requested {
        Some(req) => {
            let want = normalize_version_str(req);
            let req_parts = version_key(&want);
            if let Some((v, _)) = tiers.iter().find(|(v, _)| *v == want) {
                return Some(v.clone());
            }
            tiers
                .iter()
                .filter(|(v, _)| version_key(v) <= req_parts)
                .max_by_key(|(v, _)| version_key(v))
                .map(|(v, _)| v.clone())
        }
        None => tiers.last().map(|(v, _)| v.clone()),
    }
}

/// Parse LoadFolders.xml (BOM-tolerant) and return version tag → folder list.
/// Conditional entries (`IfModActive*`/`IfModNotActive`) are returned as typed
/// [`ConditionalEntry`] values, separate from plain ones.
/// One LoadFolders version tag: (tag, unconditional dirs, conditional entries).
type VersionFolders = (String, Vec<String>, Vec<ConditionalEntry>);

fn parse_load_folders(xml: &str) -> Option<Vec<VersionFolders>> {
    let cleaned = xml.trim_start_matches('\u{feff}');
    let doc = roxmltree::Document::parse(cleaned).ok()?;
    let root = doc.root_element();
    if root.tag_name().name() != "loadFolders" {
        return None;
    }
    let mut out = Vec::new();
    for ver in root.children().filter(|n| n.is_element()) {
        // The game reads version tags case-insensitively (<V1.6> occurs in the wild).
        let tag = ver.tag_name().name().to_lowercase();
        if !tag.starts_with('v') {
            continue;
        }
        let mut plain = Vec::new();
        let mut conditional = Vec::new();
        for li in ver
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "li")
        {
            let Some(text) = li.text().map(str::trim).filter(|t| !t.is_empty()) else {
                continue;
            };
            // Attribute value: a comma-separated packageId list (game 1.6
            // semantics). Kept in written order/case; matching lowercases.
            let cond = li
                .attribute("IfModActive")
                .map(|pkgs| ConditionalEntry {
                    kind: ConditionalKind::IfModActive,
                    packages: split_package_ids(pkgs),
                    folder: text.to_string(),
                })
                .or_else(|| {
                    li.attribute("IfModActiveAll").map(|pkgs| ConditionalEntry {
                        kind: ConditionalKind::IfModActiveAll,
                        packages: split_package_ids(pkgs),
                        folder: text.to_string(),
                    })
                })
                .or_else(|| {
                    li.attribute("IfModNotActive").map(|pkgs| ConditionalEntry {
                        kind: ConditionalKind::IfModNotActive,
                        packages: split_package_ids(pkgs),
                        folder: text.to_string(),
                    })
                });
            match cond {
                Some(entry) => conditional.push(entry),
                None => plain.push(text.to_string()),
            }
        }
        out.push((tag, plain, conditional));
    }
    Some(out)
}

/// Split an IfModActive* attribute value into trimmed packageIds
/// (`"a.b, c.d "` → `["a.b", "c.d"]`); empty items are dropped.
fn split_package_ids(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn resolve_version_from_tags(tags: &[String], requested: Option<&str>) -> Option<String> {
    // Tags are normalized to lowercase at parse time; the request accepts
    // both documented spellings ("1.6" and "v1.6").
    if let Some(req) = requested {
        let req = normalize_version_str(req);
        let want = format!("v{req}");
        if let Some(t) = tags.iter().find(|t| **t == want) {
            return Some(t.trim_start_matches('v').to_string());
        }
        // Game fallback: the largest tag <= the requested version.
        let req_parts = version_key(&req);
        return tags
            .iter()
            .filter(|t| {
                let parts: Vec<u64> = t
                    .trim_start_matches('v')
                    .split('.')
                    .filter_map(|p| p.parse().ok())
                    .collect();
                parts <= req_parts
            })
            .max_by_key(|t| {
                t.trim_start_matches('v')
                    .split('.')
                    .filter_map(|p| p.parse::<u64>().ok())
                    .collect::<Vec<_>>()
            })
            .map(|t| t.trim_start_matches('v').to_string());
    }
    latest_version_tag(tags).map(|t| t.trim_start_matches('v').to_string())
}

/// Build the effective view for a mod root. Falls back to the classic
/// version-directory layout when LoadFolders.xml is absent.
///
/// Conditional (`IfModActive*`/`IfModNotActive`) entries resolve against
/// `context` (RimTransAI parity): satisfied conditions include their dir,
/// failed conditions exclude it. `context == None` NEVER guesses: the
/// conditional dirs stay out of the view, every entry is reported in
/// [`EffectiveModView::unresolved_conditionals`], and the state is
/// [`ConditionalState::Unresolved`] — the caller surfaces a POTENTIAL view.
pub fn effective_view(
    root: &Path,
    requested: Option<&str>,
    context: Option<&ActiveModContext>,
) -> Result<EffectiveModView> {
    let languages_dir = root.join("Languages");

    if let Ok(text) = std::fs::read_to_string(root.join("LoadFolders.xml")) {
        if let Some(folders) = parse_load_folders(&text) {
            let tags: Vec<String> = folders.iter().map(|(t, _, _)| t.clone()).collect();
            let Some(version) = resolve_version_from_tags(&tags, requested) else {
                return Ok(EffectiveModView {
                    version: None,
                    content_dirs: vec![root.to_path_buf()],
                    conditional_dirs: Vec::new(),
                    conditional_state: ConditionalState::None,
                    unresolved_conditionals: Vec::new(),
                    languages_dirs: vec![languages_dir.clone()],
                    languages_dir,
                });
            };
            let vtag = format!("v{version}");
            let Some((_, plain, conditional)) = folders.iter().find(|(t, _, _)| *t == vtag) else {
                return Ok(EffectiveModView {
                    version: None,
                    content_dirs: vec![root.to_path_buf()],
                    conditional_dirs: Vec::new(),
                    conditional_state: ConditionalState::None,
                    unresolved_conditionals: Vec::new(),
                    languages_dirs: vec![languages_dir.clone()],
                    languages_dir,
                });
            };
            let mut content_dirs = Vec::new();
            for entry in plain {
                let dir = if entry == "/" {
                    root.to_path_buf()
                } else {
                    // H2: a `<li>` entry is a CONTENT SCOPE, and the join
                    // with an absolute string replaces the whole prefix —
                    // a Workshop mod must not make RimLoc scan arbitrary
                    // directories. Anything resolving outside the root is
                    // a typed refusal (real symlink views included).
                    let dir = root.join(entry);
                    if !crate::is_within(&dir, root) {
                        color_eyre::eyre::bail!(
                            "LoadFolders.xml entry `{entry}` resolves outside the mod root `{}`: external content folders are not scanned; keep all content inside the mod",
                            root.display()
                        );
                    }
                    dir
                };
                if dir.is_dir() {
                    content_dirs.push(dir);
                }
            }
            if content_dirs.is_empty() {
                content_dirs.push(root.to_path_buf());
            }
            let mut conditional_dirs = Vec::new();
            let mut unresolved_conditionals = Vec::new();
            let mut included = 0usize;
            let mut excluded = 0usize;
            for entry in conditional {
                let Some(ctx) = context else {
                    // No active-mod context: do NOT guess — the entry is
                    // reported as unresolved and its dir stays unscanned.
                    unresolved_conditionals.push(entry.clone());
                    continue;
                };
                if !entry.is_satisfied(ctx) {
                    excluded += 1;
                    continue;
                }
                included += 1;
                let dir = if entry.folder == "/" {
                    root.to_path_buf()
                } else {
                    let dir = root.join(&entry.folder);
                    if !crate::is_within(&dir, root) {
                        color_eyre::eyre::bail!(
                            "LoadFolders.xml (IfModActive) entry `{}` resolves outside the mod root `{}`: external content folders are not scanned; keep all content inside the mod",
                            entry.folder,
                            root.display()
                        );
                    }
                    dir
                };
                if dir.is_dir() {
                    conditional_dirs.push(dir);
                }
            }
            let conditional_state = if context.is_some() {
                ConditionalState::Resolved { included, excluded }
            } else {
                ConditionalState::Unresolved {
                    entries: unresolved_conditionals.len(),
                }
            };
            let mut languages_dirs = vec![languages_dir.clone()];
            for dir in content_dirs.iter().chain(conditional_dirs.iter()) {
                let d = dir.join("Languages");
                if d.is_dir() && !languages_dirs.contains(&d) {
                    languages_dirs.push(d);
                }
            }
            return Ok(EffectiveModView {
                version: Some(version),
                content_dirs,
                conditional_dirs,
                conditional_state,
                unresolved_conditionals,
                languages_dir,
                languages_dirs,
            });
        }
    }

    // Classic layout (no LoadFolders.xml): the game's conventional fallback
    // (GAME_SOURCE_FINDINGS §1.3/§1.4, verified on the 1.6 decompile) —
    // `RootDir/<newest version dir ≤ current>` plus `RootDir/Common` plus the
    // mod root ALWAYS. The folders SUM, never "or": version-specific content
    // overrides duplicate registrations by load order (specific above
    // common above root), which is exactly the view order `content_dirs`
    // carries here (first entry = highest priority for first-registration
    // rules).
    let requested = requested.map(normalize_version_str);
    let requested = requested.as_deref();
    let tiers = classic_version_dirs(root);
    let chosen = pick_classic_version(&tiers, requested);
    let mut content_dirs: Vec<PathBuf> = Vec::new();
    if let Some(v) = &chosen {
        if let Some((_, dir)) = tiers.iter().find(|(name, _)| name == v) {
            content_dirs.push(dir.clone());
        }
    }
    let common = root.join("Common");
    if common.is_dir() {
        content_dirs.push(common);
    }
    // The root always loads, even when a version folder exists.
    content_dirs.push(root.to_path_buf());
    let mut languages_dirs = vec![languages_dir.clone()];
    for dir in &content_dirs {
        let d = dir.join("Languages");
        if d.is_dir() && !languages_dirs.contains(&d) {
            languages_dirs.push(d);
        }
    }
    Ok(EffectiveModView {
        version: chosen,
        content_dirs,
        conditional_dirs: Vec::new(),
        // The classic layout has no conditional entries by construction.
        conditional_state: ConditionalState::None,
        unresolved_conditionals: Vec::new(),
        languages_dir,
        languages_dirs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn load_folders_selects_version_and_separates_conditional() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("LoadFolders.xml"),
            "\u{feff}<loadFolders><v1.4><li>/</li><li>1.4</li></v1.4>\
             <v1.6><li>/</li><li>1.6</li>\
             <li IfModActive=\"Ludeon.RimWorld.Odyssey\">1.6/Mods/Odyssey</li></v1.6></loadFolders>",
        );
        write(&root.join("1.6/Defs/X.xml"), "<Defs/>");
        write(&root.join("1.6/Mods/Odyssey/Defs/Y.xml"), "<Defs/>");

        // Without an active-mod context the conditional dir is NOT guessed in.
        let view = effective_view(root, None, None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert!(view.content_dirs.contains(&root.join("1.6")));
        assert!(view.conditional_dirs.is_empty());
        assert_eq!(
            view.conditional_state,
            ConditionalState::Unresolved { entries: 1 }
        );
        assert_eq!(view.unresolved_conditionals.len(), 1);
        assert_eq!(
            view.unresolved_conditionals[0].display_condition(),
            "IfModActive(Ludeon.RimWorld.Odyssey)"
        );
        assert_eq!(view.defs_roots(), vec![root.join("1.6/Defs")]);

        // With the context naming the dependency, the branch resolves true
        // (RimTransAI parity) and the conditional content joins the view.
        let ctx = ActiveModContext::from_package_ids(vec!["Ludeon.RimWorld.Odyssey".into()]);
        let view = effective_view(root, None, Some(&ctx)).unwrap();
        assert_eq!(view.conditional_dirs, vec![root.join("1.6/Mods/Odyssey")]);
        assert_eq!(
            view.conditional_state,
            ConditionalState::Resolved {
                included: 1,
                excluded: 0
            }
        );
        assert!(view.unresolved_conditionals.is_empty());
        assert_eq!(
            view.defs_roots(),
            vec![root.join("1.6/Defs"), root.join("1.6/Mods/Odyssey/Defs")]
        );
        assert_eq!(view.languages_dir, root.join("Languages"));
    }

    /// IfModActive resolution semantics (RimWorld 1.6): any-of / all-of /
    /// none-of, packageIds compared case-insensitively, comma-separated lists.
    #[test]
    fn conditional_entries_resolve_against_active_mod_context() {
        let satisfied = |entry: &ConditionalEntry, ctx: &ActiveModContext| entry.is_satisfied(ctx);
        let entry = |kind: ConditionalKind, pkgs: &[&str]| ConditionalEntry {
            kind,
            packages: pkgs.iter().map(|p| p.to_string()).collect(),
            folder: "X".into(),
        };
        let ctx = ActiveModContext {
            active_package_ids: vec!["some.mod".into()],
            active_dlc: vec!["Ludeon.RimWorld.Royalty".into()],
        };

        // any-of: at least one active (DLC list participates too).
        assert!(satisfied(
            &entry(
                ConditionalKind::IfModActive,
                &["other.mod", "LUDEON.RIMWORLD.ROYALTY"]
            ),
            &ctx
        ));
        assert!(!satisfied(
            &entry(ConditionalKind::IfModActive, &["other.mod"]),
            &ctx
        ));
        // all-of: every listed package must be active.
        assert!(satisfied(
            &entry(
                ConditionalKind::IfModActiveAll,
                &["Some.Mod", "ludeon.rimworld.royalty"]
            ),
            &ctx
        ));
        assert!(!satisfied(
            &entry(
                ConditionalKind::IfModActiveAll,
                &["some.mod", "missing.mod"]
            ),
            &ctx
        ));
        // none-of: true only while every listed package is inactive.
        assert!(satisfied(
            &entry(ConditionalKind::IfModNotActive, &["missing.mod"]),
            &ctx
        ));
        assert!(!satisfied(
            &entry(
                ConditionalKind::IfModNotActive,
                &["missing.mod", "some.mod"]
            ),
            &ctx
        ));
    }

    /// View-level resolution of all three conditional kinds against a
    /// context: satisfied dirs join `conditional_dirs`, failed ones are
    /// counted as excluded and never scanned.
    #[test]
    fn effective_view_resolves_all_conditional_kinds() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.6>\
             <li>Common</li>\
             <li IfModActive=\"ludeon.rimworld.royalty\">AnyDir</li>\
             <li IfModActiveAll=\"a.b,c.d\">AllDir</li>\
             <li IfModNotActive=\"c.d\">NotDir</li>\
             </v1.6></loadFolders>",
        );
        for dir in ["Common", "AnyDir", "AllDir", "NotDir"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }

        // Royalty active, a.b active, c.d NOT active:
        // AnyDir: any-of Royalty → in; AllDir: all-of a.b+c.d → out;
        // NotDir: none-of c.d → in.
        let ctx = ActiveModContext {
            active_package_ids: vec!["a.b".into()],
            active_dlc: vec!["Ludeon.RimWorld.Royalty".into()],
        };
        let view = effective_view(root, Some("1.6"), Some(&ctx)).unwrap();
        assert_eq!(
            view.conditional_dirs,
            vec![root.join("AnyDir"), root.join("NotDir")]
        );
        assert_eq!(
            view.conditional_state,
            ConditionalState::Resolved {
                included: 2,
                excluded: 1
            }
        );
        assert!(view.content_dirs.contains(&root.join("Common")));

        // Activating c.d flips AllDir in and NotDir out.
        let ctx = ActiveModContext::from_package_ids(vec!["a.b".into(), "c.d".into()]);
        let view = effective_view(root, Some("1.6"), Some(&ctx)).unwrap();
        assert_eq!(view.conditional_dirs, vec![root.join("AllDir")]);
        assert_eq!(
            view.conditional_state,
            ConditionalState::Resolved {
                included: 1,
                excluded: 2
            }
        );

        // Empty context: nothing active — any-of/all-of fail, but the
        // none-of guard (IfModNotActive c.d) is satisfied and NotDir loads.
        let ctx = ActiveModContext::default();
        let view = effective_view(root, Some("1.6"), Some(&ctx)).unwrap();
        assert_eq!(view.conditional_dirs, vec![root.join("NotDir")]);
        assert_eq!(
            view.conditional_state,
            ConditionalState::Resolved {
                included: 1,
                excluded: 2
            }
        );
    }

    #[test]
    fn requested_version_must_exist_in_load_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.5><li>1.5</li></v1.5></loadFolders>",
        );
        let view = effective_view(root, Some("1.5"), None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.5"));
        // Unknown version → no tag resolved → root fallback (not an error).
        let view = effective_view(root, Some("9.9"), None).unwrap();
        assert_eq!(view.content_dirs, vec![root.to_path_buf()]);
    }

    #[test]
    fn classic_layout_without_load_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("1.4")).unwrap();
        std::fs::create_dir_all(root.join("1.6")).unwrap();
        let view = effective_view(root, None, None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        // Game rule (§1.3): the version folder loads FIRST (highest
        // priority), the root ALWAYS loads with it — they sum, never "or".
        assert_eq!(
            view.content_dirs,
            vec![root.join("1.6"), root.to_path_buf()]
        );
    }

    /// The classic view follows the game fallback (§1.3): exact match first,
    /// else the largest version ≤ requested, else NO version folder (root
    /// only) — never a newer folder than the running game version.
    #[test]
    fn classic_version_fallback_largest_le_requested() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for name in ["1.4", "1.6"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        // Exact request.
        let view = effective_view(root, Some("1.4"), None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.4"));
        assert_eq!(view.content_dirs[0], root.join("1.4"));
        // Fallback: game 1.5 on a mod shipping 1.4/1.6 → 1.4 content.
        let view = effective_view(root, Some("1.5"), None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.4"));
        assert_eq!(view.content_dirs[0], root.join("1.4"));
        // A version lower than everything → no version folder at all.
        let view = effective_view(root, Some("1.0"), None).unwrap();
        assert_eq!(view.version.as_deref(), None);
        assert_eq!(view.content_dirs, vec![root.to_path_buf()]);
        // The prefixed flag spelling resolves like the bare one.
        let view = effective_view(root, Some("v1.4"), None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.4"));
    }

    /// `Common/` and `v`-prefixed classic folders follow the game convention:
    /// view order is version → Common → root.
    #[test]
    fn classic_view_orders_version_common_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for name in ["1.4", "v1.6", "Common"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        let view = effective_view(root, None, None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert_eq!(
            view.content_dirs,
            vec![root.join("v1.6"), root.join("Common"), root.to_path_buf()]
        );
        // Languages dirs follow the same priority (root's own Languages is
        // always the base).
        std::fs::create_dir_all(root.join("v1.6/Languages")).unwrap();
        std::fs::create_dir_all(root.join("Common/Languages")).unwrap();
        let dirs = view.languages_dirs();
        assert_eq!(
            dirs,
            vec![
                root.join("Languages"),
                root.join("v1.6/Languages"),
                root.join("Common/Languages"),
            ]
        );
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn case_insensitive_version_tags_and_conditional_variants() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><V1.5><li>Old</li></V1.5><v1.6>\
             <li>Common</li>\
             <li IfModActiveAll=\"a.b\">AllDir</li>\
             <li IfModNotActive=\"c.d\">NotDir</li>\
             </v1.6></loadFolders>",
        );
        std::fs::create_dir_all(root.join("Common")).unwrap();
        std::fs::create_dir_all(root.join("AllDir")).unwrap();
        std::fs::create_dir_all(root.join("NotDir")).unwrap();

        // No context: conditional dirs are not guessed into the view.
        let view = effective_view(root, Some("1.6"), None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert!(view.content_dirs.contains(&root.join("Common")));
        assert!(view.conditional_dirs.is_empty());
        assert_eq!(
            view.conditional_state,
            ConditionalState::Unresolved { entries: 2 }
        );

        // a.b active, c.d inactive: AllDir (all-of) and NotDir (none-of) both load.
        let ctx = ActiveModContext::from_package_ids(vec!["a.b".into()]);
        let view = effective_view(root, Some("1.6"), Some(&ctx)).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert!(view.content_dirs.contains(&root.join("Common")));
        assert_eq!(
            view.conditional_dirs,
            vec![root.join("AllDir"), root.join("NotDir")]
        );
        assert_eq!(
            view.conditional_state,
            ConditionalState::Resolved {
                included: 2,
                excluded: 0
            }
        );

        // c.d activates: AllDir stays, NotDir (none-of c.d) drops out.
        let ctx = ActiveModContext::from_package_ids(vec!["a.b".into(), "c.d".into()]);
        let view = effective_view(root, Some("1.6"), Some(&ctx)).unwrap();
        assert_eq!(view.conditional_dirs, vec![root.join("AllDir")]);
        // Languages/ inside content dirs is collected too (game rule #1).
        std::fs::create_dir_all(root.join("Common/Languages/Russian")).unwrap();
        assert!(view
            .languages_dirs()
            .contains(&root.join("Common/Languages")));
    }

    #[test]
    fn version_fallback_picks_largest_tag_le_requested() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.4><li>1.4</li></v1.4><v1.5><li>1.5</li></v1.5></loadFolders>",
        );
        std::fs::create_dir_all(root.join("1.5")).unwrap();
        // Game runs 1.6, mod ships up to 1.5 → game loads 1.5 content.
        let view = effective_view(root, Some("1.6"), None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.5"));
        assert_eq!(view.content_dirs, vec![root.join("1.5")]);
    }
}

/// Best-effort corpus identity: the About.xml packageId, if present.
pub fn about_package_id(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("About/About.xml")).ok()?;
    let start = text.find("<packageId>")? + "<packageId>".len();
    let end = start + text[start..].find("</packageId>")?;
    Some(text[start..end].trim().to_string())
}

#[cfg(test)]
mod loadfolders_containment_tests {
    //! H2: LoadFolders `<li>` entries are content scopes — anything that
    //! resolves outside the mod root is a typed refusal.

    use super::*;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn entries_outside_root_are_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside");
        write(
            &outside.join("Languages/English/Keyed/S.xml"),
            "<LanguageData><SecretKey.Outside>LEAKED-KEYED-TEXT-77</SecretKey.Outside></LanguageData>",
        );

        let root = tmp.path().join("escmod");
        write(
            &root.join("LoadFolders.xml"),
            &format!(
                "<loadFolders><v1.6><li>1.6</li><li>{}</li></v1.6></loadFolders>",
                outside.display()
            ),
        );
        write(&root.join("1.6/Defs/X.xml"), "<Defs/>");

        let err = effective_view(&root, Some("1.6"), None).unwrap_err();
        assert!(err.to_string().contains("outside the mod root"), "{err}");

        // Relative traversal is refused the same way.
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>1.6</li><li>../outside</li></v1.6></loadFolders>",
        );
        let err = effective_view(&root, Some("1.6"), None).unwrap_err();
        assert!(err.to_string().contains("outside the mod root"), "{err}");

        // Conditional (IfModActive) entries get the same containment — when
        // their condition resolves true against the active-mod context.
        write(
            &root.join("LoadFolders.xml"),
            &format!(
                "<loadFolders><v1.6><li>1.6</li><li IfModActive=\"X\">{}</li></v1.6></loadFolders>",
                outside.display()
            ),
        );
        // Without a context the entry is unresolved (reported, never
        // resolved to a path — no leak, no refusal).
        let view = effective_view(&root, Some("1.6"), None).unwrap();
        assert_eq!(view.unresolved_conditionals.len(), 1);
        assert!(view.conditional_dirs.is_empty());
        // With the condition satisfied the escape is a typed refusal.
        let ctx = ActiveModContext::from_package_ids(vec!["x".into()]);
        let err = effective_view(&root, Some("1.6"), Some(&ctx)).unwrap_err();
        assert!(err.to_string().contains("outside the mod root"), "{err}");

        // In-root entries still resolve (the guard is scoped to escapes).
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>1.6</li></v1.6></loadFolders>",
        );
        let view = effective_view(&root, Some("1.6"), None).unwrap();
        assert!(view.content_dirs.contains(&root.join("1.6")));
    }
}
