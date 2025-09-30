use super::parser::Candidate;
use crate::Result;
use roxmltree;

pub trait Classifier {
    fn score(&mut self, cand: &Candidate) -> Result<f32>;
}

pub struct DummyClassifier {
    boost: f32,
}
impl DummyClassifier {
    pub fn new(boost: f32) -> Self {
        Self { boost }
    }
}
impl Classifier for DummyClassifier {
    fn score(&mut self, cand: &Candidate) -> Result<f32> {
        // Simple heuristic: longer strings get slightly higher score
        let base = if cand.value.len() > 8 { 0.8 } else { 0.6 };
        Ok((base + self.boost).min(1.0))
    }
}

pub struct RestClassifier {
    url: String,
    client: reqwest::blocking::Client,
}
impl RestClassifier {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::blocking::Client::new(),
        }
    }
}
impl Classifier for RestClassifier {
    fn score(&mut self, cand: &Candidate) -> Result<f32> {
        #[derive(serde::Serialize)]
        struct In<'a> {
            def_type: &'a str,
            def_name: &'a str,
            field_path: &'a str,
            value: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct Out {
            score: f32,
        }
        let resp: Out = self
            .client
            .post(&self.url)
            .json(&In {
                def_type: &cand.def_type,
                def_name: &cand.def_name,
                field_path: &cand.field_path,
                value: &cand.value,
            })
            .send()?
            .error_for_status()?
            .json()?;
        Ok(resp.score)
    }
}

/// Reuse function from parser logic to navigate dot-paths.
pub(crate) fn collect_values_by_path<'a>(
    node: roxmltree::Node<'a, 'a>,
    path: &[&str],
    out: &mut Vec<&'a str>,
) {
    if path.is_empty() {
        if let Some(t) = node.text() {
            let t = t.trim();
            if !t.is_empty() {
                out.push(t);
            }
        }
        return;
    }
    let mut head = path[0];
    if let Some(pos) = head.find('{') { head = &head[..pos]; }
    let tail = &path[1..];
    if head.eq_ignore_ascii_case("li") {
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("li"))
        {
            collect_values_by_path(child, tail, out);
        }
    } else {
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(head))
        {
            collect_values_by_path(child, tail, out);
        }
    }
}
