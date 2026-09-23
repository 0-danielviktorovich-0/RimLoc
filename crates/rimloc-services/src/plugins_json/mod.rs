use crate::{plugins, Result, TransUnit};
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

pub struct JsonKeyedPlugin;

impl plugins::ParserPlugin for JsonKeyedPlugin {
    fn id(&self) -> &'static str {
        "json-keyed"
    }

    fn scan_units(&self, root: &Path) -> Result<Vec<TransUnit>> {
        let mut out = Vec::new();
        let include = std::env::var("RIMLOC_JSON_PATH_INCLUDE").ok();
        let exclude = std::env::var("RIMLOC_JSON_PATH_EXCLUDE").ok();
        let cfg = rimloc_config::load_config().unwrap_or_default();
        let (cfg_inc, cfg_exc) = cfg
            .plugins
            .as_ref()
            .map(|p| {
                (
                    p.json_include.clone().unwrap_or_default(),
                    p.json_exclude.clone().unwrap_or_default(),
                )
            })
            .unwrap_or_default();
        let incl_res: Option<Vec<regex::Regex>> = include.as_deref().map(|s| {
            s.split(',')
                .filter_map(|p| regex::Regex::new(p.trim()).ok())
                .collect()
        });
        let excl_res: Option<Vec<regex::Regex>> = exclude.as_deref().map(|s| {
            s.split(',')
                .filter_map(|p| regex::Regex::new(p.trim()).ok())
                .collect()
        });
        let mut incl_all: Vec<regex::Regex> = Vec::new();
        if let Some(v) = incl_res.as_ref() {
            incl_all.extend(v.clone());
        }
        for ptn in cfg_inc {
            if let Ok(r) = regex::Regex::new(&ptn) {
                incl_all.push(r);
            }
        }
        let mut excl_all: Vec<regex::Regex> = Vec::new();
        if let Some(v) = excl_res.as_ref() {
            excl_all.extend(v.clone());
        }
        for ptn in cfg_exc {
            if let Ok(r) = regex::Regex::new(&ptn) {
                excl_all.push(r);
            }
        }
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let is_json = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or(false);
            if !is_json {
                continue;
            }
            let norm = p.to_string_lossy().replace('\\', "/");
            if !incl_all.is_empty() && !incl_all.iter().any(|r| r.is_match(&norm)) {
                continue;
            }
            if !excl_all.is_empty() && excl_all.iter().any(|r| r.is_match(&norm)) {
                continue;
            }
            let content = match std::fs::read_to_string(p) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let val: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };
            fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
                match v {
                    Value::String(s) => {
                        if !prefix.is_empty() {
                            out.push((prefix.to_string(), s.clone()));
                        }
                    }
                    Value::Array(arr) => {
                        // join string items with newlines
                        let mut items: Vec<String> = Vec::new();
                        for it in arr {
                            if let Value::String(s) = it {
                                items.push(s.clone());
                            }
                        }
                        if !items.is_empty() && !prefix.is_empty() {
                            out.push((prefix.to_string(), items.join("\n")));
                        }
                    }
                    Value::Object(map) => {
                        for (k, vv) in map {
                            let key = if prefix.is_empty() {
                                k.clone()
                            } else {
                                format!("{}.{}", prefix, k)
                            };
                            flatten(&key, vv, out);
                        }
                    }
                    _ => {}
                }
            }
            let mut pairs = Vec::new();
            flatten("", &val, &mut pairs);
            for (k, v) in pairs {
                out.push(TransUnit {
                    key: k,
                    source: Some(v),
                    path: p.to_path_buf(),
                    line: None,
                    tkey: None,
                });
            }
        }
        out.sort_by(|a, b| {
            (
                a.path.to_string_lossy(),
                a.line.unwrap_or(0),
                a.key.as_str(),
            )
                .cmp(&(
                    b.path.to_string_lossy(),
                    b.line.unwrap_or(0),
                    b.key.as_str(),
                ))
        });
        Ok(out)
    }
}
