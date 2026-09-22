//! Dry-run cost estimation (rough char/4 token heuristic + per-1M pricing).
//! Advisory only — providers report authoritative usage in responses.

use crate::provider::TranslateUnit;

#[derive(Debug, Clone, Copy)]
pub struct PricePer1M {
    pub input_usd: f64,
    pub output_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostEstimate {
    pub units: usize,
    pub approx_input_tokens: u64,
    pub approx_output_tokens: u64,
    pub approx_cost_usd: f64,
}

/// Estimate for a full run before spending anything (dry-run support).
pub fn estimate(units: &[TranslateUnit], price: Option<PricePer1M>) -> CostEstimate {
    let src_chars: usize = units
        .iter()
        .map(|u| u.source.len() + u.context.as_deref().map_or(0, str::len))
        .sum();
    let input_tokens = (src_chars as u64 / 4).saturating_add(300 * units.len() as u64 / 25);
    let output_tokens = (src_chars as u64 / 4) * 13 / 10; // RU ~ +30%
    let approx_cost_usd = match price {
        Some(p) => {
            input_tokens as f64 / 1e6 * p.input_usd + output_tokens as f64 / 1e6 * p.output_usd
        }
        None => 0.0,
    };
    CostEstimate {
        units: units.len(),
        approx_input_tokens: input_tokens,
        approx_output_tokens: output_tokens,
        approx_cost_usd,
    }
}

use serde::Serialize;
