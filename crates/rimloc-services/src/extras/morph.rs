use crate::{scan::scan_units, Result};
use lru::LruCache;
use regex::Regex;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphProvider {
    Dummy,
    MorpherApi,
    Pymorphy2,
}

fn pluralize(s: &str) -> String {
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
            'й' | 'ь' | 'я' => {
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
    let ls = s.trim().to_lowercase();
    if ls.ends_with('a') || ls.ends_with('я') || ls.ends_with('а') || ls.ends_with('ь') {
        "Female"
    } else {
        "Male"
    }
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

#[derive(Debug, Clone)]
pub struct MorphOptions {
    pub provider: MorphProvider,
    pub target_lang_dir: String,
    pub filter_key_regex: Option<String>,
    pub limit: Option<usize>,
    pub timeout_ms: u64,
    pub cache_size: usize,
    pub pymorphy_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MorphResult {
    pub processed: usize,
    pub lang: String,
    pub warn_no_morpher: bool,
    pub warn_no_pymorphy: bool,
}

pub fn generate(root: &Path, opts: &MorphOptions) -> Result<MorphResult> {
    let re_key = opts
        .filter_key_regex
        .as_deref()
        .map(|r| Regex::new(r).unwrap_or_else(|_| Regex::new(".*").unwrap()))
        .unwrap_or_else(|| Regex::new(".*").unwrap());

    let units = scan_units(root)?;
    // Collect up to 'limit' keys/values from target language
    let mut picked: BTreeMap<String, String> = BTreeMap::new();
    for u in &units {
        if picked.len() >= opts.limit.unwrap_or(usize::MAX) {
            break;
        }
        if crate::util::is_under_languages_dir(&u.path, &opts.target_lang_dir)
            && re_key.is_match(u.key.as_str())
        {
            if let Some(val) = u.source.as_deref() {
                picked
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        }
    }

    // Generate forms
    let cache_cap = opts.cache_size.max(1);
    let mut cache = LruCache::new(NonZeroUsize::new(cache_cap).unwrap());
    let morpher_token = std::env::var("MORPHER_TOKEN").ok();
    let pym_url = opts.pymorphy_url.clone();

    let mut case_items: Vec<(String, String)> = Vec::new();
    let mut plural_items: Vec<(String, String)> = Vec::new();
    let mut gender_items: Vec<(String, String)> = Vec::new();

    for (k, base) in picked {
        let mut cases: std::collections::BTreeMap<String, String> = {
            let mut m = std::collections::BTreeMap::new();
            m.insert("Nominative".to_string(), base.clone());
            m.insert("Genitive".to_string(), format!("{}{}", base, "'s"));
            m
        };

        match opts.provider {
            MorphProvider::MorpherApi => {
                if let Some(tok) = morpher_token.as_deref() {
                    if let Some(m) = morpher_decline(tok, &base, opts.timeout_ms, &mut cache) {
                        for (name, val) in m {
                            cases.insert(name, val);
                        }
                    }
                }
            }
            MorphProvider::Pymorphy2 => {
                if let Some(url) = pym_url.as_deref() {
                    if let Some(m) = pymorphy_decline(url, &base, opts.timeout_ms, &mut cache) {
                        for (name, val) in m {
                            cases.insert(name, val);
                        }
                    }
                }
            }
            MorphProvider::Dummy => {}
        }

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
    let out_case = root
        .join("Languages")
        .join(&opts.target_lang_dir)
        .join("Keyed")
        .join("_Case.xml");
    let out_plural = root
        .join("Languages")
        .join(&opts.target_lang_dir)
        .join("Keyed")
        .join("_Plural.xml");
    let out_gender = root
        .join("Languages")
        .join(&opts.target_lang_dir)
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
    Ok(MorphResult {
        processed: processed_total,
        lang: opts.target_lang_dir.clone(),
        warn_no_morpher: opts.provider == MorphProvider::MorpherApi && morpher_token.is_none(),
        warn_no_pymorphy: opts.provider == MorphProvider::Pymorphy2 && pym_url.is_none(),
    })
}
