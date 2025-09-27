pub mod parser;
pub mod dict;
pub mod ml;
pub mod export;
pub mod keyed;

use crate::Result;
use parser::{scan_candidates, Candidate};
use dict::{load_dicts, merge_dicts};
use export::{write_missing_json, write_suggested_xml};
use ml::{Classifier, DummyClassifier, RestClassifier};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LearnOptions {
    pub mod_root: std::path::PathBuf,
    pub defs_root: Option<std::path::PathBuf>,
    pub dict_files: Vec<std::path::PathBuf>,
    pub model_path: Option<std::path::PathBuf>,
    pub ml_url: Option<String>,
    pub lang_dir: String,
    pub threshold: f32,
    pub no_ml: bool,
    pub retrain: bool,
    pub retrain_dict: Option<std::path::PathBuf>,
    pub min_len: usize,
    pub blacklist: Vec<String>,
    pub out_dir: std::path::PathBuf,
    pub learned_out: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LearnResult {
    pub candidates: Vec<Candidate>,
    pub accepted: usize,
    pub missing_path: std::path::PathBuf,
    pub suggested_path: std::path::PathBuf,
}

pub fn learn_defs(opts: &LearnOptions) -> Result<LearnResult> {
    // Load dictionaries
    let dict = merge_dicts(load_dicts(&opts.mod_root, &opts.dict_files)?);

    // Scan candidates under Defs
    let defs_root = opts.defs_root.as_deref();
    let mut cands = scan_candidates(&opts.mod_root, defs_root, &dict, opts.min_len, &opts.blacklist)?;

    // Classifier
    let mut classifier: Box<dyn Classifier> = if opts.no_ml {
        Box::new(DummyClassifier::new(1.0))
    } else if let Some(url) = &opts.ml_url {
        Box::new(RestClassifier::new(url.clone()))
    } else {
        // fallback dummy; future: load from model_path
        Box::new(DummyClassifier::new(0.9))
    };

    // Score and filter
    for cand in &mut cands {
        let score = classifier.score(cand)?;
        cand.confidence = Some(score);
    }
    let accepted: Vec<&Candidate> = cands
        .iter()
        .filter(|c| c.confidence.unwrap_or(1.0) >= opts.threshold)
        .collect();

    // Build set of existing DefInjected keys for target lang
    let existing = parser::collect_existing_definj_keys(&opts.mod_root, &opts.lang_dir)?;
    let missing_owned: Vec<Candidate> = accepted
        .into_iter()
        .filter(|c| {
            let key = format!("{}.{}", c.def_name, c.field_path);
            !existing.contains(&key)
        })
        .map(|c| (*c).clone())
        .collect();

    // Write reports
    std::fs::create_dir_all(&opts.out_dir)?;
    let missing_path = opts.out_dir.join("missing_keys.json");
    let missing_refs: Vec<&Candidate> = missing_owned.iter().collect();
    write_missing_json(&missing_path, &missing_refs)?;
    let suggested_path = opts.out_dir.join("suggested.xml");
    write_suggested_xml(&suggested_path, &missing_refs)?;

    // Save learned set for audit
    {
        #[derive(serde::Serialize)]
        struct Row<'a> { defType: &'a str, defName: &'a str, fieldPath: &'a str, confidence: f32, sourceFile: String, learnedAt: String }
        let now = chrono::Utc::now().to_rfc3339();
        let rows: Vec<Row> = missing_owned.iter().map(|c| Row {
            defType: &c.def_type,
            defName: &c.def_name,
            fieldPath: &c.field_path,
            confidence: c.confidence.unwrap_or(1.0),
            sourceFile: c.source_file.display().to_string(),
            learnedAt: now.clone(),
        }).collect();
        let learned_path = opts.learned_out.clone().unwrap_or_else(|| opts.out_dir.join("learned_defs.json"));
        let file = std::fs::File::create(learned_path)?;
        serde_json::to_writer_pretty(file, &rows)?;
    }

    // Retrain: append to dict
    if opts.retrain {
        use std::collections::{BTreeSet, HashMap};
        let base = merge_dicts(load_dicts(&opts.mod_root, &opts.dict_files)?);
        let mut merged: HashMap<String, BTreeSet<String>> = HashMap::new();
        for (k, v) in base { merged.insert(k, v.into_iter().collect()); }
        for c in &missing_owned { merged.entry(c.def_type.clone()).or_default().insert(c.field_path.clone()); }
        let mut flat: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for (k, v) in merged { flat.insert(k, v.into_iter().collect()); }
        let out_path = if let Some(p) = &opts.retrain_dict {
            p.clone()
        } else if let Some(first) = opts.dict_files.first() {
            let mut name = first.file_name().and_then(|s| s.to_str()).unwrap_or("defs_dict.json").to_string();
            if let Some(idx) = name.rfind('.') { name.replace_range(idx.., ".updated.json"); } else { name.push_str(".updated.json"); }
            first.parent().unwrap_or_else(|| std::path::Path::new(".")).join(name)
        } else {
            opts.out_dir.join("defs_dict.updated.json")
        };
        let file = std::fs::File::create(out_path)?;
        serde_json::to_writer_pretty(file, &flat)?;
    }

    Ok(LearnResult { candidates: cands, accepted: missing_owned.len(), missing_path, suggested_path })
}
