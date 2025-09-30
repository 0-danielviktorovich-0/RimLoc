use color_eyre::eyre::Result;
use rimloc_core::TransUnit;

/// Trait implemented by statically linked scan plugins.
pub trait ScanPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, _root: &std::path::Path) -> bool {
        true
    }
    fn scan(&self, root: &std::path::Path) -> Result<Vec<TransUnit>>;
}

/// FFI symbol expected from dynamically loaded plugins.
/// The function receives a C string path and must return a newly allocated C string
/// containing JSON array of TransUnit objects.
pub const SCAN_JSON_SYMBOL: &[u8] = b"rimloc_plugin_scan_json\0";

pub type ScanJsonFn = unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::os::raw::c_char;

