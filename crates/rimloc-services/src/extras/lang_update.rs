use crate::Result;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LangUpdatePlanFile {
    pub rel_path: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct LangUpdatePlan {
    pub files: Vec<LangUpdatePlanFile>,
    pub total_bytes: u64,
    pub source_lang_dir: String,
    pub target_lang_dir: String,
    pub out_languages_dir: PathBuf,
}

fn open_zip_from_bytes(bytes: &[u8]) -> zip::ZipArchive<std::io::Cursor<&[u8]>> {
    let rdr = std::io::Cursor::new(bytes);
    zip::ZipArchive::new(rdr).expect("invalid zip archive")
}

fn download_repo_zip(repo: &str, branch: Option<&str>) -> Result<Vec<u8>> {
    // Prefer codeload "heads/<branch>"; fallback to default branch when not specified by API call
    let url = if let Some(br) = branch {
        format!("https://codeload.github.com/{repo}/zip/refs/heads/{br}")
    } else {
        // default branch discovery
        let api = format!("https://api.github.com/repos/{repo}");
        let client = reqwest::blocking::Client::builder()
            .user_agent("RimLoc/cli")
            .build()
            .unwrap();
        let resp = client.get(api).send()?;
        let json: serde_json::Value = resp.json()?;
        let br = json
            .get("default_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("master");
        format!("https://codeload.github.com/{repo}/zip/refs/heads/{br}")
    };
    let client = reqwest::blocking::Client::builder()
        .user_agent("RimLoc/cli")
        .build()
        .unwrap();
    let mut resp = client.get(url).send()?;
    let mut buf: Vec<u8> = Vec::new();
    resp.copy_to(&mut buf)?;
    Ok(buf)
}

/// Build a plan by reading a zip (either downloaded or provided locally) and collecting files under Core/Languages/<source_lang_dir>
fn plan_from_zip_bytes(
    bytes: &[u8],
    source_lang_dir: &str,
    target_lang_dir: &str,
    out_languages_dir: &Path,
) -> Result<LangUpdatePlan> {
    let mut zip = open_zip_from_bytes(bytes);
    let mut files: Vec<LangUpdatePlanFile> = Vec::new();
    let mut total_bytes: u64 = 0;
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_string();
        // zip root prefix: <repo>-<hash>/**
        // we need paths like: <root>/Core/Languages/<source_lang_dir>/...
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() >= 5
            && parts[1] == "Core"
            && parts[2] == "Languages"
            && parts[3] == source_lang_dir
        {
            // rel path under <source_lang_dir>
            let rel = parts[4..].join("/");
            // target path will be out_languages_dir/<target_lang_dir>/<rel>
            let size = entry.size();
            files.push(LangUpdatePlanFile {
                rel_path: rel,
                size,
            });
            total_bytes += size;
        }
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(LangUpdatePlan {
        files,
        total_bytes,
        source_lang_dir: source_lang_dir.to_string(),
        target_lang_dir: target_lang_dir.to_string(),
        out_languages_dir: out_languages_dir.to_path_buf(),
    })
}

/// Apply plan by extracting entries and writing them to the target languages dir.
fn apply_plan(bytes: &[u8], plan: &LangUpdatePlan, backup: bool) -> Result<()> {
    let lang_dir = plan.out_languages_dir.join(&plan.target_lang_dir);
    if lang_dir.exists() && backup {
        let bak = lang_dir.with_extension("bak");
        if bak.exists() {
            fs::remove_dir_all(&bak).ok();
        }
        fs::rename(&lang_dir, &bak)?;
    }
    fs::create_dir_all(&lang_dir)?;

    let mut zip = open_zip_from_bytes(bytes);
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_string();
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() >= 5
            && parts[1] == "Core"
            && parts[2] == "Languages"
            && parts[3] == plan.source_lang_dir
        {
            let rel = parts[4..].join("/");
            let out_path = lang_dir.join(rel);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buf)?;
            let mut f = fs::File::create(out_path)?;
            f.write_all(&buf)?;
        }
    }
    Ok(())
}

pub struct LangUpdateSummary {
    pub files: usize,
    pub bytes: u64,
    pub out_dir: PathBuf,
}

#[allow(clippy::too_many_arguments)]
pub fn lang_update(
    game_root: &Path,
    repo: &str,
    branch: Option<&str>,
    zip_path: Option<&Path>,
    source_lang_dir: &str,
    target_lang_dir: &str,
    dry_run: bool,
    backup: bool,
) -> Result<(Option<LangUpdatePlan>, Option<LangUpdateSummary>)> {
    let out_languages_dir = game_root.join("Data").join("Core").join("Languages");
    fs::create_dir_all(&out_languages_dir)?;

    let bytes: Vec<u8> = if let Some(p) = zip_path {
        fs::read(p)?
    } else {
        download_repo_zip(repo, branch)?
    };

    let plan = plan_from_zip_bytes(&bytes, source_lang_dir, target_lang_dir, &out_languages_dir)?;

    if dry_run {
        return Ok((Some(plan), None));
    }
    apply_plan(&bytes, &plan, backup)?;
    Ok((
        None,
        Some(LangUpdateSummary {
            files: plan.files.len(),
            bytes: plan.total_bytes,
            out_dir: out_languages_dir.join(target_lang_dir),
        }),
    ))
}
