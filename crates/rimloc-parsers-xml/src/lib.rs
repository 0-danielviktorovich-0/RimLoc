use rimloc_core::{Result, TransUnit, RimLocError};
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use quick_xml::Reader;
use quick_xml::events::Event;

pub fn scan_keyed_xml(root: &Path) -> Result<Vec<TransUnit>> {
    let mut out: Vec<TransUnit> = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            if let Ok(txt) = std::fs::read_to_string(path) {
                match extract_with_lines(&txt, path) {
                    Ok(mut units) => out.append(&mut units),
                    Err(e) => eprintln!("[rimloc] WARN: {path:?}: {e}"),
                }
            }
        }
    }

    Ok(out)
}

fn line_starts_of(text: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(256);
    starts.push(0);
    for (i, b) in text.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn byte_pos_to_line(pos: usize, starts: &[usize]) -> u32 {
    let idx = starts.partition_point(|&s| s <= pos);
    (idx as u32).max(1)
}

#[derive(Clone, Debug)]
struct Frame {
    name: String,
    had_text: bool,
    had_child: bool,
}

fn extract_with_lines(xml: &str, path: &Path) -> Result<Vec<TransUnit>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let line_starts = line_starts_of(xml);

    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                // помечаем, что у родителя появился ребёнок
                if let Some(parent) = stack.last_mut() {
                    parent.had_child = true;
                }
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                stack.push(Frame { name, had_text: false, had_child: false });
            }

            Ok(Event::Text(e)) => {
                if !stack.is_empty() {
                    let mut key = stack.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(".");

                    if let Some(stripped) = key.strip_prefix("LanguageData.") {
                        key = stripped.to_string();
                    }

                    let value = e.unescape().unwrap_or_default().to_string();
                    let value = value.trim();

                    // помечаем, что текущий кадр имел текст
                    if let Some(last) = stack.last_mut() {
                        last.had_text = true;
                    }

                    let byte_pos = reader.buffer_position() as usize;
                    let line_no = byte_pos_to_line(byte_pos, &line_starts);

                    if !value.is_empty() {
                        out.push(TransUnit {
                            key,
                            source: Some(value.to_string()),
                            path: PathBuf::from(path),
                            line: Some(line_no),
                        });
                    } else {
                        // Пустой текстовый узел – тоже фиксируем как пустое значение
                        out.push(TransUnit {
                            key,
                            source: Some(String::new()),
                            path: PathBuf::from(path),
                            line: Some(line_no),
                        });
                    }
                }
            }

            Ok(Event::End(_e)) => {
                // На закрытии тега: если у узла нет текста и нет детей — это пустой тег
                if let Some(frame) = stack.pop() {
                    if !frame.had_text && !frame.had_child {
                        // ключ = путь всех кадров (включая текущий)
                        let mut key = stack
                            .iter()
                            .map(|f| f.name.as_str())
                            .chain(std::iter::once(frame.name.as_str()))
                            .collect::<Vec<_>>()
                            .join(".");

                        if let Some(stripped) = key.strip_prefix("LanguageData.") {
                            key = stripped.to_string();
                        }

                        let byte_pos = reader.buffer_position() as usize;
                        let line_no = byte_pos_to_line(byte_pos, &line_starts);

                        out.push(TransUnit {
                            key,
                            source: Some(String::new()),
                            path: PathBuf::from(path),
                            line: Some(line_no),
                        });
                    }
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(RimLocError::Xml(format!("{e}"))),
            _ => {}
        }

        buf.clear();
    }

    Ok(out)
}
