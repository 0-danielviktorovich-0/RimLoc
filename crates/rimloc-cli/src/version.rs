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
        } else {
            return Err(color_eyre::eyre::eyre!(
                "Requested version '{}' not found under {}",
                req,
                base.display()
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
