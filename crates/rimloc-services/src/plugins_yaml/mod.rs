use crate::{plugins, Result, TransUnit};
use std::path::Path;
use walkdir::WalkDir;

pub struct YamlKeyedPlugin;

impl plugins::ParserPlugin for YamlKeyedPlugin {
    fn id(&self) -> &'static str { "yaml-keyed" }

    fn scan_units(&self, root: &Path) -> Result<Vec<TransUnit>> {
        let mut out = Vec::new();
        let include = std::env::var("RIMLOC_YAML_PATH_INCLUDE").ok();
        let exclude = std::env::var("RIMLOC_YAML_PATH_EXCLUDE").ok();
        let incl_res: Option<Vec<regex::Regex>> = include.as_deref().map(|s| s.split(',').filter_map(|p| regex::Regex::new(p.trim()).ok()).collect());
        let excl_res: Option<Vec<regex::Regex>> = exclude.as_deref().map(|s| s.split(',').filter_map(|p| regex::Regex::new(p.trim()).ok()).collect());
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_file() { continue; }
            let is_yaml = p.extension().and_then(|e| e.to_str()).map(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml")).unwrap_or(false);
            if !is_yaml { continue; }
            let norm = p.to_string_lossy().replace('\\', "/");
            if let Some(res) = incl_res.as_ref() { if !res.iter().any(|r| r.is_match(&norm)) { continue; } }
            if let Some(res) = excl_res.as_ref() { if res.iter().any(|r| r.is_match(&norm)) { continue; } }
            let content = match std::fs::read_to_string(p) { Ok(s) => s, Err(_) => continue };
            let val: serde_yaml::Value = match serde_yaml::from_str(&content) { Ok(v) => v, Err(_) => continue };
            fn flatten(prefix: &str, v: &serde_yaml::Value, out: &mut Vec<(String, String)>) {
                match v {
                    serde_yaml::Value::String(s) => { if !prefix.is_empty() { out.push((prefix.to_string(), s.clone())); } }
                    serde_yaml::Value::Sequence(arr) => {
                        let mut items: Vec<String> = Vec::new();
                        for it in arr { if let serde_yaml::Value::String(s) = it { items.push(s.clone()); } }
                        if !items.is_empty() && !prefix.is_empty() { out.push((prefix.to_string(), items.join("\n"))); }
                    }
                    serde_yaml::Value::Mapping(map) => {
                        for (k, vv) in map {
                            if let serde_yaml::Value::String(ks) = k {
                                let key = if prefix.is_empty() { ks.clone() } else { format!("{}.{}", prefix, ks) };
                                flatten(&key, vv, out);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let mut pairs = Vec::new();
            flatten("", &val, &mut pairs);
            for (k, v) in pairs {
                out.push(TransUnit { key: k, source: Some(v), path: p.to_path_buf(), line: None });
            }
        }
        out.sort_by(|a, b| (a.path.to_string_lossy(), a.line.unwrap_or(0), a.key.as_str()).cmp(&(b.path.to_string_lossy(), b.line.unwrap_or(0), b.key.as_str())));
        Ok(out)
    }
}
