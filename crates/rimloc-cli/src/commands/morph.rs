use crate::version::resolve_game_version_root;
use lru::LruCache;
use std::io::IsTerminal;
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub enum MorphProvider {
    Dummy,
    MorpherApi,
    Pymorphy2,
}

impl MorphProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "morpher" | "morpher_api" => Self::MorpherApi,
            "pymorphy2" | "pymorphy" => Self::Pymorphy2,
            _ => Self::Dummy,
        }
    }
}

fn pluralize(s: &str) -> String {
    // Heuristics:
    // - Latin: add 's'
    // - Cyrillic:
    //   * й → и
    //   * ь → и
    //   * я → и
    //   * а → ы (but → и after г,к,х,ж,ч,ш,щ)
    //   * ж/ч/ш/щ (no vowel change) → +и
    //   * default → +ы
    let has_cyr = s.chars().any(|c| (c as u32) >= 0x0400);
    if !has_cyr {
        return format!("{}s", s);
    }
    let lower = s.trim().to_lowercase();
    let mut chars: Vec<char> = lower.chars().collect();
    if let Some(&last) = chars.last() {
        let prev = chars
            .get(chars.len().saturating_sub(2))
            .copied()
            .unwrap_or('\0');
        match last {
            'й' => {
                chars.pop();
                chars.push('и');
                return chars.iter().collect();
            }
            'ь' => {
                chars.pop();
                chars.push('и');
                return chars.iter().collect();
            }
            'я' => {
                chars.pop();
                chars.push('и');
                return chars.iter().collect();
            }
            'а' => {
                let hush = matches!(prev, 'г' | 'к' | 'х' | 'ж' | 'ч' | 'ш' | 'щ');
                let repl = if hush { 'и' } else { 'ы' };
                chars.pop();
                chars.push(repl);
                return chars.iter().collect();
            }
            'ж' | 'ч' | 'ш' | 'щ' => {
                return format!("{}{}", s, 'и');
            }
            _ => {}
        }
    }
    format!("{}{}", s, 'ы')
}

fn guess_gender(s: &str) -> &'static str {
    // Heuristics: for Cyrillic nouns treat endings 'а', 'я', 'ь' as Female, else Male.
    // For Latin fallback: 'a' → Female.
    let ls = s.trim().to_lowercase();
    if ls.ends_with('a') || ls.ends_with('я') || ls.ends_with('а') || ls.ends_with('ь') {
        "Female"
    } else {
        "Male"
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_morph(
    root: std::path::PathBuf,
    provider: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    filter_key_regex: Option<String>,
    limit: Option<usize>,
    game_version: Option<String>,
    timeout_ms: Option<u64>,
    cache_size: Option<usize>,
    pymorphy_url: Option<String>,
) -> color_eyre::Result<()> {
    use regex::Regex;
    use std::collections::BTreeMap;

    let provider = MorphProvider::from_str(provider.as_deref().unwrap_or("dummy"));
    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "morph_version_resolved", version = ver, path = %scan_root.display());
    }

    let target_lang = if let Some(dir) = lang_dir {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };

    let http_timeout = timeout_ms.unwrap_or(1500);
    let opts = rimloc_services::MorphOptions {
        provider: match provider {
            MorphProvider::MorpherApi => rimloc_services::MorphProvider::MorpherApi,
            MorphProvider::Pymorphy2 => rimloc_services::MorphProvider::Pymorphy2,
            _ => rimloc_services::MorphProvider::Dummy,
        },
        target_lang_dir: target_lang.clone(),
        filter_key_regex,
        limit,
        timeout_ms: http_timeout,
        cache_size: cache_size.unwrap_or(1024),
        pymorphy_url,
    };
    let res = rimloc_services::morph_generate(&scan_root, &opts)?;
    crate::ui_ok!("morph-summary", processed = (res.processed as i64), lang = res.lang.as_str());
    if res.warn_no_morpher { crate::ui_warn!("morph-provider-morpher-stub"); }
    if res.warn_no_pymorphy { crate::ui_warn!("morph-provider-morpher-stub"); }
    Ok(())
}
fn morpher_decline(
    token: &str,
    word: &str,
    timeout_ms: u64,
    cache: &mut LruCache<String, std::collections::HashMap<String, String>>,
) -> Option<std::collections::HashMap<String, String>> {
    if let Some(v) = cache.get(word) {
        return Some(v.clone());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .ok()?;
    // WS3 Russian declension endpoint
    let url = format!(
        "https://ws3.morpher.ru/russian/declension?s={}",
        urlencoding::encode(word)
    );
    let req = client
        .get(url)
        .query(&[("format", "json")])
        .header("Authorization", format!("Basic {}", token));
    let res = req.send().ok()?;
    if !res.status().is_success() {
        return None;
    }
    let v: serde_json::Value = res.json().ok()?;
    let mut map = std::collections::HashMap::new();
    // map typical cases present in response
    for (k, key) in [
        ("И", "Nominative"),
        ("Р", "Genitive"),
        ("Д", "Dative"),
        ("В", "Accusative"),
        ("Т", "Instrumental"),
        ("П", "Prepositional"),
    ] {
        if let Some(val) = v.get(k).and_then(|x| x.as_str()) {
            map.insert(key.to_string(), val.to_string());
        }
    }
    if map.is_empty() {
        return None;
    }
    cache.put(word.to_string(), map.clone());
    Some(map)
}

fn pymorphy_decline(
    url: &str,
    word: &str,
    timeout_ms: u64,
    cache: &mut LruCache<String, std::collections::HashMap<String, String>>,
) -> Option<std::collections::HashMap<String, String>> {
    let key = format!("py:{}", word);
    if let Some(v) = cache.get(&key) {
        return Some(v.clone());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .ok()?;
    let req = client
        .get(format!("{}/declension", url.trim_end_matches('/')))
        .query(&[("text", word)])
        .header("Accept", "application/json");
    let res = req.send().ok()?;
    if !res.status().is_success() {
        return None;
    }
    let v: serde_json::Value = res.json().ok()?;
    let mut map = std::collections::HashMap::new();
    // Map pymorphy2 tags to English case names used by RimLoc
    for (k, key) in [
        ("nomn", "Nominative"),
        ("gent", "Genitive"),
        ("datv", "Dative"),
        ("accs", "Accusative"),
        ("ablt", "Instrumental"),
        ("loct", "Prepositional"),
    ] {
        if let Some(val) = v.get(k).and_then(|x| x.as_str()) {
            map.insert(key.to_string(), val.to_string());
        }
    }
    if map.is_empty() {
        return None;
    }
    cache.put(key, map.clone());
    Some(map)
}
