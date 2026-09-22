use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn is_under_languages_dir(path: &Path, lang_dir: &str) -> bool {
    let mut comps = path.components();
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("Languages") {
            if let Some(lang) = comps.next() {
                let lang_s = lang.as_os_str().to_string_lossy();
                return lang_s == lang_dir;
            }
            return false;
        }
    }
    false
}

/// Normalize a CLI language argument that may be either a bare directory
/// name ("English") or a full path pointing at a language directory
/// ("/path/to/mod/Languages/English") into the bare directory name used by
/// `is_under_languages_dir`. Fixes coverage/validate receiving full paths.
pub fn normalize_lang_dir(input: &str) -> String {
    let t = input.trim();
    if t.is_empty() {
        return t.to_string();
    }
    let p = Path::new(t);
    if t.contains('/') || t.contains('\\') || p.is_dir() || p.is_file() {
        if let Some(name) = p.file_name().and_then(OsStr::to_str) {
            return name.to_string();
        }
    }
    t.to_string()
}

/// Return true if a path should be considered part of the source set for a given
/// RimWorld language directory name. For English, treat both Languages/English and
/// Defs as valid sources (since many mods omit English LanguageData and rely on Defs).
pub fn is_source_for_lang_dir(path: &Path, lang_dir: &str) -> bool {
    if is_under_languages_dir(path, lang_dir) {
        return true;
    }
    if lang_dir.eq_ignore_ascii_case("English") {
        // Any XML under Defs/* counts as English source
        let s = path.to_string_lossy();
        return s.contains("/Defs/") || s.contains("\\Defs\\");
    }
    false
}

/// Derive the canonical DefInjected path for a Defs XML file.
///
/// This keeps file names stable while rewriting the directory to
/// `Languages/<lang_dir>/DefInjected/<def_type>/...` so callers can
/// surface the expected target location for translators.
pub fn def_injected_target_path(
    scan_root: &Path,
    lang_dir: &str,
    def_type: &str,
    source_path: &Path,
) -> PathBuf {
    let file_name = source_path
        .file_name()
        .map(|s| s.to_owned())
        .unwrap_or_else(|| OsStr::new("Defs.xml").to_owned());
    scan_root
        .join("Languages")
        .join(lang_dir)
        .join("DefInjected")
        .join(def_type)
        .join(file_name)
}

pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let tmp = path.with_extension("tmp.write");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.flush()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}
