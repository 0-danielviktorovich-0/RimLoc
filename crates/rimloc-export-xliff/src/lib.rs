use rimloc_core::{Result, TransUnit};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn write_xliff_12(path: &Path, units: &[TransUnit], src_lang: &str, trg_lang: &str) -> Result<()> {
    let mut w = BufWriter::new(File::create(path)?);
    writeln!(w, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(w, r#"<xliff version="1.2">"#)?;
    writeln!(w, r#"  <file source-language="{}" target-language="{}" datatype="plaintext" original="rimloc">"#, src_lang, trg_lang)?;
    writeln!(w, "    <body>")?;
    for u in units {
        let id = &u.key;
        let src = u.source.as_deref().unwrap_or("");
        writeln!(w, "      <trans-unit id=\"{}\">", xml_escape(id))?;
        writeln!(w, "        <source>{}</source>", xml_escape(src))?;
        writeln!(w, "        <target/>")?;
        if let Some(line) = u.line {
            writeln!(w, "        <note>{}:{}</note>", u.path.display(), line)?;
        } else {
            writeln!(w, "        <note>{}</note>", u.path.display())?;
        }
        writeln!(w, "      </trans-unit>")?;
    }
    writeln!(w, "    </body>")?;
    writeln!(w, "  </file>")?;
    writeln!(w, "</xliff>")?;
    w.flush()?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

