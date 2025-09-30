//! Simple plugin scaffolding for extensible scanners/exporters.
//! Public trait lives in `rimloc-plugin-api`. This module handles registration
//! and dynamic loading for scan plugins.

use crate::{Result, TransUnit};
use once_cell::sync::Lazy;
use rimloc_plugin_api::{ScanJsonFn, ScanPlugin, SCAN_JSON_SYMBOL};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static SCAN_PLUGINS: Lazy<RwLock<Vec<&'static dyn ScanPlugin>>> = Lazy::new(|| RwLock::new(Vec::new()));

pub fn register_scan_plugin(p: &'static dyn ScanPlugin) {
    if let Ok(mut guard) = SCAN_PLUGINS.write() {
        guard.push(p);
    }
}

/// Wrapper around a dynamically loaded library + exported scan function.
struct DynJsonScanPlugin {
    _lib: libloading::Library,
    func: ScanJsonFn,
    name: String,
}

impl DynJsonScanPlugin {
    fn new(lib: libloading::Library, func: ScanJsonFn, path: &Path) -> Self {
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("dyn").to_string();
        Self { _lib: lib, func, name }
    }
}

impl ScanPlugin for DynJsonScanPlugin {
    fn name(&self) -> &'static str {
        // leak to 'static; acceptable for process lifetime
        Box::leak(self.name.clone().into_boxed_str())
    }
    fn matches(&self, _root: &Path) -> bool { true }
    fn scan(&self, root: &Path) -> Result<Vec<TransUnit>> {
        let s = root.to_string_lossy();
        let c_root = CString::new(s.as_bytes())?;
        let ptr = unsafe { (self.func)(c_root.as_ptr()) };
        if ptr.is_null() { return Ok(Vec::new()); }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let json = c_str.to_string_lossy().into_owned();
        // It is up to the plugin to decide who frees the string. We ignore for now.
        let units: Vec<TransUnit> = serde_json::from_str(&json).unwrap_or_default();
        Ok(units)
    }
}

/// Load dynamic plugins from a directory. Returns count of loaded plugins.
pub fn load_dynamic_plugins_from(dir: &Path) -> usize {
    let mut loaded = 0usize;
    if !dir.is_dir() { return 0; }
    let mut libs: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()).map(|ext| match std::env::consts::OS {
                "macos" => ext.eq_ignore_ascii_case("dylib"),
                "windows" => ext.eq_ignore_ascii_case("dll"),
                _ => ext.eq_ignore_ascii_case("so"),
            }).unwrap_or(false) {
                libs.push(p);
            }
        }
    }
    for p in libs {
        unsafe {
            if let Ok(lib) = libloading::Library::new(&p) {
                let func: Option<ScanJsonFn> = match lib.get::<ScanJsonFn>(SCAN_JSON_SYMBOL) {
                    Ok(sym) => Some(*sym),
                    Err(_) => None,
                };
                if let Some(func) = func {
                    let plugin = DynJsonScanPlugin::new(lib, func, &p);
                    let boxed: Box<dyn ScanPlugin> = Box::new(plugin);
                    let static_ref: &'static dyn ScanPlugin = Box::leak(boxed);
                    register_scan_plugin(static_ref);
                    loaded += 1;
                }
            }
        }
    }
    loaded
}

/// Try environment variables to load plugin directories: RIMLOC_PLUGINS (path list).
pub fn load_plugins_from_env() -> usize {
    let mut total = 0;
    if let Ok(val) = std::env::var("RIMLOC_PLUGINS") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for part in val.split(sep).map(str::trim).filter(|s| !s.is_empty()) {
            total += load_dynamic_plugins_from(Path::new(part));
        }
    }
    total
}

/// Run all registered plugins and merge their results.
pub fn run_scan_plugins(root: &Path) -> Result<Vec<TransUnit>> {
    let mut out = Vec::new();
    if let Ok(guard) = SCAN_PLUGINS.read() {
        for p in guard.iter() {
            if p.matches(root) {
                let mut units = p.scan(root)?;
                out.append(&mut units);
            }
        }
    }
    Ok(out)
}
