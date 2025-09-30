use std::{collections::BTreeSet, fs, path::Path};

fn load_ids(dir: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n").join(dir);
    for entry in fs::read_dir(&base).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|e| e.to_str()) == Some("ftl") {
            // codeql[rust/path-injection]: test-only code reading known fixture files under i18n/
            let text = fs::read_to_string(entry.path()).unwrap();
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
