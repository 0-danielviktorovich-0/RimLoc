use std::path::Path;

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
