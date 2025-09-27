use crate::version::resolve_game_version_root;

#[allow(clippy::too_many_arguments)]
pub fn run_learn_defs(
    mod_root: std::path::PathBuf,
    dict: Vec<std::path::PathBuf>,
    model: Option<std::path::PathBuf>,
    ml_url: Option<String>,
    lang_dir: Option<String>,
    threshold: f32,
    out_dir: std::path::PathBuf,
    no_ml: bool,
    retrain: bool,
    learned_out: Option<std::path::PathBuf>,
    retrain_dict: Option<std::path::PathBuf>,
    min_len: Option<usize>,
    blacklist: Option<Vec<String>>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    let (scan_root, _) = resolve_game_version_root(&mod_root, game_version.as_deref())?;
    let opts = rimloc_services::learn::LearnOptions {
        mod_root: scan_root.clone(),
        defs_root: None,
        dict_files: dict,
        model_path: model,
        ml_url,
        lang_dir: lang_dir.unwrap_or_else(|| "English".to_string()),
        threshold,
        no_ml,
        retrain,
        min_len: min_len.unwrap_or(1),
        blacklist: blacklist.unwrap_or_default(),
        out_dir,
        learned_out,
        retrain_dict,
    };
    let res = rimloc_services::learn::learn_defs(&opts)?;
    crate::ui_out!(
        "learn-defs-summary",
        candidates = res.candidates.len(),
        accepted = res.accepted,
        missing = res.missing_path.display().to_string(),
        suggested = res.suggested_path.display().to_string()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_learn_keyed(
    mod_root: std::path::PathBuf,
    dict: Vec<std::path::PathBuf>,
    ml_url: Option<String>,
    source_lang_dir: Option<String>,
    lang_dir: Option<String>,
    threshold: f32,
    out_dir: std::path::PathBuf,
    no_ml: bool,
    learned_out: Option<std::path::PathBuf>,
    retrain_dict: Option<std::path::PathBuf>,
    min_len: Option<usize>,
    blacklist: Option<Vec<String>>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    let (scan_root, _) = resolve_game_version_root(&mod_root, game_version.as_deref())?;
    // load keyed dicts
    let mut dicts = Vec::new();
    for p in dict {
        let pp = if p.is_absolute() { p } else { scan_root.join(p) };
        if let Ok(d) = rimloc_services::learn::keyed::load_keyed_dict_from_file(&pp) { dicts.push(d); }
    }
    let mut classifier: Box<dyn rimloc_services::learn::ml::Classifier> = if no_ml {
        Box::new(rimloc_services::learn::ml::DummyClassifier::new(1.0))
    } else if let Some(url) = ml_url { Box::new(rimloc_services::learn::ml::RestClassifier::new(url)) } else { Box::new(rimloc_services::learn::ml::DummyClassifier::new(0.9)) };
    let src_dir = source_lang_dir.unwrap_or_else(|| "English".to_string());
    let trg_dir = lang_dir.unwrap_or_else(|| "Russian".to_string());
    let missing = rimloc_services::learn::keyed::learn_keyed(
        &scan_root,
        &src_dir,
        &trg_dir,
        &dicts,
        min_len.unwrap_or(1),
        &blacklist.unwrap_or_default(),
        threshold,
        classifier.as_mut(),
    )?;
    std::fs::create_dir_all(&out_dir)?;
    // Save learned set for audit
    {
        #[derive(serde::Serialize)] struct Row<'a> { key: &'a str, value: &'a str, confidence: f32, sourceFile: String, learnedAt: String }
        let now = chrono::Utc::now().to_rfc3339();
        let rows: Vec<Row> = missing.iter().map(|c| Row { key: &c.key, value: &c.value, confidence: c.confidence.unwrap_or(1.0), sourceFile: c.source_file.display().to_string(), learnedAt: now.clone() }).collect();
        let learned_path = learned_out.unwrap_or_else(|| out_dir.join("learned_keyed.json"));
        let file = std::fs::File::create(learned_path)?;
        serde_json::to_writer_pretty(file, &rows)?;
    }
    let miss = out_dir.join("missing_keyed.json");
    rimloc_services::learn::keyed::write_keyed_missing_json(&miss, &missing)?;
    let sug = out_dir.join("_SuggestedKeyed.xml");
    rimloc_services::learn::keyed::write_keyed_suggested_xml(&sug, &missing)?;

    // Retrain: update a dict file (append exact keys as regex ^key$)
    if let Some(p) = retrain_dict.as_ref() {
        let dict0 = if p.exists() { Some(rimloc_services::learn::keyed::load_keyed_dict_from_file(p)?) } else { None };
        let mut include: Vec<String> = dict0.as_ref().and_then(|d| d.include.clone()).unwrap_or_default();
        for c in &missing { include.push(format!("^{}$", regex::escape(&c.key))); }
        include.sort(); include.dedup();
        #[derive(serde::Serialize)] struct KD { include: Vec<String>, #[serde(skip_serializing_if="Option::is_none")] exclude: Option<Vec<String>> }
        let out = KD { include, exclude: dict0.and_then(|d| d.exclude) };
        let file = std::fs::File::create(p)?;
        serde_json::to_writer_pretty(file, &out)?;
    }
    crate::ui_out!(
        "learn-defs-summary",
        candidates = missing.len(),
        accepted = missing.len(),
        missing = miss.display().to_string(),
        suggested = sug.display().to_string()
    );
    Ok(())
}
