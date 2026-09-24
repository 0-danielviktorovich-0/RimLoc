use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Canonical matching key for TKey-suffixed DefInjected entries. The game's
/// canonical path adds type-dependent suffixes (`.slateRef`, `.value.slateRef`)
/// to the base `<defName>.<TKey>` identity RimLoc extracts; matching layers
/// normalize both sides through this function.
pub fn canonical_match_key(key: &str) -> String {
    for suffix in [".value.slateRef", ".slateRef"] {
        if let Some(base) = key.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    key.to_string()
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

/// Monotonic counter making temp file names collision-free within a process.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Atomically write `bytes` to `path` with symlink-safe staging:
///
/// - the staging file is created under a unique name with `create_new`, so a
///   pre-planted temp symlink (or a concurrently written temp) can never be
///   opened or truncated through this path — `create_new` refuses to follow
///   a symlink AT THE STAGING LEAF itself;
/// - parent directories of the staging path may still resolve symlinks the
///   way the OS does; keeping the destination outside read-only trees is the
///   caller's boundary policy (see `is_within`);
/// - the final `rename` replaces the destination atomically and does not
///   follow a symlink at the destination leaf — a planted symlink there is
///   replaced itself instead of being written through.
///
/// Shared by every RimLoc writer (bundle artifacts, reports, exports); keep
/// it symlink-safe when touching.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let mut attempt: u32 = 0;
    loop {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = dir.join(format!(
            ".{name}.tmp.{}-{nanos:x}-{n:03}",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(mut f) => {
                let write = f.write_all(bytes).and_then(|_| f.flush());
                drop(f);
                match write {
                    Ok(()) => {
                        let rename_result = fs::rename(&tmp, path);
                        if rename_result.is_err() {
                            let _ = fs::remove_file(&tmp);
                        }
                        return rename_result;
                    }
                    Err(e) => {
                        let _ = fs::remove_file(&tmp);
                        return Err(e);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempt < 8 => {
                // Astronomically unlikely (pid+nanos+counter); just retry.
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Canonical containment view of a path with REAL symlink resolution,
/// portable prefix semantics included:
///
/// - the LONGEST EXISTING PREFIX is resolved with `std::fs::canonicalize`
///   (extended-length `\\?\` form + canonical case on Windows) and the
///   non-existing tail is appended onto that base — so the slow path lands
///   in the exact same representation as the fast path and containment
///   (`starts_with`) compares like with like;
/// - every existing component is additionally checked with
///   `symlink_metadata`; a symlink target is resolved iteratively (relative
///   targets against the resolved prefix), `..` is applied to the resolved
///   prefix;
/// - only `NotFound` is treated as "non-existing tail"; any other IO error
///   (permission, not-a-directory, …) fails closed with `Err`.
///
/// Containment callers must fail closed on `Err` rather than guess.
pub fn canonical_view(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    const MAX_JUMPS: usize = 64;
    fn map(c: Component<'_>) -> Step {
        match c {
            Component::RootDir => Step::Root,
            Component::Prefix(p) => Step::Prefix(p.as_os_str().to_os_string()),
            Component::CurDir => Step::Cur,
            Component::ParentDir => Step::Parent,
            Component::Normal(n) => Step::Name(n.to_os_string()),
        }
    }

    // Fast path: the path exists as-is — std canonicalize is authoritative
    // (extended-length prefix + canonical case on Windows).
    if let Ok(c) = path.canonicalize() {
        return Ok(c);
    }

    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        // Relative candidate: failure to learn the cwd is an error, not a
        // silent "." default.
        std::env::current_dir()?.join(path)
    };

    // Seed the resolver with the LONGEST EXISTING PREFIX, canonically
    // resolved. This is what keeps fast and slow paths comparable: both
    // share the same canonical prefix form and case before the
    // non-existing tail is appended.
    let components: Vec<Component<'_>> = abs.components().collect();
    let mut keep = components.len();
    let canonical_base = loop {
        if keep == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "no existing anchor to resolve the path against",
            ));
        }
        let mut candidate = PathBuf::new();
        for c in &components[..keep] {
            candidate.push(c.as_os_str());
        }
        match candidate.canonicalize() {
            Ok(c) => break c,
            // Shrinking the prefix may cross back into existing territory
            // (not-a-directory chains etc.); only shrinking past a missing
            // entry is benign — anything else fails closed.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => keep -= 1,
            Err(e) => return Err(e),
        }
    };

    let mut resolved = canonical_base;
    let mut pending: std::collections::VecDeque<Step> =
        components[keep..].iter().map(|&c| map(c)).collect();
    let mut jumps = 0usize;

    while let Some(step) = pending.pop_front() {
        match step {
            Step::Root => {
                // On Windows this is the root separator right after a prefix;
                // on POSIX the leading "/".
                resolved.push(std::path::MAIN_SEPARATOR.to_string());
            }
            Step::Prefix(p) => {
                resolved.push(p);
            }
            Step::Cur => {}
            Step::Parent => {
                resolved.pop();
            }
            Step::Name(name) => {
                resolved.push(&name);
                match std::fs::symlink_metadata(&resolved) {
                    Ok(md) if md.file_type().is_symlink() => {
                        jumps += 1;
                        if jumps > MAX_JUMPS {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                "symlink loop or too many link jumps while resolving path",
                            ));
                        }
                        let target = std::fs::read_link(&resolved)?;
                        resolved.pop(); // the link itself is replaced by its target
                        for c in target.components().rev() {
                            pending.push_front(map(c));
                        }
                    }
                    Ok(_) => {}
                    // Only a missing entry is benign: no symlink can hide in
                    // a non-existing tail. Anything else fails closed.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e),
                }
            }
        }
    }

    if resolved.as_os_str().is_empty() {
        resolved.push(".");
    }
    Ok(resolved)
}

#[derive(Debug)]
enum Step {
    Root,
    Prefix(std::ffi::OsString),
    Name(std::ffi::OsString),
    Parent,
    Cur,
}

/// True when `candidate` is equal to or nested inside `root` after real
/// symlink resolution (sibling traversals and symlink aliases included).
/// Fail-closed: an unresolvable candidate is treated as contained, so that
/// write guards reject instead of guessing.
pub fn is_within(candidate: &std::path::Path, root: &std::path::Path) -> bool {
    if root.as_os_str().is_empty() {
        return false;
    }
    match (canonical_view(candidate), canonical_view(root)) {
        (Ok(c), Ok(r)) => c.starts_with(&r),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_never_follows_preplanted_temp_symlink() {
        let tmp = tempfile::tempdir().expect("tmp");
        let source = tmp.path().join("source");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&source).expect("dirs");
        std::fs::create_dir_all(&out).expect("dirs");
        let sentinel = source.join("sentinel.txt");
        std::fs::write(&sentinel, b"KEEP").expect("sentinel");

        // Pre-plant every historically predictable temp name as symlinks
        // into the source sentinel (old name scheme + dot-prefixed variant).
        let target = out.join("environment.json");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&sentinel, out.join("environment.tmp.write"))
                .expect("plant old-name symlink");
            std::os::unix::fs::symlink(&sentinel, out.join(".environment.json.tmp.write"))
                .expect("plant dot-name symlink");
        }

        write_atomic(&target, b"FRESH").expect("write");
        assert_eq!(
            std::fs::read(&sentinel).expect("read sentinel"),
            b"KEEP",
            "sentinel must survive pre-planted temp symlinks"
        );
        assert_eq!(std::fs::read(&target).expect("read target"), b"FRESH");

        // Overwrite through the same path keeps working (rename replaces).
        write_atomic(&target, b"SECOND").expect("overwrite");
        assert_eq!(std::fs::read(&target).expect("read"), b"SECOND");
        assert_eq!(std::fs::read(&sentinel).expect("sentinel"), b"KEEP");

        // No NEW staging leftovers; the pre-planted symlinks themselves must
        // survive untouched (write_atomic never deletes foreign entries).
        let mut names: Vec<String> = std::fs::read_dir(&out)
            .expect("readdir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        let mut expected = vec![
            ".environment.json.tmp.write".to_string(),
            "environment.json".to_string(),
            "environment.tmp.write".to_string(),
        ];
        expected.sort();
        assert_eq!(names, expected, "exactly target + planted symlinks remain");
    }

    #[test]
    fn containment_catches_sibling_traversal_and_symlinks() {
        let tmp = tempfile::tempdir().expect("tmp");
        let base = tmp.path();
        let source = base.join("source");
        let out = base.join("out");
        std::fs::create_dir_all(source.join("Defs")).expect("dirs");
        std::fs::create_dir_all(&out).expect("dirs");

        // Lead probe case: sibling dirs + non-existing traversal tail.
        let traversal = out.join("missing/../../source/bundle");
        assert!(
            is_within(&traversal, &source),
            "sibling traversal into source must be caught"
        );
        // The resolved view lands inside the source, not in `out`.
        let view = canonical_view(&traversal).expect("resolvable");
        let source_view = canonical_view(&source).expect("source view");
        assert!(view.starts_with(source_view));

        // Escaping traversal OUT of the source must NOT false-positive.
        let escape = source.join("Defs/../../../outside");
        assert!(
            !is_within(&escape, &source),
            "traversal escaping the source is outside"
        );

        // Symlink alias into the source → contained.
        #[cfg(unix)]
        {
            let alias = base.join("alias");
            std::os::unix::fs::symlink(&source, &alias).expect("symlink");
            assert!(is_within(&alias.join("bundle"), &source));

            // Symlink pointing OUT of the source → not contained (no false
            // positive on the destination that the link resolves to).
            let jump_out = source.join("jump");
            std::os::unix::fs::symlink(&out, &jump_out).expect("symlink out");
            assert!(!is_within(&jump_out, &source));

            // Dangling symlink resolves deterministically to its (missing)
            // target — not ambiguous, so it must NOT fail closed.
            let dangling = base.join("dangling");
            std::os::unix::fs::symlink(base.join("nowhere"), &dangling).expect("symlink");
            let view = canonical_view(&dangling).expect("dangling resolves");
            assert!(view.ends_with("nowhere"));
            assert!(!is_within(&dangling, &source));

            // Symlink loop → fail-closed via Err.
            let a = base.join("loop-a");
            let b = base.join("loop-b");
            std::os::unix::fs::symlink(&b, &a).expect("symlink a→b");
            std::os::unix::fs::symlink(&a, &b).expect("symlink b→a");
            assert!(canonical_view(&a.join("x")).is_err(), "loop must error");
        }

        // Plain containment and plain outside.
        assert!(is_within(&source.join("Defs/x.xml"), &source));
        assert!(!is_within(&out, &source));
    }

    // Windows prefix semantics: compiled on every platform, EXECUTABLE only
    // on Windows — a macOS run does NOT prove these; run on a Windows host
    // (cargo test -p rimloc-services) before relying on them.
    #[cfg(windows)]
    #[test]
    fn containment_distinguishes_drive_and_unc_prefixes() {
        // Different drives are never contained, even with matching names.
        assert!(!is_within(Path::new(r"D:\data"), Path::new(r"C:\src")));
        assert!(!is_within(Path::new(r"C:\src"), Path::new(r"D:\src")));
        // Same drive: containment and sibling discrimination.
        assert!(is_within(Path::new(r"C:\src\sub"), Path::new(r"C:\src")));
        assert!(!is_within(Path::new(r"C:\srcx"), Path::new(r"C:\src")));
        // UNC roots are preserved and compared natively.
        assert!(is_within(
            Path::new(r"\\server\share\src\sub"),
            Path::new(r"\\server\share\src")
        ));
        assert!(!is_within(
            Path::new(r"\\server\share\other"),
            Path::new(r"\\server\share\src")
        ));
        assert!(!is_within(
            Path::new(r"\\otherserver\share\src"),
            Path::new(r"\\server\share\src")
        ));
        // Forward-slash spellings of a drive resolve to the same view.
        assert!(is_within(Path::new("C:/src/sub/.."), Path::new(r"C:\\src")));
    }

    // Round-016: fast path (std canonicalize, extended-length + canonical
    // case) and slow path (canonical base + non-existing tail) must land in
    // the SAME representation, or starts_with containment misses.
    #[cfg(windows)]
    #[test]
    fn slow_path_matches_fast_path_prefix_and_case() {
        let base =
            std::env::temp_dir().join(format!("rimloc-canonical-parity-{}", std::process::id()));
        let source = base.join("Source"); // existing, mixed case
        std::fs::create_dir_all(&source).expect("dirs");

        // Existing root + non-existing nested output = contained (the guard
        // rejects), and the slow view is IDENTICAL to the fast view.
        let nested = source.join("new").join("bundle");
        assert!(is_within(&nested, &source), "nested output contained");
        let fast = canonical_view(&source).expect("fast");
        let slow = canonical_view(&nested).expect("slow");
        assert!(slow.starts_with(&fast), "slow {slow:?} vs fast {fast:?}");

        // Case alias of the existing root normalizes to the canonical case:
        // the slow view of base/source/new equals the fast view of Source/new.
        let lower_alias = base.join("source").join("new");
        assert_eq!(
            canonical_view(&lower_alias).expect("alias slow"),
            slow,
            "case alias must normalize to the canonical case"
        );

        // Extended-length spelling of the same root is the same view.
        let ext = PathBuf::from(format!(r"\\?\{}", source.display()));
        assert_eq!(
            canonical_view(&ext).expect("ext fast"),
            canonical_view(&source).expect("std fast"),
            "extended-length and standard spellings agree"
        );
        assert!(is_within(&ext.join("new"), &source));

        // Different drive stays outside.
        assert!(!is_within(Path::new(r"D:\new"), &source));

        std::fs::remove_dir_all(&base).ok();
    }
}
