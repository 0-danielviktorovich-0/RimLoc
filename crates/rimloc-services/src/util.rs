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
///
/// This function is pure write MECHANICS: the boundary policy (where a
/// destination may resolve) belongs to the canonical guard
/// [`ensure_writable_output_path`] / [`ensure_free_output_path`], which
/// every tainted-path caller runs BEFORE calling this — and writes to the
/// path the guard returns.
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

/// DENY-direction containment: true when `candidate` is equal to or nested
/// inside `root` after real symlink resolution (sibling traversals and
/// symlink aliases included). For write guards of the shape
/// `if is_within(target, protected_root) { reject }`: an UNRESOLVABLE
/// candidate is treated as contained, so the guard rejects instead of
/// guessing. Never use this to GRANT access — for allow-direction checks
/// use [`is_within_allow`].
pub fn is_within(candidate: &std::path::Path, root: &std::path::Path) -> bool {
    if root.as_os_str().is_empty() {
        return false;
    }
    match (canonical_view(candidate), canonical_view(root)) {
        (Ok(c), Ok(r)) => c.starts_with(&r),
        // Deny orientation: unresolvable → treated as contained → reject.
        _ => true,
    }
}

/// ALLOW-direction containment: true ONLY when containment is proven —
/// both paths resolve through the real symlink view AND `candidate` is
/// equal to or nested inside `root`. Any canonicalization failure is a
/// plain `false`: an allow decision must never rest on an unverified
/// path. For deny-direction guards (`reject when contained`) use
/// [`is_within`], whose failure mode rejects.
pub fn is_within_allow(candidate: &std::path::Path, root: &std::path::Path) -> bool {
    if root.as_os_str().is_empty() {
        return false;
    }
    match (canonical_view(candidate), canonical_view(root)) {
        (Ok(c), Ok(r)) => c.starts_with(&r),
        // Allow orientation: containment must be PROVEN.
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// THE canonical write-path guard
// ---------------------------------------------------------------------------
// Every flagged `rust/path-injection` write site funnels through this one
// guard before touching the filesystem: the taint-relevant decision
// (absolute form, real-location resolution, containment) is made HERE, on
// canonical views, and the caller writes to the RETURNED path — never to
// the raw input. Two public faces over one core pipeline, because the two
// site classes answer different questions:
//
// - [`ensure_writable_output_path`] — the destination has a defined
//   writable root (app-managed caches, embedded exports): containment
//   inside one of `allow_roots` must be PROVEN on the symlink-resolved
//   view;
// - [`ensure_free_output_path`] — a caller-chosen destination (user picks
//   a path in a dialog / CLI argument): any location is legitimate, so the
//   guard only refuses RimLoc-owned `protected` roots (managed project
//   store, read-only source trees) — checked on the canonical view, so a
//   symlink alias into a protected root is caught too.

/// Why [`ensure_writable_output_path`] / [`ensure_free_output_path`]
/// refused a path. Kept machine-comparable so IPC/CLI layers can map the
/// verdict onto their own typed errors (the session layer does exactly
/// that: `NotAbsolute` → `invalid_output_path`, `ProtectedRoot` →
/// `guard_output_denied`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathGuardErrorKind {
    /// Relative input. The guard never resolves silently: pass an absolute
    /// path, or join an explicit base first (`resolve_cli_out_path` is the
    /// documented CLI flavor — the GUI surface refuses relative instead,
    /// per the RC K4 `RimLoc-Export` incident).
    NotAbsolute,
    /// Real-location resolution failed with a non-NotFound IO error
    /// (permission, not-a-directory, symlink loop). Fail-closed by design.
    Unresolvable,
    /// The resolved location is inside NONE of the allow roots.
    OutsideAllowRoots,
    /// The raw path spelled a location inside an allow root, but the
    /// symlink-resolved real location landed outside every root.
    SymlinkEscape,
    /// Free-form mode: the resolved location is inside a protected
    /// (RimLoc-owned or read-only) root.
    ProtectedRoot,
}

/// Typed rejection of the canonical write guard. Public fields let IPC/CLI
/// render a precise answer without re-parsing the message string.
#[derive(Debug, Clone)]
pub struct PathGuardError {
    pub kind: PathGuardErrorKind,
    /// The path exactly as supplied.
    pub path: PathBuf,
    /// Canonical (symlink-resolved) view, when it could be computed.
    pub resolved: Option<PathBuf>,
    /// Human-readable reason, ready for an IPC/CLI response.
    pub message: String,
}

impl std::fmt::Display for PathGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PathGuardError {}

/// THE canonical write guard (allow-direction): `path` must be absolute and
/// its REAL (symlink-resolved) location must sit inside at least one of
/// `allow_roots`. Returns the canonical path to write to.
///
/// Pipeline, in order:
/// 1. absolute form required — relative input is [`PathGuardErrorKind::NotAbsolute`],
///    never silently resolved against an ambient CWD;
/// 2. canonical resolution via [`canonical_view`] — the longest existing
///    prefix is `std::fs::canonicalize`d, the non-existing tail is resolved
///    component-wise with symlink following; any non-NotFound IO error is
///    [`PathGuardErrorKind::Unresolvable`] (fail-closed);
/// 3. containment is proven on the canonical view (`starts_with` over
///    components, so sibling-prefix tricks like `/a/bc` vs `/a/b` cannot
///    pass); a path lexically inside a root that resolves outside every
///    root is the distinct [`PathGuardErrorKind::SymlinkEscape`] verdict;
/// 4. the returned canonical path is the ONLY blessed write target.
pub fn ensure_writable_output_path(
    path: &std::path::Path,
    allow_roots: &[&std::path::Path],
) -> Result<PathBuf, PathGuardError> {
    let resolved = resolve_for_write(path)?;
    if allow_roots.is_empty() {
        return Err(PathGuardError {
            kind: PathGuardErrorKind::OutsideAllowRoots,
            path: path.to_path_buf(),
            resolved: Some(resolved),
            message: "no writable roots configured for this surface; refusing the write".to_string(),
        });
    }
    // Lexical pre-check on the RAW spelling (components normalized, `..`
    // applied, symlinks NOT followed) only to classify the refusal: a path
    // that spells "inside a root" but resolves elsewhere escaped through a
    // symlink, and the error should say so.
    let lexically_inside = allow_roots
        .iter()
        .any(|root| lexically_within(path, root));
    for root in allow_roots {
        // ALLOW orientation: containment must be PROVEN on the real view.
        if is_within_allow(&resolved, root) {
            return Ok(resolved);
        }
    }
    if lexically_inside {
        let message = format!(
            "`{}` resolves (through a symlink) to `{}`, which is outside every writable root",
            path.display(),
            resolved.display()
        );
        return Err(PathGuardError {
            kind: PathGuardErrorKind::SymlinkEscape,
            path: path.to_path_buf(),
            resolved: Some(resolved),
            message,
        });
    }
    let message = format!(
        "`{}` resolves to `{}`, which is outside every writable root ({})",
        path.display(),
        resolved.display(),
        allow_roots
            .iter()
            .map(|r| format!("`{}`", r.display()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Err(PathGuardError {
        kind: PathGuardErrorKind::OutsideAllowRoots,
        path: path.to_path_buf(),
        resolved: Some(resolved),
        message,
    })
}

/// Free-form flavor of the canonical write guard for caller-chosen
/// destinations (CLI arguments, GUI dialog/IPC paths): ANY location the
/// operator picks is legitimate — including next to or inside the scanned
/// mod tree — so there are no allow roots; the guard refuses only the
/// RimLoc-owned/protected roots passed in `protected` (managed project
/// store, read-only source trees). The deny check runs on the canonical
/// view, so a symlink alias pointing into a protected root is caught as
/// well. Returns the canonical path to write to.
pub fn ensure_free_output_path(
    path: &std::path::Path,
    protected: &[&std::path::Path],
) -> Result<PathBuf, PathGuardError> {
    let resolved = resolve_for_write(path)?;
    for root in protected {
        // DENY orientation: an unresolvable protected root is treated as
        // containing the candidate → the write is refused, never guessed.
        if is_within(&resolved, root) {
            let message = format!(
                "`{}` resolves to `{}`, which is inside the protected root `{}`",
                path.display(),
                resolved.display(),
                root.display()
            );
            return Err(PathGuardError {
                kind: PathGuardErrorKind::ProtectedRoot,
                path: path.to_path_buf(),
                resolved: Some(resolved),
                message,
            });
        }
    }
    Ok(resolved)
}

/// Explicit CLI resolution policy for caller-supplied output arguments:
/// a relative path is joined against the process CWD HERE — visibly, once,
/// at the CLI boundary — instead of being resolved implicitly by the OS at
/// every fs call. The result then goes through [`ensure_free_output_path`].
/// The GUI surface must NOT use this: it refuses relative paths outright
/// (RC K4 — a silently CWD-resolved GUI path is how the built-app
/// `…/RimLoc-Export/…` incident happened).
pub fn resolve_cli_out_path(path: &std::path::Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

/// Shared pipeline head of both guard flavors: absolute-form check +
/// canonical real-location resolution (fail-closed).
fn resolve_for_write(path: &std::path::Path) -> Result<PathBuf, PathGuardError> {
    if !path.is_absolute() {
        return Err(PathGuardError {
            kind: PathGuardErrorKind::NotAbsolute,
            path: path.to_path_buf(),
            resolved: None,
            message: format!(
                "`{}` is not an absolute path; pass an absolute path (the GUI surface refuses relative paths; the CLI resolves them against the working directory explicitly)",
                path.display()
            ),
        });
    }
    canonical_view(path).map_err(|e| PathGuardError {
        kind: PathGuardErrorKind::Unresolvable,
        path: path.to_path_buf(),
        resolved: None,
        message: format!("`{}` could not be resolved to a real location: {e}", path.display()),
    })
}

/// Component-wise lexical containment of `path` inside `root` WITHOUT
/// symlink resolution: `.` dropped, `..` applied to the accumulated
/// prefix. Used solely to distinguish `SymlinkEscape` from
/// `OutsideAllowRoots` in the guard's verdict; the grant itself always
/// rests on the canonical view.
fn lexically_within(path: &std::path::Path, root: &std::path::Path) -> bool {
    fn normalize(p: &std::path::Path) -> Vec<std::ffi::OsString> {
        let mut out: Vec<std::ffi::OsString> = Vec::new();
        for c in p.components() {
            match c {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                Component::Normal(n) => out.push(n.to_os_string()),
                Component::RootDir | Component::Prefix(_) => out.push(c.as_os_str().to_os_string()),
            }
        }
        out
    }
    if root.as_os_str().is_empty() {
        return false;
    }
    let (p, r) = (normalize(path), normalize(root));
    p.len() >= r.len() && p[..r.len()] == r[..]
}

/// Strict language-folder form for WRITE paths (H1 — the CLI sibling of
/// the session's locale guard, P1-2): the folder is joined into
/// `<mod>/Languages/<dir>/...`, and `Path::join` with an absolute string
/// replaces the whole prefix. Anything but a plain folder name (letters,
/// digits, `_`, `-`) is rejected before any path is built.
pub fn lang_dir_form_ok(lang_dir: &str) -> bool {
    static LANG_DIR_FORM: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"^[A-Za-z0-9_-]+$").unwrap());
    LANG_DIR_FORM.is_match(lang_dir)
}

/// Combined write-target guard (H1): strict folder form PLUS containment
/// — the joined `Languages/<dir>` base must stay inside the mod root
/// (deny-direction containment; symlink aliases included).
pub fn ensure_lang_write_target(mod_root: &std::path::Path, lang_dir: &str) -> crate::Result<()> {
    if !lang_dir_form_ok(lang_dir) {
        color_eyre::eyre::bail!(
            "malformed language folder `{lang_dir}`: expected a plain folder name (letters, digits, `_`, `-`); traversal or absolute paths are not allowed"
        );
    }
    let base = mod_root.join("Languages").join(lang_dir);
    if !crate::is_within(&base, mod_root) {
        color_eyre::eyre::bail!(
            "language folder `{}` resolves outside the mod root `{}`",
            base.display(),
            mod_root.display()
        );
    }
    Ok(())
}

/// RimWorld packageId derived from an arbitrary human name (H4): the game
/// requires `<author>.<mod>` over a restricted charset — a raw folder name
/// like `My Mod & <Test>` is not a valid packageId. The slug keeps only
/// `[a-z0-9-]` (everything else collapses to `-`), and the fixed `rimloc.`
/// author segment guarantees the required single-dot shape.
pub fn package_id_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = true; // suppress leading dashes
    for c in name.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
            last_dash = false;
        } else if c.is_ascii_uppercase() {
            slug.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        slug.push_str("translation");
    }
    format!("rimloc.{slug}")
}

/// Strict XML 1.0 character verification of every `.xml` file under
/// `root` (H3 tripwire): the lenient scanner can silently accept raw
/// control characters that a strict parser — and the game — would reject
/// with a whole-file drop. Export verification runs this AFTER the write;
/// a failure here is a writer bug and must fail the operation, never ship.
pub fn verify_xml_char_validity(root: &std::path::Path) -> crate::Result<()> {
    fn walk(dir: &std::path::Path) -> crate::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path)?;
                continue;
            }
            let is_xml = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("xml"))
                .unwrap_or(false);
            if !is_xml {
                continue;
            }
            let bytes = std::fs::read(&path)?;
            let text = std::str::from_utf8(&bytes).map_err(|e| {
                color_eyre::eyre::eyre!("written XML `{}` is not valid UTF-8: {e}", path.display())
            })?;
            if let Some(bad) = rimloc_core::xml_chars::find_invalid_xml_char(text) {
                color_eyre::eyre::bail!(
                    "written XML `{}` contains raw control character U+{:04X} that is invalid in XML 1.0",
                    path.display(),
                    bad as u32
                );
            }
        }
        Ok(())
    }
    walk(root)
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

    /// P2-7: the two containment orientations fail in OPPOSITE, honest
    /// directions. An unresolvable candidate (an existing FILE used as an
    /// intermediate component → ENOTDIR) is deny-safe for `is_within`
    /// (treated as contained, so write guards reject) and refuses to grant
    /// for `is_within_allow`. Proven containment behaves identically in
    /// both.
    #[test]
    fn deny_and_allow_orientations_fail_opposite_ways_on_unresolvable() {
        let tmp = tempfile::tempdir().expect("tmp");
        let source = tmp.path().join("source");
        let blocker = tmp.path().join("blocker");
        std::fs::create_dir_all(&source).expect("dirs");
        std::fs::write(&blocker, b"not a dir").expect("file");

        // Unresolvable candidate: `blocker/child` cannot exist.
        let unresolvable = blocker.join("child");
        assert!(is_within(&unresolvable, &source), "deny-safe: reject");
        assert!(!is_within_allow(&unresolvable, &source), "no grant");

        // Proven containment agrees in both orientations.
        let nested = source.join("Defs").join("x.xml");
        assert!(is_within(&nested, &source));
        assert!(is_within_allow(&nested, &source));

        // Proven escape agrees in both orientations.
        let sibling = tmp.path().join("elsewhere");
        assert!(!is_within(&sibling, &source));
        assert!(!is_within_allow(&sibling, &source));

        // Empty root denies in both orientations (nothing to contain).
        assert!(!is_within(&nested, Path::new("")));
        assert!(!is_within_allow(&nested, Path::new("")));
    }

    /// H3 tripwire: the strict byte-level XML 1.0 char check catches raw
    /// control characters the lenient scanner misses.
    #[test]
    fn verify_xml_char_validity_catches_raw_control_bytes() {
        let tmp = tempfile::tempdir().expect("tmp");
        let good = tmp.path().join("good.xml");
        std::fs::write(
            &good,
            "<?xml version=\"1.0\"?><LanguageData><K>ок</K></LanguageData>",
        )
        .expect("write good");
        verify_xml_char_validity(tmp.path()).expect("clean tree passes");

        let nested = tmp.path().join("DefInjected").join("ThingDef");
        std::fs::create_dir_all(&nested).expect("dirs");
        let bad = nested.join("C.xml");
        std::fs::write(
            &bad,
            "<LanguageData><C.label>плохой\u{7}текст</C.label></LanguageData>",
        )
        .expect("write bad");
        let err = verify_xml_char_validity(tmp.path()).expect_err("control char caught");
        assert!(
            err.to_string().contains("U+0007"),
            "names the offending codepoint: {err}"
        );
        // The non-xml sibling is never inspected.
        std::fs::remove_file(&bad).expect("cleanup");
        std::fs::write(nested.join("notes.txt"), "raw \u{0c} bytes fine here").expect("write txt");
        verify_xml_char_validity(tmp.path()).expect("only .xml files are verified");
    }
    /// H4: packageId slugs keep the RimWorld `<author>.<mod>` shape over a
    /// restricted charset for arbitrary human names.
    #[test]
    fn package_id_slug_is_rimworld_shaped() {
        assert_eq!(package_id_slug("My Mod & <Test>"), "rimloc.my-mod-test");
        assert_eq!(package_id_slug("Урон 50%"), "rimloc.50");
        assert_eq!(package_id_slug("..."), "rimloc.translation");
        let slug = package_id_slug("Over-the-Top v2.0!");
        assert!(
            regex::Regex::new(r"^rimloc\.[a-z0-9-]+$")
                .unwrap()
                .is_match(&slug),
            "{slug}"
        );
    }

    // --- the canonical write-path guard ------------------------------------

    /// Guard: a `../`-traversal that lexically leaves the allow root is
    /// refused as OutsideAllowRoots; the returned path for a LEGIT nested
    /// destination is the canonical view.
    #[test]
    fn guard_rejects_parent_traversal_out_of_root() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("writable");
        std::fs::create_dir_all(&root).expect("dirs");
        let outside = tmp.path().join("outside");

        let escape = root.join("sub/../../outside/evil.json");
        let err = ensure_writable_output_path(&escape, &[&root]).expect_err("traversal refused");
        assert_eq!(err.kind, PathGuardErrorKind::OutsideAllowRoots);
        assert!(
            err.message.contains("outside every writable root"),
            "{err}"
        );
        // No side effects: the refused path was never created.
        assert!(!outside.exists(), "refused path must not be created");

        // The same shape that stays INSIDE the root is fine.
        let ok = root.join("sub/../out.json");
        let got = ensure_writable_output_path(&ok, &[&root]).expect("inside ok");
        assert_eq!(got, canonical_view(&root).unwrap().join("out.json"));
    }

    /// Guard: an absolute destination outside every allow root is refused
    /// even though it is perfectly well-formed.
    #[test]
    fn guard_rejects_absolute_outside_allow_roots() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("writable");
        std::fs::create_dir_all(&root).expect("dirs");
        let sibling = tmp.path().join("sibling");
        std::fs::create_dir_all(&sibling).expect("dirs");

        let err = ensure_writable_output_path(&sibling.join("x.json"), &[&root])
            .expect_err("sibling root refused");
        assert_eq!(err.kind, PathGuardErrorKind::OutsideAllowRoots);
        // The resolved parent is the CANONICAL sibling (macOS /var→/private
        // style aliases included), not the raw spelling.
        assert_eq!(
            err.resolved.as_deref().map(Path::parent),
            Some(Some(canonical_view(&sibling).unwrap().as_path()))
        );

        // Equal to the root itself counts as contained.
        let got = ensure_writable_output_path(&root, &[&root]).expect("root itself contained");
        assert_eq!(got, canonical_view(&root).unwrap());
    }

    /// Guard: a symlink INSIDE the allow root pointing OUT of it must be
    /// refused (SymlinkEscape) when the raw spelling looks contained —
    /// writing through the link would land outside every root. Unix-only:
    /// needs real symlinks.
    #[test]
    #[cfg(unix)]
    fn guard_rejects_symlink_escape_from_root() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("writable");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&root).expect("dirs");
        std::fs::create_dir_all(&outside).expect("dirs");
        std::os::unix::fs::symlink(&outside, root.join("jump")).expect("symlink out");

        let via_link = root.join("jump/evil.json");
        let err =
            ensure_writable_output_path(&via_link, &[&root]).expect_err("symlink escape refused");
        assert_eq!(err.kind, PathGuardErrorKind::SymlinkEscape, "{err}");
        assert!(
            err.message.contains("symlink"),
            "verdict must name the mechanism: {err}"
        );
        // The escape target stayed untouched.
        assert!(!outside.join("evil.json").exists());
    }

    /// Guard: a symlink ALIAS OF THE ROOT ITSELF (the link resolves to the
    /// root, not away from it) stays legitimate — containment is judged on
    /// the real location, not the spelling.
    #[test]
    #[cfg(unix)]
    fn guard_allows_symlink_alias_into_root() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("writable");
        std::fs::create_dir_all(&root).expect("dirs");
        let alias = tmp.path().join("alias-to-root");
        std::os::unix::fs::symlink(&root, &alias).expect("symlink alias");

        let got = ensure_writable_output_path(&alias.join("out.json"), &[&root])
            .expect("alias into root is contained");
        assert_eq!(got, canonical_view(&root).unwrap().join("out.json"));
    }

    /// Guard: a non-existing TAIL inside an existing root is fine — the
    /// guard resolves the longest existing prefix and proves containment of
    /// the would-be location; parents are NOT created by the guard itself
    /// (that stays the writer's job).
    #[test]
    fn guard_accepts_missing_tail_inside_existing_root() {
        let tmp = tempfile::tempdir().expect("tmp");
        let root = tmp.path().join("writable");
        std::fs::create_dir_all(&root).expect("dirs");

        let fresh = root.join("a/b/report.json");
        let got = ensure_writable_output_path(&fresh, &[&root]).expect("missing tail ok");
        assert_eq!(got, canonical_view(&root).unwrap().join("a/b/report.json"));
        assert!(!fresh.exists(), "the guard never creates anything");
    }

    /// Guard: relative input is a typed NotAbsolute refusal in both
    /// flavors — never a silent CWD resolution (RC K4). The CLI flavor
    /// resolves explicitly via `resolve_cli_out_path` first.
    #[test]
    fn guard_refuses_relative_input_and_cli_resolver_makes_it_explicit() {
        let err = ensure_free_output_path(Path::new("reports/out.json"), &[])
            .expect_err("relative refused");
        assert_eq!(err.kind, PathGuardErrorKind::NotAbsolute);
        let err = ensure_writable_output_path(Path::new("reports/out.json"), &[Path::new("/")])
            .expect_err("relative refused in allow flavor too");
        assert_eq!(err.kind, PathGuardErrorKind::NotAbsolute);

        // The explicit CLI resolver joins the CWD, after which the guard
        // accepts: the CWD dependence is one visible step, not an OS
        // accident at every fs call.
        let resolved = resolve_cli_out_path(Path::new("reports/out.json")).expect("cwd join");
        assert!(resolved.is_absolute(), "{resolved:?}");
    }

    /// Free-form flavor: a destination that resolves INTO a protected root
    /// is refused — including through a symlink alias planted inside an
    /// innocent-looking directory. An empty protected list is legal (pure
    /// form+resolution check).
    #[test]
    #[cfg(unix)]
    fn free_guard_refuses_protected_root_including_via_symlink() {
        let tmp = tempfile::tempdir().expect("tmp");
        let managed = tmp.path().join("managed");
        let user = tmp.path().join("user");
        std::fs::create_dir_all(&managed).expect("dirs");
        std::fs::create_dir_all(user.join("link")).expect("dirs");
        std::os::unix::fs::symlink(&managed, user.join("link/managed")).expect("symlink");

        // Plain containment in the protected root.
        let err = ensure_free_output_path(&managed.join("x.json"), &[&managed])
            .expect_err("protected root refused");
        assert_eq!(err.kind, PathGuardErrorKind::ProtectedRoot, "{err}");

        // The same destination reached through a symlink alias.
        let via_link = user.join("link/managed/x.json");
        let err = ensure_free_output_path(&via_link, &[&managed])
            .expect_err("symlink alias into protected root refused");
        assert_eq!(err.kind, PathGuardErrorKind::ProtectedRoot, "{err}");

        // A sibling of the protected root stays legitimate.
        let got =
            ensure_free_output_path(&user.join("out.json"), &[&managed]).expect("free path ok");
        assert_eq!(got, canonical_view(&user).unwrap().join("out.json"));
    }
}
