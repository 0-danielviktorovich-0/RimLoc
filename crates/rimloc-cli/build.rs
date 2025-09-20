use std::{env, fs, path::Path};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let i18n_dir = Path::new(&crate_dir).join("i18n");

    let mut locales: Vec<String> = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&i18n_dir) {
        for entry in read_dir.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                // проверим, что внутри есть хотя бы один .ftl
                let has_ftl = fs::read_dir(entry.path())
                    .ok()
                    .into_iter()
                    .flat_map(|rd| rd.flatten())
                    .any(|e| e.path().extension().map(|e| e=="ftl").unwrap_or(false));
                if has_ftl { locales.push(name); }
            }
        }
    }
    locales.sort();

    let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    let dst = out_dir.join("supported_locales.rs");
    let body = format!(
        "pub static SUPPORTED_LOCALES: &[&str] = &{:?};",
        locales.iter().map(String::as_str).collect::<Vec<_>>()
    );
    fs::write(&dst, body).unwrap();

    // чтобы пересобирать при изменениях i18n/
    println!("cargo:rerun-if-changed=i18n");
}