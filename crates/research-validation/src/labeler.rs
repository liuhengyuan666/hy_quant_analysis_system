use chrono::NaiveDate;
use core_domain::DailyBar;
use serde::{Deserialize, Serialize};

/// Automatic regime labeler based on forward-looking returns.
#[derive(Debug, Clone)]
pub struct RegimeLabeler {
    /// Number of forward days to compute return (e.g., 20)
    pub lookforward_days: usize,
    /// Return threshold above which regime is labeled risk_on (e.g., 0.08 for 8%)
    pub risk_on_threshold: f64,
    /// Return threshold below which regime is labeled risk_off (e.g., -0.08 for -8%)
    pub risk_off_threshold: f64,
}

impl Default for RegimeLabeler {
    fn default() -> Self {
        Self {
            lookforward_days: 20,
            risk_on_threshold: 0.08,
            risk_off_threshold: -0.08,
        }
    }
}

/// A single ground-truth regime label for a symbol on a date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledRegime {
    pub date: NaiveDate,
    pub symbol: String,
    pub regime: GroundTruthRegime,
    pub future_return: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundTruthRegime {
    RiskOn,
    Neutral,
    RiskOff,
}

impl RegimeLabeler {
    /// Label regime states from a sorted sequence of daily bars.
    ///
    /// Returns one label per bar where enough forward data exists.
    pub fn label(&self, bars: &[DailyBar]) -> Vec<LabeledRegime> {
        if bars.len() <= self.lookforward_days {
            return Vec::new();
        }

        let mut results = Vec::new();
        for (idx, bar) in bars.iter().enumerate() {
            let future_idx = idx + self.lookforward_days;
            if future_idx >= bars.len() {
                break;
            }
            let future_bar = &bars[future_idx];
            let future_return = if bar.close != 0.0 {
                (future_bar.close - bar.close) / bar.close
            } else {
                0.0
            };

            let regime = if future_return > self.risk_on_threshold {
                GroundTruthRegime::RiskOn
            } else if future_return < self.risk_off_threshold {
                GroundTruthRegime::RiskOff
            } else {
                GroundTruthRegime::Neutral
            };

            results.push(LabeledRegime {
                date: bar.date,
                symbol: bar.symbol.clone(),
                regime,
                future_return,
            });
        }

        results
    }

    /// Label regimes across multiple symbols, each with its own bar sequence.
    pub fn label_multi(&self, symbol_bars: &[(String, Vec<DailyBar>)]) -> Vec<LabeledRegime> {
        let mut all = Vec::new();
        for (symbol, bars) in symbol_bars {
            let mut labeled = self.label(bars);
            // Ensure symbol consistency (label already copies from bar, but safety)
            for item in &mut labeled {
                item.symbol = symbol.clone();
            }
            all.append(&mut labeled);
        }
        all.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.symbol.cmp(&b.symbol)));
        all
    }
}

// ------------------------------------------------------------------
// Validation helpers
// ------------------------------------------------------------------

/// Compare a skill-predicted regime against ground truth.
pub fn compare_prediction(
    predicted: &str,
    ground_truth: GroundTruthRegime,
) -> PredictionOutcome {
    let pred = parse_regime(predicted);
    match (pred, ground_truth) {
        (Some(p), gt) if p == gt => PredictionOutcome::TruePositive(gt),
        (Some(p), gt) => PredictionOutcome::FalsePositive { predicted: p, actual: gt },
        (None, gt) => PredictionOutcome::ParseFailure(gt),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionOutcome {
    TruePositive(GroundTruthRegime),
    FalsePositive { predicted: GroundTruthRegime, actual: GroundTruthRegime },
    ParseFailure(GroundTruthRegime),
}

fn parse_regime(s: &str) -> Option<GroundTruthRegime> {
    match s {
        "risk_on" => Some(GroundTruthRegime::RiskOn),
        "neutral" => Some(GroundTruthRegime::Neutral),
        "risk_off" => Some(GroundTruthRegime::RiskOff),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar(date: &str, close: f64) -> DailyBar {
        DailyBar {
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            symbol: "TEST".to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
            turnover: None,
        }
    }

    #[test]
    fn test_label_risk_on() {
        let labeler = RegimeLabeler::default();
        // 20-day lookforward: 100 -> 110 = +10%
        let bars: Vec<DailyBar> = (0..=20)
            .map(|i| make_bar(&format!("2024-01-{:02}", i + 1), if i == 20 { 110.0 } else { 100.0 }))
            .collect();
        let labels = labeler.label(&bars);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].regime, GroundTruthRegime::RiskOn);
        assert!((labels[0].future_return - 0.10).abs() < 1e-6);
    }

    #[test]
    fn test_label_risk_off() {
        let labeler = RegimeLabeler::default();
        // 20-day lookforward: 100 -> 90 = -10%
        let bars: Vec<DailyBar> = (0..=20)
            .map(|i| make_bar(&format!("2024-01-{:02}", i + 1), if i == 20 { 90.0 } else { 100.0 }))
            .collect();
        let labels = labeler.label(&bars);
        assert_eq!(labels[0].regime, GroundTruthRegime::RiskOff);
    }

    #[test]
    fn test_label_neutral() {
        let labeler = RegimeLabeler::default();
        // 20-day lookforward: 100 -> 103 = +3%
        let bars: Vec<DailyBar> = (0..=20)
            .map(|i| make_bar(&format!("2024-01-{:02}", i + 1), if i == 20 { 103.0 } else { 100.0 }))
            .collect();
        let labels = labeler.label(&bars);
        assert_eq!(labels[0].regime, GroundTruthRegime::Neutral);
    }
}
