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
