//! Effective mod view (§B): model the content folders RimWorld would actually
//! load for a game version, including LoadFolders.xml semantics.
//!
//! RimWorld's rules (verified against 1.6 loadFolders usage in the wild and
//! GAME_SOURCE_FINDINGS.md): `<loadFolders><v1.6><li>/</li><li>1.6</li>
//! <li IfModActive="pkg">1.6/Mods/X</li></v1.6></loadFolders>`. `/` means the
//! mod root; `IfModActive` entries are game-state dependent and cannot be
//! resolved offline, so they are reported separately.

use std::path::{Path, PathBuf};

use crate::Result;

#[derive(Debug, Clone, Default)]
pub struct EffectiveModView {
    /// Resolved game version ("1.6"), when determinable.
    pub version: Option<String>,
    /// Folders whose content the game loads unconditionally for this version.
    pub content_dirs: Vec<PathBuf>,
    /// `IfModActive` folders: loaded only when the dependency is active.
    /// Offline RimLoc includes them in extraction (superset) unless told not to.
    pub conditional_dirs: Vec<PathBuf>,
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

/// Parse LoadFolders.xml (BOM-tolerant) and return version tag → folder list.
/// Conditional entries (`IfModActive`) are returned separately from plain ones.
/// One LoadFolders version tag: (tag, unconditional dirs, IfModActive dirs).
type VersionFolders = (String, Vec<String>, Vec<String>);

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
            if li.has_attribute("IfModActive")
                || li.has_attribute("IfModActiveAll")
                || li.has_attribute("IfModNotActive")
            {
                conditional.push(text.to_string());
            } else {
                plain.push(text.to_string());
            }
        }
        out.push((tag, plain, conditional));
    }
    Some(out)
}

fn resolve_version_from_tags(tags: &[String], requested: Option<&str>) -> Option<String> {
    // Tags are normalized to lowercase at parse time.
    if let Some(req) = requested {
        let want = format!("v{}", req.to_lowercase());
        if let Some(t) = tags.iter().find(|t| **t == want) {
            return Some(t.trim_start_matches('v').to_string());
        }
        // Game fallback: the largest tag <= the requested version.
        let req_parts: Vec<u64> = req.split('.').filter_map(|p| p.parse().ok()).collect();
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
pub fn effective_view(root: &Path, requested: Option<&str>) -> Result<EffectiveModView> {
    let languages_dir = root.join("Languages");

    if let Ok(text) = std::fs::read_to_string(root.join("LoadFolders.xml")) {
        if let Some(folders) = parse_load_folders(&text) {
            let tags: Vec<String> = folders.iter().map(|(t, _, _)| t.clone()).collect();
            let Some(version) = resolve_version_from_tags(&tags, requested) else {
                return Ok(EffectiveModView {
                    version: None,
                    content_dirs: vec![root.to_path_buf()],
                    conditional_dirs: Vec::new(),
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
                    languages_dirs: vec![languages_dir.clone()],
                    languages_dir,
                });
            };
            let mut content_dirs = Vec::new();
            for entry in plain {
                let dir = if entry == "/" {
                    root.to_path_buf()
                } else {
                    root.join(entry)
                };
                if dir.is_dir() {
                    content_dirs.push(dir);
                }
            }
            if content_dirs.is_empty() {
                content_dirs.push(root.to_path_buf());
            }
            let mut conditional_dirs = Vec::new();
            for entry in conditional {
                let dir = if entry == "/" {
                    root.to_path_buf()
                } else {
                    root.join(entry)
                };
                if dir.is_dir() {
                    conditional_dirs.push(dir);
                }
            }
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
                languages_dir,
                languages_dirs,
            });
        }
    }

    // Classic layout: root itself, or 1.x directories picked by the existing
    // version-resolution rules (caller passes the resolved version here).
    let version = requested.map(str::to_string);
    let versioned = root
        .read_dir()
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.split('.').all(|p| p.parse::<u64>().is_ok()) && n.contains('.'))
                        .unwrap_or(false)
                })
                .map(|e| root.join(e.file_name()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if versioned.is_empty() {
        return Ok(EffectiveModView {
            version: None,
            content_dirs: vec![root.to_path_buf()],
            conditional_dirs: Vec::new(),
            languages_dirs: vec![languages_dir.clone()],
            languages_dir,
        });
    }
    let chosen = match &version {
        Some(v) => versioned
            .iter()
            .find(|d| d.file_name().and_then(|f| f.to_str()) == Some(v.as_str()))
            .cloned(),
        None => versioned
            .iter()
            .max_by_key(|d| {
                d.file_name()
                    .and_then(|f| f.to_str())
                    .map(|n| {
                        n.split('.')
                            .filter_map(|p| p.parse::<u64>().ok())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .cloned(),
    };
    let chosen_dir = chosen.clone().unwrap_or_else(|| root.to_path_buf());
    let version_name = chosen
        .as_ref()
        .and_then(|d| d.file_name().and_then(|f| f.to_str()).map(str::to_string));
    let mut languages_dirs = vec![languages_dir.clone()];
    let lang_in_chosen = chosen_dir.join("Languages");
    if lang_in_chosen.is_dir() {
        languages_dirs.push(lang_in_chosen);
    }
    Ok(EffectiveModView {
        version: version_name,
        content_dirs: vec![chosen_dir],
        conditional_dirs: Vec::new(),
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

        let view = effective_view(root, None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert!(view.content_dirs.contains(&root.join("1.6")));
        assert_eq!(view.conditional_dirs, vec![root.join("1.6/Mods/Odyssey")]);
        assert_eq!(
            view.defs_roots(),
            vec![root.join("1.6/Defs"), root.join("1.6/Mods/Odyssey/Defs")]
        );
        assert_eq!(view.languages_dir, root.join("Languages"));
    }

    #[test]
    fn requested_version_must_exist_in_load_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("LoadFolders.xml"),
            "<loadFolders><v1.5><li>1.5</li></v1.5></loadFolders>",
        );
        let view = effective_view(root, Some("1.5")).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.5"));
        // Unknown version → no tag resolved → root fallback (not an error).
        let view = effective_view(root, Some("9.9")).unwrap();
        assert_eq!(view.content_dirs, vec![root.to_path_buf()]);
    }

    #[test]
    fn classic_layout_without_load_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("1.4")).unwrap();
        std::fs::create_dir_all(root.join("1.6")).unwrap();
        let view = effective_view(root, None).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert_eq!(view.content_dirs, vec![root.join("1.6")]);
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

        let view = effective_view(root, Some("1.6")).unwrap();
        assert_eq!(view.version.as_deref(), Some("1.6"));
        assert!(view.content_dirs.contains(&root.join("Common")));
        assert!(view.conditional_dirs.contains(&root.join("AllDir")));
        assert!(view.conditional_dirs.contains(&root.join("NotDir")));
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
        let view = effective_view(root, Some("1.6")).unwrap();
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
