use chrono::NaiveDate;
use core_domain::{
    ChangeDetectionMetrics, DailyBar, DrawdownAlignment, MarketRegimeSnapshot,
    RegimeInformationScore, StateAlignmentScore, TrendAlignment,
};
use std::collections::HashMap;

// ============================================================
// State Alignment Audit (TASK-025A)
// Production Pipeline Audit for macro-engine regimes.
//
// Principles:
// 1. Multi-threshold drawdown (10/20/30%) to avoid HK false positive.
// 2. Strict daily alignment (no tolerance) + separate change detection.
// 3. Information score (entropy) to catch "69% RiskOff" collapse.
// 4. Pass gate: overall_alignment > 0.75 AND overall_information > 0.60.
// ============================================================

/// Compute rolling drawdown from all-time high within the window.
pub fn compute_rolling_drawdown(bars: &[DailyBar]) -> Vec<(NaiveDate, f64)> {
    let mut result = Vec::new();
    let mut recent_high = 0.0;

    for bar in bars {
        if bar.high > recent_high {
            recent_high = bar.high;
        }
        if recent_high > 0.0 {
            let dd = ((bar.close - recent_high) / recent_high * 100.0).clamp(-100.0, 0.0);
            result.push((bar.date, dd));
        } else {
            result.push((bar.date, 0.0));
        }
    }
    result
}

/// Compute simple moving average.
fn sma(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut result = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        if i + 1 < period {
            result.push(None);
        } else {
            let window = &values[i + 1 - period..=i];
            let avg = window.iter().sum::<f64>() / period as f64;
            result.push(Some(avg));
        }
    }
    result
}

/// Market state classification used as ground truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketState {
    DeepDrawdown10, // dd < -10%
    DeepDrawdown20, // dd < -20%
    DeepDrawdown30, // dd < -30%
    Uptrend,        // close > MA20 > MA60
    Sideways,       // everything else
}

/// Compute market states for each bar date.
/// Returns a map with multiple drawdown thresholds + trend state.
pub fn compute_market_states_multi(bars: &[DailyBar]) -> HashMap<NaiveDate, (f64, bool)> {
    let mut result = HashMap::new();
    if bars.is_empty() {
        return result;
    }

    let drawdowns = compute_rolling_drawdown(bars);
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let ma20 = sma(&closes, 20);
    let ma60 = sma(&closes, 60);

    for (i, bar) in bars.iter().enumerate() {
        let dd = drawdowns.get(i).map(|(_, v)| *v).unwrap_or(0.0);
        let is_uptrend = if let (Some(m20), Some(m60)) = (ma20[i], ma60[i]) {
            bar.close > m20 && m20 > m60
        } else {
            false
        };
        result.insert(bar.date, (dd, is_uptrend));
    }

    result
}

/// Parse production regime label string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsedRegime {
    RiskOn,
    Neutral,
    RiskOff,
}

fn parse_regime_label(label: &str) -> Option<ParsedRegime> {
    match label.to_lowercase().as_str() {
        "risk_on" | "riskon" => Some(ParsedRegime::RiskOn),
        "risk_off" | "riskoff" => Some(ParsedRegime::RiskOff),
        "neutral" => Some(ParsedRegime::Neutral),
        _ => None,
    }
}

/// Binary classification metrics.
fn binary_metrics(tp: usize, fp: usize, fn_: usize) -> (f64, f64, f64) {
    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        0.0
    };
    let recall = if tp + fn_ > 0 {
        tp as f64 / (tp + fn_) as f64
    } else {
        0.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    (precision, recall, f1)
}

/// Compute strict daily alignment for RiskOff vs a specific drawdown threshold.
fn compute_drawdown_alignment(
    regime_dates: &[(NaiveDate, ParsedRegime)],
    market_states: &HashMap<NaiveDate, (f64, bool)>,
    threshold: f64,
) -> (f64, f64, f64) {
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_ = 0usize;

    for (date, regime) in regime_dates {
        let (dd, _) = market_states.get(date).copied().unwrap_or((0.0, false));
        let is_deep_dd = dd < threshold;

        match regime {
            ParsedRegime::RiskOff => {
                if is_deep_dd {
                    tp += 1;
                } else {
                    fp += 1;
                }
            }
            _ => {
                if is_deep_dd {
                    fn_ += 1;
                }
            }
        }
    }

    binary_metrics(tp, fp, fn_)
}

/// Compute strict daily alignment for RiskOn vs Uptrend.
fn compute_trend_alignment(
    regime_dates: &[(NaiveDate, ParsedRegime)],
    market_states: &HashMap<NaiveDate, (f64, bool)>,
) -> (f64, f64, f64) {
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_ = 0usize;

    for (date, regime) in regime_dates {
        let (_, is_uptrend) = market_states.get(date).copied().unwrap_or((0.0, false));

        match regime {
            ParsedRegime::RiskOn => {
                if is_uptrend {
                    tp += 1;
                } else {
                    fp += 1;
                }
            }
            _ => {
                if is_uptrend {
                    fn_ += 1;
                }
            }
        }
    }

    binary_metrics(tp, fp, fn_)
}

/// Extract change points (regime transitions) from a sequence.
fn extract_change_points(dates: &[(NaiveDate, ParsedRegime)]) -> Vec<(NaiveDate, ParsedRegime, ParsedRegime)> {
    let mut changes = Vec::new();
    for window in dates.windows(2) {
        let (d1, r1) = window[0];
        let (d2, r2) = window[1];
        if r1 != r2 {
            changes.push((d2, r1, r2));
        }
    }
    changes
}

/// Extract state change points from market state sequence.
fn extract_state_changes(
    dates: &[NaiveDate],
    market_states: &HashMap<NaiveDate, (f64, bool)>,
    threshold: f64,
) -> Vec<(NaiveDate, String, String)> {
    let mut changes = Vec::new();
    for window in dates.windows(2) {
        let d1 = window[0];
        let d2 = window[1];
        let (dd1, up1) = market_states.get(&d1).copied().unwrap_or((0.0, false));
        let (dd2, up2) = market_states.get(&d2).copied().unwrap_or((0.0, false));

        let state1 = if dd1 < threshold {
            "dd"
        } else if up1 {
            "up"
        } else {
            "side"
        };
        let state2 = if dd2 < threshold {
            "dd"
        } else if up2 {
            "up"
        } else {
            "side"
        };

        if state1 != state2 {
            changes.push((d2, state1.to_string(), state2.to_string()));
        }
    }
    changes
}

/// Compute change detection metrics with tolerance.
fn compute_change_detection(
    regime_changes: &[(NaiveDate, ParsedRegime, ParsedRegime)],
    state_changes: &[(NaiveDate, String, String)],
    tolerance_days: usize,
) -> ChangeDetectionMetrics {
    let mut matched_regime = vec![false; regime_changes.len()];
    let mut matched_state = vec![false; state_changes.len()];
    let mut latencies = Vec::new();

    for (i, (r_date, r_from, r_to)) in regime_changes.iter().enumerate() {
        for (j, (s_date, s_from, s_to)) in state_changes.iter().enumerate() {
            if matched_state[j] {
                continue;
            }
            let diff = (*s_date - *r_date).num_days().abs() as usize;
            if diff <= tolerance_days {
                // Check if the transition direction is similar
                let r_transition = match (r_from, r_to) {
                    (ParsedRegime::RiskOn, ParsedRegime::RiskOff) | (ParsedRegime::RiskOn, ParsedRegime::Neutral) => "risk_on_exit",
                    (ParsedRegime::RiskOff, ParsedRegime::RiskOn) | (ParsedRegime::RiskOff, ParsedRegime::Neutral) => "risk_off_exit",
                    (ParsedRegime::Neutral, ParsedRegime::RiskOn) => "to_risk_on",
                    (ParsedRegime::Neutral, ParsedRegime::RiskOff) => "to_risk_off",
                    _ => "other",
                };
                let s_transition = match (s_from.as_str(), s_to.as_str()) {
                    ("up", "dd") | ("up", "side") => "risk_on_exit",
                    ("dd", "up") | ("dd", "side") => "risk_off_exit",
                    ("side", "up") => "to_risk_on",
                    ("side", "dd") => "to_risk_off",
                    _ => "other",
                };

                if r_transition == s_transition || r_transition == "other" || s_transition == "other" {
                    matched_regime[i] = true;
                    matched_state[j] = true;
                    latencies.push(diff as f64);
                    break;
                }
            }
        }
    }

    let tp = matched_regime.iter().filter(|&&x| x).count();
    let fp = matched_regime.iter().filter(|&&x| !x).count();
    let fn_ = matched_state.iter().filter(|&&x| !x).count();

    let (precision, recall, _) = binary_metrics(tp, fp, fn_);
    let avg_latency = if !latencies.is_empty() {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    } else {
        0.0
    };

    ChangeDetectionMetrics {
        precision,
        recall,
        avg_latency_days: avg_latency,
    }
}

/// Compute information score (Shannon entropy) of regime distribution.
fn compute_information_score(regime_dates: &[(NaiveDate, ParsedRegime)]) -> RegimeInformationScore {
    let total = regime_dates.len() as f64;
    if total == 0.0 {
        return RegimeInformationScore {
            entropy: 0.0,
            normalized_entropy: 0.0,
            effective_states: 0.0,
        };
    }

    let risk_on_count = regime_dates.iter().filter(|(_, r)| *r == ParsedRegime::RiskOn).count() as f64;
    let risk_off_count = regime_dates.iter().filter(|(_, r)| *r == ParsedRegime::RiskOff).count() as f64;
    let neutral_count = regime_dates.iter().filter(|(_, r)| *r == ParsedRegime::Neutral).count() as f64;

    let p_on = risk_on_count / total;
    let p_off = risk_off_count / total;
    let p_neu = neutral_count / total;

    let entropy = -[p_on, p_off, p_neu]
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| p * p.log2())
        .sum::<f64>();

    // Max entropy for 3 states is log2(3) ≈ 1.585
    let max_entropy = (3.0f64).log2();
    let normalized_entropy = (entropy / max_entropy).clamp(0.0, 1.0);

    // Effective states = 2^entropy
    let effective_states = 2.0f64.powf(entropy);

    RegimeInformationScore {
        entropy,
        normalized_entropy,
        effective_states,
    }
}

/// Full state alignment computation for production pipeline regimes.
pub fn compute_state_alignment(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    tolerance_days: usize,
) -> Option<StateAlignmentScore> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let market_states = compute_market_states_multi(bars);

    // Filter regimes to dates with bar data
    let regime_dates: Vec<(NaiveDate, ParsedRegime)> = regimes
        .iter()
        .filter_map(|r| {
            let parsed = parse_regime_label(&r.regime_label)?;
            if market_states.contains_key(&r.date) {
                Some((r.date, parsed))
            } else {
                None
            }
        })
        .collect();

    if regime_dates.is_empty() {
        return None;
    }

    let total_days = regime_dates.len();
    let window_from = regime_dates.first().map(|(d, _)| *d).unwrap_or(bars[0].date);
    let window_to = regime_dates.last().map(|(d, _)| *d).unwrap_or(bars[bars.len() - 1].date);
    let scope = regimes.first().map(|r| r.market.clone()).unwrap_or_default();

    // 1. Drawdown alignment (multi-threshold)
    let (dd10_p, dd10_r, dd10_f1) = compute_drawdown_alignment(&regime_dates, &market_states, -10.0);
    let (dd20_p, dd20_r, dd20_f1) = compute_drawdown_alignment(&regime_dates, &market_states, -20.0);
    let (dd30_p, dd30_r, dd30_f1) = compute_drawdown_alignment(&regime_dates, &market_states, -30.0);

    let drawdown_alignment = DrawdownAlignment {
        dd10_precision: dd10_p,
        dd10_recall: dd10_r,
        dd10_f1: dd10_f1,
        dd20_precision: dd20_p,
        dd20_recall: dd20_r,
        dd20_f1: dd20_f1,
        dd30_precision: dd30_p,
        dd30_recall: dd30_r,
        dd30_f1: dd30_f1,
    };

    // 2. Trend alignment
    let (riskon_p, riskon_r, riskon_f1) = compute_trend_alignment(&regime_dates, &market_states);
    let trend_alignment = TrendAlignment {
        riskon_precision: riskon_p,
        riskon_recall: riskon_r,
        riskon_f1: riskon_f1,
    };

    // 3. Change detection
    let regime_changes = extract_change_points(&regime_dates);
    let all_dates: Vec<NaiveDate> = regime_dates.iter().map(|(d, _)| *d).collect();
    let state_changes = extract_state_changes(&all_dates, &market_states, -10.0);
    let change_detection = compute_change_detection(&regime_changes, &state_changes, tolerance_days);

    // 4. Information score
    let information_score = compute_information_score(&regime_dates);

    // 5. Overall scores
    // Use average of DD20 F1 and RiskOn F1 as primary alignment (DD10 is too easy for HK)
    let alignment_components = [dd20_f1, riskon_f1];
    let overall_alignment = alignment_components.iter().sum::<f64>() / alignment_components.len() as f64;
    let overall_information = information_score.normalized_entropy;

    let overall_passed = overall_alignment > 0.75 && overall_information > 0.60;

    Some(StateAlignmentScore {
        scope,
        window_from,
        window_to,
        total_days,
        drawdown_alignment,
        trend_alignment,
        change_detection,
        information_score,
        overall_alignment,
        overall_information,
        overall_passed,
    })
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bar(date: NaiveDate, close: f64, high: f64) -> DailyBar {
        DailyBar {
            date,
            symbol: "TEST".to_string(),
            open: close,
            high,
            low: close,
            close,
            volume: 1000.0,
            turnover: Some(10000.0),
        }
    }

    fn make_regime(date: NaiveDate, label: &str) -> MarketRegimeSnapshot {
        MarketRegimeSnapshot {
            date,
            macro_as_of_date: date,
            market: "TEST".to_string(),
            trend_score: 50.0,
            liquidity_score: 50.0,
            risk_score: 50.0,
            regime_label: label.to_string(),
        }
    }

    #[test]
    fn test_drawdown_computation() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let bars = vec![
            make_bar(start, 100.0, 100.0),
            make_bar(start + chrono::Duration::days(1), 90.0, 100.0),
            make_bar(start + chrono::Duration::days(2), 80.0, 100.0),
            make_bar(start + chrono::Duration::days(3), 85.0, 100.0),
        ];

        let dd = compute_rolling_drawdown(&bars);
        assert_eq!(dd[0].1, 0.0);
        assert!((dd[1].1 - -10.0).abs() < 0.01);
        assert!((dd[2].1 - -20.0).abs() < 0.01);
        assert!((dd[3].1 - -15.0).abs() < 0.01);
    }

    #[test]
    fn test_riskoff_dd10_alignment_perfect() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut bars = Vec::new();
        let mut regimes = Vec::new();

        for i in 0..10 {
            let close = if i < 5 { 80.0 } else { 95.0 };
            let high = 100.0;
            bars.push(make_bar(start + chrono::Duration::days(i), close, high));
            let label = if i < 5 { "risk_off" } else { "neutral" };
            regimes.push(make_regime(start + chrono::Duration::days(i), label));
        }

        let score = compute_state_alignment(&regimes, &bars, 0).unwrap();
        assert!(score.drawdown_alignment.dd10_precision > 0.8, "dd10 precision should be high: {}", score.drawdown_alignment.dd10_precision);
        assert!(score.drawdown_alignment.dd10_recall > 0.8, "dd10 recall should be high: {}", score.drawdown_alignment.dd10_recall);
    }

    #[test]
    fn test_information_score_balanced() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut regimes = Vec::new();
        for i in 0..30 {
            let label = match i % 3 {
                0 => "risk_on",
                1 => "neutral",
                _ => "risk_off",
            };
            regimes.push(make_regime(start + chrono::Duration::days(i), label));
        }
        let bars: Vec<DailyBar> = (0..30)
            .map(|i| make_bar(start + chrono::Duration::days(i), 100.0, 100.0))
            .collect();

        let score = compute_state_alignment(&regimes, &bars, 0).unwrap();
        assert!(score.information_score.normalized_entropy > 0.9, "balanced regimes should have high entropy: {}", score.information_score.normalized_entropy);
        assert!(score.information_score.effective_states > 2.5, "effective states should be ~3: {}", score.information_score.effective_states);
    }

    #[test]
    fn test_information_score_imbalanced() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut regimes = Vec::new();
        for i in 0..100 {
            let label = if i < 90 { "risk_off" } else { "neutral" };
            regimes.push(make_regime(start + chrono::Duration::days(i), label));
        }
        let bars: Vec<DailyBar> = (0..100)
            .map(|i| make_bar(start + chrono::Duration::days(i), 100.0, 100.0))
            .collect();

        let score = compute_state_alignment(&regimes, &bars, 0).unwrap();
        assert!(score.information_score.normalized_entropy < 0.5, "imbalanced regimes should have low entropy: {}", score.information_score.normalized_entropy);
        assert!(score.information_score.effective_states < 2.0, "effective states should be low: {}", score.information_score.effective_states);
        assert!(!score.overall_passed, "imbalanced regimes should fail overall gate");
    }

    #[test]
    fn test_riskon_alignment_high_when_uptrend() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut bars = Vec::new();
        let mut regimes = Vec::new();

        // 200 days of strong uptrend (close > MA20 > MA60 after warm-up)
        for i in 0..200 {
            let close = 100.0 + i as f64 * 2.0;
            bars.push(make_bar(start + chrono::Duration::days(i), close, close));
            regimes.push(make_regime(start + chrono::Duration::days(i), "risk_on"));
        }

        let score = compute_state_alignment(&regimes, &bars, 0).unwrap();
        // After 60-day MA warm-up, most days should be detected as uptrend
        assert!(score.trend_alignment.riskon_precision > 0.7, "riskon precision should be high: {}", score.trend_alignment.riskon_precision);
        assert!(score.trend_alignment.riskon_recall > 0.7, "riskon recall should be high: {}", score.trend_alignment.riskon_recall);
    }
}
