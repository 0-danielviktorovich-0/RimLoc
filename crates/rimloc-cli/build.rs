use std::{env, fs, path::Path};

/// Build script that discovers available locales and generates a small Rust file
/// with a `SUPPORTED_LOCALES` constant. We also emit fine‑grained
/// `rerun-if-changed` instructions for each `*.ftl` so changes in translations
/// retrigger rebuilds.
///
/// This script is intentionally "dumb/simple": it treats any subdirectory of
/// `i18n/` that contains at least one `.ftl` file as a valid locale.
///
/// Notes:
/// - We don't parse FTL here; i18n-embed does proper loading at runtime.
/// - We keep this script dependency-free (besides `cargo-emit`) to speed up builds.
fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let i18n_dir = Path::new(&crate_dir).join("i18n");

    let mut locales: Vec<String> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(&i18n_dir) {
        for entry in read_dir.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let locale_name = entry.file_name().to_string_lossy().to_string();

                // Check if the locale folder contains at least one .ftl file.
                let mut has_ftl = false;
                if let Ok(sub) = fs::read_dir(entry.path()) {
                    for f in sub.flatten() {
                        if f.path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("ftl"))
                        {
                            has_ftl = true;

                            // Emit fine-grained triggers for each translation file.
                            cargo_emit::rerun_if_changed!(f.path().to_string_lossy());
                        }
                    }
                }

                if has_ftl {
                    locales.push(locale_name);
                }
            }
        }
    }

    // Sort locales deterministically to avoid churn in the generated file.
    locales.sort();

    // Write generated Rust with the discovered locales.
    let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    let dst = out_dir.join("supported_locales.rs");
    let body = format!(
        "pub static SUPPORTED_LOCALES: &[&str] = &{:?};",
        locales.iter().map(String::as_str).collect::<Vec<_>>()
    );
    fs::write(&dst, body).unwrap();

    // Also watch the i18n root & this script itself (belt-and-suspenders).
    cargo_emit::rerun_if_changed!("i18n/");
    cargo_emit::rerun_if_changed!("build.rs");
}
