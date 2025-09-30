use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

#[derive(serde::Serialize)]
struct Unit<'a> {
    key: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

fn scan_ftl(root: &Path) -> Vec<Unit<'static>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() { continue; }
        if p.extension().and_then(|s| s.to_str()).map(|ext| ext.eq_ignore_ascii_case("ftl")).unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(p) {
                let mut line_no = 0usize;
                for line in content.lines() {
                    line_no += 1;
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
                    if trimmed.starts_with('-') { continue; } // skip terms
                    if let Some((key, val)) = trimmed.split_once('=') {
                        let k = key.trim();
                        if k.is_empty() { continue; }
                        let v = val.trim();
                        out.push(Unit{
                            key: Box::leak(k.to_string().into_boxed_str()),
                            source: if v.is_empty() { None } else { Some(Box::leak(v.to_string().into_boxed_str())) },
                            path: p.display().to_string(),
                            line: Some(line_no),
                        });
                    }
                }
            }
        }
    }
    out
}

#[no_mangle]
pub extern "C" fn rimloc_plugin_scan_json(root: *const c_char) -> *mut c_char {
    if root.is_null() {
        return std::ptr::null_mut();
    }
    let c = unsafe { CStr::from_ptr(root) };
    let path_str = match c.to_str() { Ok(s) => s, Err(_) => return std::ptr::null_mut() };
    let units = scan_ftl(Path::new(path_str));
    match serde_json::to_string(&units) {
        Ok(s) => CString::new(s).unwrap().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

