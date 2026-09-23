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
        let tag = ver.tag_name().name().to_string();
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
            if li.has_attribute("IfModActive") {
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
    if let Some(req) = requested {
        let want = format!("v{req}");
        return tags
            .iter()
            .find(|t| **t == want)
            .cloned()
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
                    languages_dir,
                });
            };
            let vtag = format!("v{version}");
            let Some((_, plain, conditional)) = folders.iter().find(|(t, _, _)| *t == vtag) else {
                return Ok(EffectiveModView {
                    version: None,
                    content_dirs: vec![root.to_path_buf()],
                    conditional_dirs: Vec::new(),
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
            return Ok(EffectiveModView {
                version: Some(version),
                content_dirs,
                conditional_dirs,
                languages_dir,
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
    Ok(EffectiveModView {
        version: chosen
            .as_ref()
            .and_then(|d| d.file_name().and_then(|f| f.to_str()).map(str::to_string)),
        content_dirs: vec![chosen.unwrap_or_else(|| root.to_path_buf())],
        conditional_dirs: Vec::new(),
        languages_dir,
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
