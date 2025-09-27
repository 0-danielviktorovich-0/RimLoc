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
    // naive pluralization: english: add 's'; cyrillic: add 'ы'
    if s.chars().any(|c| (c as u32) >= 0x0400) {
        format!("{}{}", s, "ы")
    } else {
        format!("{}{}", s, "s")
    }
}

fn guess_gender(s: &str) -> &'static str {
    // naive: words ending with 'a'/'я'/'а' -> female; else male
    let ls = s.trim().to_lowercase();
    if ls.ends_with('a') || ls.ends_with('я') || ls.ends_with('а') {
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

    let re_key = filter_key_regex
        .as_deref()
        .map(|r| Regex::new(r).unwrap_or_else(|_| Regex::new(".*").unwrap()))
        .unwrap_or_else(|| Regex::new(".*").unwrap());

    let units = rimloc_parsers_xml::scan_keyed_xml(&scan_root)?;
    // Collect up to 'limit' keys/values from target language
    let mut picked: BTreeMap<String, String> = BTreeMap::new();
    for u in &units {
        if picked.len() >= limit.unwrap_or(usize::MAX) {
            break;
        }
        if crate::is_under_languages_dir(&u.path, &target_lang) && re_key.is_match(u.key.as_str()) {
            if let Some(val) = u.source.as_deref() {
                picked
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        }
    }

    // Generate forms
    let cache_cap = cache_size.unwrap_or(1024).max(1);
    let mut cache = LruCache::new(NonZeroUsize::new(cache_cap).unwrap());
    let morpher_token = std::env::var("MORPHER_TOKEN").ok();
    // CLI flag takes precedence over ENV
    let pym_url = pymorphy_url.or_else(|| std::env::var("PYMORPHY_URL").ok());
    let http_timeout = timeout_ms.unwrap_or(1500);
    let mut case_items: Vec<(String, String)> = Vec::new();
    let mut plural_items: Vec<(String, String)> = Vec::new();
    let mut gender_items: Vec<(String, String)> = Vec::new();

    for (k, base) in picked {
        // Build a map of cases; start with naive defaults, then overlay provider results
        let mut cases: std::collections::BTreeMap<String, String> = {
            let mut m = std::collections::BTreeMap::new();
            m.insert("Nominative".to_string(), base.clone());
            m.insert("Genitive".to_string(), format!("{}{}", base, "'s"));
            m
        };

        match provider {
            MorphProvider::MorpherApi => {
                if let Some(tok) = morpher_token.as_deref() {
                    if let Some(m) = morpher_decline(tok, &base, http_timeout, &mut cache) {
                        for (name, val) in m {
                            cases.insert(name, val);
                        }
                    }
                }
            }
            MorphProvider::Pymorphy2 => {
                if let Some(url) = pym_url.as_deref() {
                    if let Some(m) = pymorphy_decline(url, &base, http_timeout, &mut cache) {
                        for (name, val) in m {
                            cases.insert(name, val);
                        }
                    }
                }
            }
            _ => {}
        }
        // Emit known case set if present
        for cname in [
            "Nominative",
            "Genitive",
            "Dative",
            "Accusative",
            "Instrumental",
            "Prepositional",
        ] {
            if let Some(v) = cases.get(cname) {
                case_items.push((format!("Case.{}.{}", k, cname), v.clone()));
            }
        }
        plural_items.push((format!("Plural.{}", k), pluralize(&base)));
        gender_items.push((format!("Gender.{}", k), guess_gender(&base).to_string()));
    }

    // Write under Languages/<lang>/Keyed
    let out_case = scan_root
        .join("Languages")
        .join(&target_lang)
        .join("Keyed")
        .join("_Case.xml");
    let out_plural = scan_root
        .join("Languages")
        .join(&target_lang)
        .join("Keyed")
        .join("_Plural.xml");
    let out_gender = scan_root
        .join("Languages")
        .join(&target_lang)
        .join("Keyed")
        .join("_Gender.xml");

    if !case_items.is_empty() {
        rimloc_import_po::write_language_data_xml(&out_case, &case_items)?;
    }
    if !plural_items.is_empty() {
        rimloc_import_po::write_language_data_xml(&out_plural, &plural_items)?;
    }
    if !gender_items.is_empty() {
        rimloc_import_po::write_language_data_xml(&out_gender, &gender_items)?;
    }

    let processed_total: usize = case_items.len() + plural_items.len() + gender_items.len();
    crate::ui_ok!(
        "morph-summary",
        processed = (processed_total as i64),
        lang = target_lang
    );
    if matches!(provider, MorphProvider::MorpherApi) && morpher_token.is_none() {
        crate::ui_warn!("morph-provider-morpher-stub");
    }
    if matches!(provider, MorphProvider::Pymorphy2) && pym_url.is_none() {
        crate::ui_warn!("morph-provider-morpher-stub");
    }
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
