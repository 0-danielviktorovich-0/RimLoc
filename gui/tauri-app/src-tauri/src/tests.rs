// Enforcement + smoke tests for the Tauri shell (binding wave 2).
// Wired back into the build in this wave: the file had been dead since
// 7fc196f (moved here without a `mod tests;` declaration) and its legacy
// smoke bodies rotted against removed api_* functions — they are gone.
#[cfg(test)]
// Named `shell` (not `tests`) to avoid a module named like its containing
// file-module (clippy module_inception).
mod shell {
    use crate::{LEGACY_PRIVILEGED_COMMANDS, LIVE_COMMANDS};

    /// The ORIGINAL registration (pre-binding-wave main.rs) — the partition
    /// base for the wave-2 deregistration enforcement (lead decision 033 #4).
    const ORIGINAL_REGISTERED: &[&str] = &[
        "get_app_info",
        "scan_mod",
        "learn_defs",
        "export_po",
        "validate_mod",
        "xml_health",
        "import_po",
        "build_mod",
        "diff_xml_cmd",
        "lang_update_cmd",
        "annotate_cmd",
        "init_lang_cmd",
        "get_log_info",
        "pick_directory",
        "save_text_via_dialog",
        "log_message",
        "open_path",
        "set_debug_options",
        "get_diagnostics",
        "collect_diagnostics_via_dialog",
        "simulate_error",
        "simulate_panic",
        "morph_cmd",
        "learn_keyed_cmd",
        "dump_schemas",
        "get_profile",
        "validate_po_gui",
        "learn_patches_cmd",
        "get_cli_i18n",
        "apply_translation",
        "load_tm",
        "scan_strings_gui",
        "load_plugin_cmd",
        "list_plugins_cmd",
        "coverage_gui",
        "export_xliff_gui",
        "import_xliff_gui",
        "merge_keyed_gui",
    ];

    #[test]
    fn live_entry_registers_every_contract_command() {
        for name in rimloc_gui_lib::contract_adapter::CONTRACT_COMMANDS {
            assert!(
                LIVE_COMMANDS.contains(name),
                "contract command `{name}` missing from the live registration"
            );
        }
    }

    #[test]
    fn live_entry_never_registers_privileged_legacy_commands() {
        for name in LEGACY_PRIVILEGED_COMMANDS {
            assert!(
                !LIVE_COMMANDS.contains(name),
                "privileged legacy command `{name}` must not be in the live registration"
            );
        }
    }

    #[test]
    fn live_and_legacy_sets_partition_the_original_registration() {
        for name in ORIGINAL_REGISTERED {
            let in_live = LIVE_COMMANDS.contains(name);
            let in_legacy = LEGACY_PRIVILEGED_COMMANDS.contains(name);
            assert!(
            in_live ^ in_legacy,
            "command `{name}` must live in exactly one registration set (live={in_live}, legacy={in_legacy})"
        );
        }
        // Post-original safe extras (selfloc entry, mandate D) are a
        // deliberate ADDITION on top of the frozen partition base — declared
        // in one constant so the arithmetic stays explicit, not implicit.
        assert_eq!(
            LIVE_COMMANDS.len() + LEGACY_PRIVILEGED_COMMANDS.len(),
            ORIGINAL_REGISTERED.len()
                + rimloc_gui_lib::contract_adapter::CONTRACT_COMMANDS.len()
                + rimloc_gui_lib::selfloc_catalog::POST_ORIGINAL_LIVE_EXTRAS.len()
        );
        for name in rimloc_gui_lib::selfloc_catalog::POST_ORIGINAL_LIVE_EXTRAS {
            assert!(
                LIVE_COMMANDS.contains(name),
                "post-original extra `{name}` must be live-registered"
            );
        }
    }

    #[test]
    fn supplementary_regex_no_privileged_pattern_in_live() {
        let live = LIVE_COMMANDS.join("\n");
        for pattern in [
            "apply_translation",
            "save_text",
            "open_path",
            "plugin",
            "simulate_",
            "morph_cmd",
            "dump_schemas",
            "get_profile",
            "load_tm",
        ] {
            assert!(
                !live.contains(pattern),
                "live registration leaks `{pattern}`"
            );
        }
    }

    /// P2-1/P2-2 (J cross-review): dump_schemas writes 5 schema files into a
    /// caller-chosen directory; get_profile returns raw profile.jsonl records
    /// — both are privileged legacy now, never in the live entry.
    #[test]
    fn dump_schemas_and_get_profile_are_privileged_legacy() {
        for name in ["dump_schemas", "get_profile"] {
            assert!(!LIVE_COMMANDS.contains(&name), "`{name}` must not be live");
            assert!(
                LEGACY_PRIVILEGED_COMMANDS.contains(&name),
                "`{name}` must be in the privileged legacy set"
            );
        }
    }

    /// Night audit P1-2: load_tm reads ANY caller-chosen baseline PO and
    /// walkdir-walks ANY caller-chosen tm_roots with no root containment or
    /// symlink checks — arbitrary read, same exposure class as open_path.
    /// The live entry (and therefore frontend-v2 via the one transport seam)
    /// must never reach it; it returns with root guards via the contract TM
    /// slice.
    #[test]
    fn load_tm_is_privileged_legacy_arbitrary_read() {
        assert!(
            !LIVE_COMMANDS.contains(&"load_tm"),
            "load_tm must not be live: unrestricted caller-chosen paths"
        );
        assert!(
            LEGACY_PRIVILEGED_COMMANDS.contains(&"load_tm"),
            "load_tm must be in the privileged legacy set"
        );
    }
}

/// K4 closeout (2026-09-27): the legacy write surface now enforces the
/// contract form policy on caller-chosen out paths — relative paths are a
/// loud refusal, never a silent resolution against the process CWD.
mod legacy_guards {
    use std::path::Path;

    #[test]
    fn caller_relative_out_paths_are_refused_loudly() {
        let err = crate::ensure_caller_path_absolute("out_dir", Path::new("RimLoc-Export"))
            .expect_err("a relative caller path must be refused");
        assert!(
            err.message.contains("absolute"),
            "error names the fix: {err}"
        );
        assert!(
            err.message.contains("out_dir"),
            "error names the field: {err}"
        );

        // Absolute stays the required, accepted form.
        crate::ensure_caller_path_absolute("out_dir", Path::new("/tmp/rimloc-abs"))
            .expect("an absolute caller path is the required form");
    }

    #[test]
    fn relative_deep_and_traversal_shapes_are_all_refused() {
        for shape in [
            "out.xlf",
            "./docs/assets/schemas",
            "…/RimLoc-Export/proj-x-Russian",
            "out/../more",
        ] {
            let err =
                crate::ensure_caller_path_absolute("out_path", Path::new(shape)).expect_err(shape);
            assert!(err.message.contains("absolute"), "[{shape}] {err}");
        }
    }
}

/// F-1 containment half of the legacy guard (`ensure_legacy_out_path`):
/// after the absolute-form refusal the destination is resolved to its real
/// (symlink-resolved) location via
/// `rimloc_services::ensure_free_output_path` against the RimLoc-managed
/// projects store as the protected root. Adversarial coverage (mandate
/// P0 §8, lane fs-adversarial): the managed store is never written
/// through a legacy out path — not directly, not through a symlink alias,
/// not through a parent-traversal spelling — and the Ok-branch returns the
/// canonical path the caller must write to.
mod legacy_out_containment {
    use std::path::Path;

    fn managed_root() -> std::path::PathBuf {
        crate::contract_adapter::default_managed_root()
    }

    #[test]
    fn managed_store_itself_is_refused() {
        // Equal to the protected root.
        let err = crate::ensure_legacy_out_path("out_json", &managed_root())
            .expect_err("the managed root itself must be refused");
        assert!(err.message.contains("protected root"), "{err}");

        // Nested inside it.
        let err = crate::ensure_legacy_out_path("out_json", &managed_root().join("x.json"))
            .expect_err("a path inside the managed store must be refused");
        assert!(err.message.contains("protected root"), "{err}");
    }

    #[test]
    fn parent_traversal_into_the_managed_store_is_refused() {
        // `<store-parent>/unrelated-tmp/../<store-leaf>/evil.json` lexically
        // wanders OFF into an unrelated sibling and re-enters the store;
        // the canonical view resolves it INSIDE, and the guard must refuse
        // on the real location, not the spelling.
        let store = managed_root();
        let file_name = store.file_name().expect("store has a leaf name").to_owned();
        let traversing = store
            .parent()
            .unwrap_or(Path::new("/"))
            .join("unrelated-tmp")
            .join("..")
            .join(&file_name)
            .join("evil.json");
        let err = crate::ensure_legacy_out_path("out_json", &traversing)
            .expect_err("traversal into the managed store must be refused");
        assert!(err.message.contains("protected root"), "{err}");
        // Nothing was created by the refused attempt.
        assert!(
            !store.join("evil.json").exists(),
            "refused attempt must not write"
        );

        // Control: the honest sibling spelling that does NOT re-enter the
        // store stays legitimate — the refusal is containment, not paranoia.
        let honest_sibling = store
            .parent()
            .unwrap_or(Path::new("/"))
            .join("unrelated-tmp")
            .join("evil.json");
        crate::ensure_legacy_out_path("out_json", &honest_sibling)
            .expect("a genuine sibling of the store is not fenced");
    }

    #[test]
    #[cfg(unix)]
    fn symlink_alias_of_the_managed_store_is_refused() {
        let tmp = tempfile::tempdir().expect("tmp");
        let alias = tmp.path().join("innocent-link");
        std::os::unix::fs::symlink(managed_root(), &alias).expect("symlink");
        let err = crate::ensure_legacy_out_path("out_json", &alias.join("x.json"))
            .expect_err("symlink alias into the managed store must be refused");
        assert!(err.message.contains("protected root"), "{err}");
    }

    #[test]
    fn relative_stays_refused_before_any_fs_access() {
        // The full guard keeps the form refusal FIRST: a relative path is
        // a loud ApiError even when it would resolve inside the store.
        let err = crate::ensure_legacy_out_path("out_json", Path::new("out.json"))
            .expect_err("relative out path refused");
        assert!(err.message.contains("must be an absolute path"), "{err}");
    }

    #[test]
    fn operator_absolute_path_outside_the_store_is_blessed_canonical() {
        let tmp = tempfile::tempdir().expect("tmp");
        let got = crate::ensure_legacy_out_path("out_json", &tmp.path().join("report.json"))
            .expect("an absolute path outside the managed store stays legitimate");
        assert_eq!(
            got,
            rimloc_services::canonical_view(&tmp.path().join("report.json"))
                .expect("canonical view"),
            "the caller receives the canonical path to write to"
        );
        assert!(
            !tmp.path().join("report.json").exists(),
            "guard writes nothing"
        );
    }
}
