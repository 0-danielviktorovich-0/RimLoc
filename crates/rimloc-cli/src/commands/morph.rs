use crate::version::resolve_game_version_root;
use std::io::IsTerminal;

#[derive(Debug, Clone)]
pub enum MorphProvider {
    Dummy,
    MorpherApi,
}

impl MorphProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "morpher" | "morpher_api" => Self::MorpherApi,
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
    let mut case_items: Vec<(String, String)> = Vec::new();
    let mut plural_items: Vec<(String, String)> = Vec::new();
    let mut gender_items: Vec<(String, String)> = Vec::new();

    for (k, base) in picked {
        // Case: extremely naive Nominative/Genitive forms
        case_items.push((format!("Case.{}.Nominative", k), base.clone()));
        case_items.push((format!("Case.{}.Genitive", k), format!("{}{}", base, "'s")));
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
    if matches!(provider, MorphProvider::MorpherApi) {
        crate::ui_warn!("morph-provider-morpher-stub");
    }
    Ok(())
}
