use crate::{Result, TransUnit};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub trait ParserPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn scan_units(&self, root: &std::path::Path) -> Result<Vec<TransUnit>>;
}

static REGISTRY: OnceCell<RwLock<Vec<Arc<dyn ParserPlugin>>>> = OnceCell::new();

pub fn register(p: Arc<dyn ParserPlugin>) {
    REGISTRY
        .get_or_init(|| RwLock::new(Vec::new()))
        .write()
        .unwrap()
        .push(p);
}

pub fn iter() -> Vec<Arc<dyn ParserPlugin>> {
    let guard = REGISTRY
        .get_or_init(|| RwLock::new(Vec::new()))
        .read()
        .unwrap();
    guard.iter().cloned().collect()
}

pub fn init_builtin() {
    // Register built-in plugins exactly once
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register(Arc::new(
            crate::plugins_xml_ext::XmlExtensionsSettingsPlugin,
        ));
        register(Arc::new(crate::plugins_msf::ModSettingsFrameworkPlugin));
    });
}

// --- Dynamic plugin loading (optional) ---

struct DynPlugin {
    id: String,
    #[allow(dead_code)]
    lib: libloading::Library,
    func: rimloc_plugin_api::ScanJsonFn,
}

impl ParserPlugin for DynPlugin {
    fn id(&self) -> &'static str {
        Box::leak(self.id.clone().into_boxed_str())
    }
    fn scan_units(&self, root: &std::path::Path) -> Result<Vec<TransUnit>> {
        use std::ffi::CString;
        let c_root = CString::new(root.to_string_lossy().as_bytes()).unwrap();
        let s_ptr = unsafe { (self.func)(c_root.as_ptr()) };
        if s_ptr.is_null() {
            return Ok(Vec::new());
        }
        let c_str = unsafe { std::ffi::CString::from_raw(s_ptr) };
        let json = c_str.to_string_lossy().to_string();
        #[derive(serde::Deserialize)]
        struct Unit {
            key: String,
            #[serde(default)]
            source: Option<String>,
            path: String,
            #[serde(default)]
            line: Option<usize>,
        }
        let units: Vec<Unit> = serde_json::from_str(&json).unwrap_or_default();
        Ok(units
            .into_iter()
            .map(|u| TransUnit {
                key: u.key,
                source: u.source,
                path: PathBuf::from(u.path),
                line: u.line,
            })
            .collect())
    }
}

fn is_dyn_lib(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let e = ext.to_ascii_lowercase();
        return e == "so" || e == "dll" || e == "dylib";
    }
    false
}

pub fn load_dynamic_plugin(path: &Path) -> Result<()> {
    if !is_dyn_lib(path) {
        return Ok(());
    }
    unsafe {
        let lib = libloading::Library::new(path)?;
        let symbol_name =
            std::ffi::CStr::from_bytes_with_nul_unchecked(rimloc_plugin_api::SCAN_JSON_SYMBOL);
        let func: libloading::Symbol<rimloc_plugin_api::ScanJsonFn> =
            lib.get(symbol_name.to_bytes())?;
        let id = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("dyn")
            .to_string();
        let fn_ptr: rimloc_plugin_api::ScanJsonFn = *func;
        register(Arc::new(DynPlugin {
            id,
            lib,
            func: fn_ptr,
        }));
        Ok(())
    }
}

pub fn load_dynamic_plugins_from(dir: &Path) -> Result<usize> {
    let mut count = 0usize;
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let p = entry?.path();
            if is_dyn_lib(&p) {
                let _ = load_dynamic_plugin(&p);
                count += 1;
            }
        }
    }
    Ok(count)
}

pub fn load_plugins_from_env() -> Result<usize> {
    let mut loaded = 0usize;
    if let Ok(val) = std::env::var("RIMLOC_PLUGINS") {
        for token in val.split([';', ':']) {
            let t = token.trim();
            if t.is_empty() {
                continue;
            }
            let p = PathBuf::from(t);
            if p.is_dir() {
                loaded += load_dynamic_plugins_from(&p)?;
            } else {
                let _ = load_dynamic_plugin(&p);
                loaded += 1;
            }
        }
    }
    Ok(loaded)
}

pub fn run_scan_plugins(root: &Path) -> Result<Vec<TransUnit>> {
    let mut all = Vec::new();
    for p in iter() {
        match p.scan_units(root) {
            Ok(mut v) => {
                all.append(&mut v);
            }
            Err(_) => { /* ignore plugin failure */ }
        }
    }
    all.sort_by(|a, b| {
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
    Ok(all)
}
