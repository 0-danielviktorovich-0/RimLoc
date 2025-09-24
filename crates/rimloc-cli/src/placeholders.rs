use regex::Regex;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[allow(dead_code)]
pub fn extract_placeholders(s: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();

    static RE_PCT: OnceLock<Regex> = OnceLock::new();
    let re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
    for m in re_pct.find_iter(s) {
        set.insert(m.as_str().to_string());
    }

    static RE_BRACE: OnceLock<Regex> = OnceLock::new();
    let re_brace = RE_BRACE.get_or_init(|| Regex::new(r"\{[^}]+\}").unwrap());
    for m in re_brace.find_iter(s) {
        set.insert(m.as_str().to_string());
    }

    set
}
