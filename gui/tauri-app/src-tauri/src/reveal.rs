//! Session reveal guard (§8 F8.4) — the fenced cousin of the privileged
//! legacy `open_path`.
//!
//! A SUCCESSFUL `project_export` / `project_build_mod` ack blesses its
//! `out_dir`; the shell-level `reveal_path` command (thin wrapper in
//! main.rs, same shape as `selfloc_catalog_dir`) opens ONLY a path whose
//! canonical (symlink-resolved) view sits inside one of the blessed roots
//! of THIS session. Anything else — a path typed by hand, a leftover from a
//! previous run, the managed store, the source tree — is a refusal.
//!
//! Containment follows the canonical write-guard orientation
//! (`rimloc_services::util::is_within_allow` semantics): the decision is
//! made on canonical views on BOTH sides, `starts_with` compares whole
//! components (sibling-prefix tricks like `/a/bc` vs `/a/b` cannot pass),
//! and an unresolvable path proves NOTHING — allow decisions never rest on
//! an unverified spelling. `util` is private to rimloc-services, so the
//! two primitives it needs (`canonical_view`) are consumed through the
//! crate-root re-export and the component-wise `Path::starts_with`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Session-scoped allow-list of output roots a successful build/export has
/// acked. Lives as Tauri managed state (`app.manage` in main.rs) — it dies
/// with the process, so "this session" is exactly the app's lifetime and a
/// restarted app starts with an EMPTY list (nothing is revealable before a
/// fresh successful output).
#[derive(Default)]
pub struct RevealState {
    allowed: Mutex<HashSet<PathBuf>>,
}

impl RevealState {
    /// Bless one output directory (called ONLY on a successful ack — the
    /// ack's `out_dir` is the backend-verified real location, reparse-
    /// checked before it reaches this line). The canonical view is stored;
    /// an unresolvable ack path is never blessed (allow orientation).
    pub fn allow(&self, dir: &Path) {
        if let Ok(canon) = rimloc_services::canonical_view(dir) {
            if let Ok(mut set) = self.allowed.lock() {
                set.insert(canon);
            }
        }
    }

    /// Allow-direction containment verdict for `reveal_path`: true ONLY
    /// when the requested path's canonical view is equal to or nested
    /// inside at least one blessed root. Unresolvable candidate ⇒ false;
    /// poisoned lock ⇒ false; empty session list ⇒ false. Fail-closed by
    /// construction, exactly like the write guard's allow face.
    pub fn may_reveal(&self, path: &Path) -> bool {
        let Ok(candidate) = rimloc_services::canonical_view(path) else {
            return false;
        };
        let Ok(set) = self.allowed.lock() else {
            return false;
        };
        set.iter().any(|root| candidate.starts_with(root))
    }
}

#[cfg(test)]
mod tests {
    use super::RevealState;
    use std::path::Path;

    #[test]
    fn empty_session_reveals_nothing() {
        let st = RevealState::default();
        assert!(
            !st.may_reveal(Path::new("/tmp/whatever")),
            "fail-closed: nothing blessed yet"
        );
    }

    #[test]
    fn blessed_root_reveals_itself_and_children_only() {
        let tmp = tempfile::tempdir().expect("tmp");
        let out = tmp.path().join("RimLoc-Export").join("proj-x-Russian");
        std::fs::create_dir_all(&out).expect("mkdir");
        let st = RevealState::default();
        st.allow(&out);
        assert!(st.may_reveal(&out), "the blessed dir itself reveals");
        assert!(
            st.may_reveal(&out.join("About").join("About.xml")),
            "children reveal"
        );
        assert!(
            !st.may_reveal(&tmp.path().join("RimLoc-Export").join("sibling")),
            "a sibling of the blessed dir is refused"
        );
        assert!(
            !st.may_reveal(&tmp.path().join("unrelated")),
            "an unrelated path is refused"
        );
    }

    #[test]
    fn unresolvable_candidate_is_refused() {
        let tmp = tempfile::tempdir().expect("tmp");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).expect("mkdir");
        std::fs::write(out.join("About.xml"), b"<xml/>").expect("write");
        let st = RevealState::default();
        st.allow(&out);
        // `…/About.xml/child` anchors on an existing FILE — its tail could
        // never exist, so canonical_view fails closed and the allow verdict
        // refuses instead of guessing. (A merely not-yet-existing child of a
        // blessed DIR resolves by design — the command's `exists()` check
        // refuses that shape one layer earlier; see the reveal_path wrapper.)
        assert!(
            !st.may_reveal(&out.join("About.xml").join("child")),
            "unresolvable ⇒ refuse"
        );
    }

    #[test]
    fn sibling_prefix_spelling_cannot_pass() {
        let tmp = tempfile::tempdir().expect("tmp");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).expect("mkdir");
        let st = RevealState::default();
        st.allow(&out);
        // Component-wise starts_with: `/…/out` must not bless `/…/out2`.
        let sibling_prefix = tmp.path().join("out2");
        std::fs::create_dir_all(&sibling_prefix).expect("mkdir");
        assert!(!st.may_reveal(&sibling_prefix), "/a/bc is not inside /a/b");
    }

    #[test]
    fn symlink_alias_into_blessed_root_resolves_through() {
        let tmp = tempfile::tempdir().expect("tmp");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(out.join("About")).expect("mkdir");
        let alias = tmp.path().join("innocent-link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&out, &alias).expect("symlink");
        #[cfg(not(unix))]
        std::os::windows::fs::symlink_dir(&out, &alias).expect("symlink");
        let st = RevealState::default();
        st.allow(&out);
        // The canonical view of the alias lands INSIDE the blessed root —
        // same real-location resolution as the write guard, so a legit
        // alias keeps working while an alias OUT would fail the verdict.
        assert!(
            st.may_reveal(&alias.join("About")),
            "alias into the blessed root reveals"
        );
    }
}
