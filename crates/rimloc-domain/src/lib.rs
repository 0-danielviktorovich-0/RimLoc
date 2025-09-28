use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScanUnit {
    pub schema_version: u32,
    pub path: String,
    pub line: Option<usize>,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationMsg {
    pub schema_version: u32,
    pub kind: String,
    pub key: String,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportFileStat {
    pub path: String,
    pub keys: usize,
    pub status: String,
    pub added: Vec<String>,
    pub changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportSummary {
    pub mode: String,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub keys: usize,
    pub files: Vec<ImportFileStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffOutput {
    pub changed: Vec<(String, String)>,
    pub only_in_translation: Vec<String>,
    pub only_in_mod: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthIssue {
    pub path: String,
    pub category: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HealthReport {
    pub checked: usize,
    pub issues: Vec<HealthIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnnotateFilePlan {
    pub path: String,
    pub add: usize,
    pub strip: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnnotatePlan {
    pub files: Vec<AnnotateFilePlan>,
    pub total_add: usize,
    pub total_strip: usize,
    pub processed: usize,
}
