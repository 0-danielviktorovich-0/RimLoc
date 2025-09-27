use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RimLocConfig {
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub game_version: Option<String>,
    pub list_limit: Option<usize>,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("{0}")]
    Other(String),
}

pub fn load_config() -> Result<RimLocConfig, ConfigError> {
    // Search order: CWD/rimloc.toml, $HOME/.config/rimloc/rimloc.toml
    let mut merged = RimLocConfig::default();
    if let Ok(p) = std::env::current_dir() {
        let path = p.join("rimloc.toml");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<RimLocConfig>(&s) {
                merged = merge(merged, cfg);
            }
        }
    }
    if let Some(base) = dirs::config_dir() {
        let path = base.join("rimloc").join("rimloc.toml");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<RimLocConfig>(&s) {
                merged = merge(merged, cfg);
            }
        }
    }
    Ok(merged)
}

fn merge(mut a: RimLocConfig, b: RimLocConfig) -> RimLocConfig {
    if a.source_lang.is_none() { a.source_lang = b.source_lang; }
    if a.target_lang.is_none() { a.target_lang = b.target_lang; }
    if a.game_version.is_none() { a.game_version = b.game_version; }
    if a.list_limit.is_none() { a.list_limit = b.list_limit; }
    a
}

