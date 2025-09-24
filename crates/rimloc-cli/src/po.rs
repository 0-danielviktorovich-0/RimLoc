use std::io::{BufRead, BufReader};

#[allow(dead_code)]
pub type PoEntry = (Option<String>, String, String, Option<String>);

#[allow(dead_code)]
pub fn parse_po_basic(path: &std::path::Path) -> color_eyre::eyre::Result<Vec<PoEntry>> {
    use std::fs::File;

    let f = File::open(path)?;
    let rdr = BufReader::new(f);

    let mut entries = Vec::new();
    let mut ctx: Option<String> = None;
    let mut id = String::new();
    let mut strv = String::new();
    let mut refv: Option<String> = None;

    enum Mode {
        None,
        InId,
        InStr,
    }
    let mut mode = Mode::None;

    fn unquote_po(s: &str) -> String {
        let mut out = String::new();
        let raw = s.trim().trim_start_matches('"').trim_end_matches('"');
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(n) = chars.next() {
                    match n {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        _ => {
                            out.push('\\');
                            out.push(n);
                        }
                    }
                } else {
                    out.push('\\');
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    let mut push_if_complete = |ctx: &mut Option<String>,
                                id: &mut String,
                                strv: &mut String,
                                refv: &mut Option<String>| {
        if !id.is_empty() || !strv.is_empty() {
            entries.push((
                ctx.clone(),
                std::mem::take(id),
                std::mem::take(strv),
                refv.clone(),
            ));
            *ctx = None;
            *refv = None;
        }
    };

    for line in rdr.lines() {
        let line = line?;
        let t = line.trim();

        if t.is_empty() {
            push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
            mode = Mode::None;
            continue;
        }

        if let Some(rest) = t.strip_prefix("msgctxt ") {
            push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
            ctx = Some(unquote_po(rest));
            mode = Mode::None;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgid ") {
            push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
            id = unquote_po(rest);
            mode = Mode::InId;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgstr ") {
            strv = unquote_po(rest);
            mode = Mode::InStr;
            continue;
        }
        if let Some(rest) = t.strip_prefix("#: ") {
            refv = Some(rest.to_string());
            continue;
        }

        match mode {
            Mode::InId | Mode::InStr => {
                if t.starts_with('"') {
                    let chunk = unquote_po(t);
                    match mode {
                        Mode::InId => id.push_str(&chunk),
                        Mode::InStr => strv.push_str(&chunk),
                        Mode::None => {}
                    }
                }
            }
            Mode::None => {}
        }
    }

    push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
    Ok(entries)
}
