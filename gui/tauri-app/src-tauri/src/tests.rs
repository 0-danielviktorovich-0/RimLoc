#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  fn ws_root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf() }

  #[test]
  fn version_nonempty() {
    let v = crate::api_app_version().unwrap();
    assert!(!v.is_empty());
  }

  #[test]
  fn schema_dump_tmp() {
    let dir = tempfile::tempdir().unwrap();
    let out = crate::api_schema_dump(dir.path().display().to_string()).unwrap();
    assert!(std::path::Path::new(&out).exists());
  }

  #[test]
  fn smoke_diff_and_reports() {
    let root = ws_root().join("test/TestMod");
    let diff = crate::api_diff_xml(root.display().to_string(), Some("en".into()), None, Some("ru".into()), None, None).unwrap();
    assert!(diff.only_in_mod.len() >= 0);
    let outd = tempfile::tempdir().unwrap();
    let saved = crate::api_diff_save_reports(ws_root().join("test/TestMod").display().to_string(), Some("en".into()), None, Some("ru".into()), None, None, outd.path().display().to_string()).unwrap();
    assert!(std::path::Path::new(&saved).exists());
  }

  #[test]
  fn smoke_annotate_and_import_dry() {
    let root = ws_root().join("test/TestMod");
    let plan = crate::api_annotate_dry(root.display().to_string(), Some("en".into()), None, Some("ru".into()), None, Some("EN:".into()), false).unwrap();
    assert!(plan.processed >= 0);
    let dir = tempfile::tempdir().unwrap();
    let po = dir.path().join("mod.po");
    rimloc_services::export_po_with_tm(root.as_path(), po.as_path(), Some("ru"), Some("en"), None, None).unwrap();
    let plan2 = crate::api_import_po_dry(po.display().to_string(), root.display().to_string(), Some("ru".into()), None, false, false, None, true).unwrap();
    assert!(plan2.total_keys >= 0);
  }
}

