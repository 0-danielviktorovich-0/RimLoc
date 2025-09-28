use crate::Result;
use std::path::{Path, PathBuf};

pub fn load_dicts(
    base: &Path,
    files: &[PathBuf],
) -> Result<Vec<std::collections::HashMap<String, Vec<String>>>> {
    let mut out = Vec::new();
    // embedded minimal dict
    out.push(rimloc_parsers_xml::load_embedded_defs_dict().0);
    for f in files {
        let p = if f.is_absolute() {
            f.clone()
        } else {
            base.join(f)
        };
        if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&p) {
            out.push(d.0);
        }
    }
    Ok(out)
}

pub fn merge_dicts(
    dicts: Vec<std::collections::HashMap<String, Vec<String>>>,
) -> std::collections::HashMap<String, Vec<String>> {
    use std::collections::{BTreeSet, HashMap};
    let mut out: HashMap<String, BTreeSet<String>> = HashMap::new();
    for d in dicts {
        for (k, v) in d {
            let e = out.entry(k).or_default();
            for s in v {
                e.insert(s);
            }
        }
    }
    let mut flat = std::collections::HashMap::new();
    for (k, v) in out {
        flat.insert(k, v.into_iter().collect());
    }
    flat
}
