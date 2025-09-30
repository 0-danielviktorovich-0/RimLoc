use std::{collections::BTreeSet, fs, path::Path};

fn load_ids(dir: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    // Only allow known fixture dirs
    assert!(matches!(dir, "en" | "ru"), "invalid i18n dir: {dir}");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = repo_root.join("i18n").join(dir);

    // Normalize and ensure we stay within the repository (defensive for scanners)
    let repo_root = repo_root
        .canonicalize()
        .expect("canonicalize repo root for tests");
    let base = base
        .canonicalize()
        .expect("canonicalize i18n base for tests");
    assert!(base.starts_with(&repo_root), "i18n base escaped repo root");

    for entry in fs::read_dir(&base).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("ftl") {
            continue;
        }

        // Resolve and ensure the file is under the expected base dir
        let canon = path.canonicalize().expect("canonicalize ftl file");
        assert!(canon.starts_with(&base), "i18n file escaped base dir");

        let text = fs::read_to_string(&canon).unwrap();
        for line in text.lines() {
            // грубый, но практичный парс: message-id до '=' в начале строки
            if let Some((id, _rest)) = line.split_once('=') {
                let id = id.trim();
                if !id.is_empty() && !id.starts_with('#') && id.chars().all(|c| c != ' ') {
                    ids.insert(id.to_string());
                }
            }
        }
    }
    ids
}

#[test]
fn ru_has_all_en_keys() {
    let en = load_ids("en");
    let ru = load_ids("ru");
    let missing: Vec<_> = en.difference(&ru).collect();
    assert!(
        missing.is_empty(),
        "ru is missing keys found in en: {missing:#?}"
    );
}
