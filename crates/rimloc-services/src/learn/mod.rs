pub mod parser;
pub mod dict;
pub mod ml;
pub mod export;

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
    pub min_len: usize,
    pub blacklist: Vec<String>,
    pub out_dir: std::path::PathBuf,
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

    Ok(LearnResult { candidates: cands, accepted: missing_owned.len(), missing_path, suggested_path })
}
