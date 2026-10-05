use std::cmp::Ordering;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct VersionEntry {
    name: String,
    components: Vec<u32>,
    path: PathBuf,
}

fn parse_version_components(name: &str) -> Option<Vec<u32>> {
    let trimmed = name.trim_start_matches('v');
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for part in trimmed.split('.') {
        if part.is_empty() {
            return None;
        }
        let value: u32 = part.parse().ok()?;
        parts.push(value);
    }
    if parts.is_empty() {
        None
    } else if parts.len() == 1 {
        // Numeric-only names ("3242000764" — a Steam workshop id, a year,
        // a build number) are not RimWorld game versions: legit versions
        // carry at least one dot ("1.5", "v1.4.3"). Without this guard a
        // workshop-id mod root short-circuits resolve_game_version_root
        // and `--game-version 1.5` is silently ignored.
        None
    } else {
        Some(parts)
    }
}

fn normalize_version_input(raw: &str) -> String {
    raw.trim_start_matches('v').to_string()
}

fn find_version_directory(base: &Path, requested: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let normalized = normalize_version_input(requested);
    if requested.starts_with('v') {
        candidates.push(requested.trim().to_string());
        candidates.push(normalized.clone());
    } else {
        candidates.push(normalized.clone());
        candidates.push(format!("v{}", normalized));
    }
    for name in candidates.into_iter() {
        if name.is_empty() {
            continue;
        }
        let candidate = base.join(&name);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn list_version_directories(base: &Path) -> color_eyre::Result<Vec<VersionEntry>> {
    let mut entries = Vec::new();
    let read_dir = match fs::read_dir(base) {
        Ok(iter) => iter,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(entries),
        Err(err) => return Err(err.into()),
    };
    for entry in read_dir {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name_os = entry.file_name();
        let name = match name_os.to_str() {
            Some(s) => s,
            None => continue,
        };
        if let Some(components) = parse_version_components(name) {
            entries.push(VersionEntry {
                name: name.to_string(),
                components,
                path: entry.path(),
            });
        }
    }
    Ok(entries)
}

fn is_version_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .and_then(parse_version_components)
        .is_some()
}

/// True when About/supportedVersions explicitly declares the version. Protects
/// against typos while allowing flat mods to honor a configured game version.
fn flat_mod_supports_version(base: &Path, requested: &str) -> bool {
    let about = base.join("About").join("About.xml");
    let Ok(content) = fs::read_to_string(&about) else {
        // No About.xml at all: this is not a mod (e.g. a game Data root like
        // Core/DLC — whole-game localization targets). Nothing declares
        // versions, so the flat root is the only layout.
        return !base.join("About").exists();
    };
    let Some(block) = content.find("<supportedVersions>").and_then(|start| {
        let rest = &content[start..];
        content[start..]
            .find("</supportedVersions>")
            .map(|end| &rest[..end])
    }) else {
        return false;
    };
    // <li>1.5</li> entries; exact match on the requested string.
    let mut from = 0usize;
    while let Some(a) = block[from..].find("<li>").map(|i| i + from) {
        let Some(b) = block[a..].find("</li>").map(|i| i + a) else {
            break;
        };
        if block[a + 4..b].trim() == requested {
            return true;
        }
        from = b;
    }
    false
}

pub fn resolve_game_version_root(
    base: &Path,
    requested: Option<&str>,
) -> color_eyre::Result<(PathBuf, Option<String>)> {
    if is_version_directory(base) {
        let name = base
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        return Ok((base.to_path_buf(), name));
    }

    let mut entries = list_version_directories(base)?;

    if let Some(req) = requested {
        if let Some(path) = find_version_directory(base, req) {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok((path, name));
        } else if entries.is_empty() && flat_mod_supports_version(base, req) {
            // Flat mod (no versioned folders) that explicitly declares the
            // requested version in About/supportedVersions: the root layout is
            // the only one there is.
            return Ok((base.to_path_buf(), None));
        } else {
            return Err(color_eyre::eyre::eyre!(
                "Requested version '{}' not found under {} (available: {})",
                req,
                base.display(),
                entries
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    if entries.is_empty() {
        return Ok((base.to_path_buf(), None));
    }

    entries.sort_by(|a, b| {
        let len_cmp = a.components.len().cmp(&b.components.len());
        if len_cmp != Ordering::Equal {
            return len_cmp;
        }
        a.components.cmp(&b.components)
    });

    if let Some(entry) = entries.last() {
        return Ok((entry.path.clone(), Some(entry.name.clone())));
    }

    Ok((base.to_path_buf(), None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_version_components_ok() {
        assert_eq!(parse_version_components("1.4"), Some(vec![1, 4]));
        assert_eq!(parse_version_components("v1.4.3"), Some(vec![1, 4, 3]));
        assert_eq!(parse_version_components("10.0"), Some(vec![10, 0]));
    }

    #[test]
    fn parse_version_components_bad() {
        assert_eq!(parse_version_components(""), None);
        assert_eq!(parse_version_components("v"), None);
        assert_eq!(parse_version_components("1..2"), None);
        assert_eq!(parse_version_components("a.b"), None);
    }

    /// Regression (diff vs Text Grabber, workshop mod 3242000764): a Steam
    /// workshop content folder is named by its numeric workshop id. An
    /// all-digits name has no dot-separated components, so it is not a
    /// RimWorld game version (legit forms: `1.5`, `v1.5`, `v1.4.3`).
    #[test]
    fn numeric_only_name_is_not_a_version() {
        assert_eq!(parse_version_components("3242000764"), None);
        assert_eq!(parse_version_components("v3242000764"), None);
        assert_eq!(parse_version_components("2026"), None);
        assert_eq!(parse_version_components("1234567890"), None);
        // Legit versions keep parsing
        assert_eq!(parse_version_components("1.5"), Some(vec![1, 5]));
        assert_eq!(parse_version_components("v1.5"), Some(vec![1, 5]));
    }

    /// The failing user scenario behind the numeric-only fix: the mod root
    /// itself is the numeric workshop directory, `--game-version 1.5` must
    /// resolve into `1.5/` instead of silently scanning the whole root
    /// (which shipped 1.6 content for a 1.5 request).
    #[test]
    fn workshop_id_root_honors_game_version_request() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("3242000764");
        fs::create_dir_all(base.join("About")).unwrap();
        fs::create_dir_all(base.join("1.5")).unwrap();
        fs::create_dir_all(base.join("1.6")).unwrap();

        // Requested version must win over the numeric-looking root name
        let (p, n) = resolve_game_version_root(&base, Some("1.5")).unwrap();
        assert!(p.ends_with("1.5"), "expected …/1.5, got {}", p.display());
        assert_eq!(n.as_deref(), Some("1.5"));

        // Without a request the latest real version is picked, not the id
        let (p2, n2) = resolve_game_version_root(&base, None).unwrap();
        assert!(p2.ends_with("1.6"), "expected …/1.6, got {}", p2.display());
        assert_eq!(n2.as_deref(), Some("1.6"));
    }

    #[test]
    fn normalize_input_strips_prefix() {
        assert_eq!(normalize_version_input("v1.4"), "1.4");
        assert_eq!(normalize_version_input("1.4"), "1.4");
    }

    #[test]
    fn list_and_pick_latest_version() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        // Create version-like subfolders
        for name in ["1.3", "v1.4", "1.10", "1.9.1", "foo", "1.a"].iter() {
            let p = base.join(name);
            fs::create_dir_all(&p).unwrap();
        }

        // Internal helpers should filter only version-like folders
        let entries = list_version_directories(base).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"1.3"));
        assert!(names.contains(&"v1.4"));
        assert!(names.contains(&"1.10"));
        assert!(names.contains(&"1.9.1"));
        assert!(!names.contains(&"foo"));

        // resolve_game_version_root without request picks the "latest" by sort
        let (_path, picked) = resolve_game_version_root(base, None).unwrap();
        // With our length-first sorting, 1.9.1 (len=3) is considered newer than 1.10 (len=2)
        assert_eq!(picked.as_deref(), Some("1.9.1"));

        // Explicit request by either form should resolve
        let (p1, n1) = resolve_game_version_root(base, Some("1.4")).unwrap();
        assert!(p1.ends_with("v1.4") || p1.ends_with("1.4"));
        assert!(matches!(n1.as_deref(), Some("v1.4") | Some("1.4")));

        let (p2, n2) = resolve_game_version_root(base, Some("v1.4")).unwrap();
        assert!(p2.ends_with("v1.4") || p2.ends_with("1.4"));
        assert!(matches!(n2.as_deref(), Some("v1.4") | Some("1.4")));
    }
}
