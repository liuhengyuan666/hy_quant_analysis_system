use chrono::NaiveDate;
use core_domain::DailyBar;
use core_domain::IndicatorSnapshot;
use gt_regime_generator::{PersistenceConfig, RegimePipeline};
use market_state_extractor::TrendDirectionMethod;

use crate::external_validation;

// ============================================================
// TrendDirection Sensitivity Replay (TASK-018B.1)
// Replays the full GT Regime Pipeline with different
// TrendDirection classification strategies and compares
// economic validation results.
// ============================================================

/// Which TrendDirection classification strategy to use.
/// Maps directly to market_state_extractor::TrendDirectionMethod.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirectionStrategy {
    /// Current baseline: absolute slope threshold 0.001
    Baseline,
    /// Relative slope: slope / close, threshold 0.001 = 0.1%
    RelativeSlope,
    /// Percentile-based: dynamic P5/P25/P75/P95 thresholds from historical distribution
    Percentile,
    /// Z-score based: z > 1.5 StrongUp, z > 0.5 Up, z < -0.5 Down, z < -1.5 StrongDown
    ZScore,
}

impl TrendDirectionStrategy {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Baseline => "Baseline",
            Self::RelativeSlope => "RelativeSlope",
            Self::Percentile => "Percentile",
            Self::ZScore => "ZScore",
        }
    }

    pub fn to_method(&self) -> TrendDirectionMethod {
        match self {
            Self::Baseline => TrendDirectionMethod::Baseline,
            Self::RelativeSlope => TrendDirectionMethod::RelativeSlope,
            Self::Percentile => TrendDirectionMethod::Percentile,
            Self::ZScore => TrendDirectionMethod::ZScore,
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Baseline, Self::RelativeSlope, Self::Percentile, Self::ZScore]
    }
}

/// Per-variant comparison result.
#[derive(Debug, Clone)]
pub struct ReplayVariantResult {
    pub variant: String,
    pub separation_score: f64,
    pub gates_passed: usize,
    pub riskon_return_60d: f64,
    pub riskoff_return_60d: f64,
    pub churn_rate: f64,
    pub imbalance_ratio: f64,
}

/// Overall sensitivity replay report.
#[derive(Debug, Clone)]
pub struct SensitivityReplayReport {
    pub scope: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub comparison: Vec<ReplayVariantResult>,
    pub recommendation: String,
}

/// Run sensitivity replay across all TrendDirectionStrategy variants.
///
/// For each variant:
/// 1. Extract MarketStateObservation using that strategy
/// 2. Run GT Regime Pipeline (10 confirmation days, 5 min days)
/// 3. Run Regime Audit (default gates)
/// 4. Run External Validation (validate_regimes_economically)
///
/// Returns a comparison report with economic separation scores,
/// gate results, churn rates, and a recommendation.
pub fn run_sensitivity_replay(
    bars: &[DailyBar],
    indicators: &[IndicatorSnapshot],
    scope: &str,
    anchor_symbol: &str,
) -> SensitivityReplayReport {
    let window_from = bars.first().map(|b| b.date).unwrap_or(NaiveDate::MIN);
    let window_to = bars.last().map(|b| b.date).unwrap_or(NaiveDate::MIN);

    let config = PersistenceConfig {
        min_days: 5,
        confirmation_days: 10,
    };

    let mut comparison = Vec::new();

    for strategy in TrendDirectionStrategy::all() {
        let method = strategy.to_method();
        let name = strategy.name().to_string();

        // Step 1: Extract observations using this strategy
        let observations = market_state_extractor::extract_market_state_observations_with_method(
            bars,
            indicators,
            scope,
            method,
        );

        // Step 2: Run GT Regime Pipeline
        let mut pipeline = RegimePipeline::with_config(scope, config.clone());
        let labels = pipeline.process_sequence(&observations);

        if labels.is_empty() {
            comparison.push(ReplayVariantResult {
                variant: name,
                separation_score: 0.0,
                gates_passed: 0,
                riskon_return_60d: 0.0,
                riskoff_return_60d: 0.0,
                churn_rate: 0.0,
                imbalance_ratio: 0.0,
            });
            continue;
        }

        // Step 3: Run Regime Audit
        let audit_report = crate::audit_regime_labels_default(&labels);

        // Step 4: Run External Validation
        let val_report =
            external_validation::validate_regimes_economically(&labels, bars, scope, anchor_symbol);

        let separation = &val_report.separation_score;
        let gates_passed = separation.gate_results.values().filter(|&v| *v).count();

        let riskon_return_60d = val_report
            .stats
            .get("riskon")
            .map(|s| s.forward_return_60d_mean)
            .unwrap_or(0.0);
        let riskoff_return_60d = val_report
            .stats
            .get("riskoff")
            .map(|s| s.forward_return_60d_mean)
            .unwrap_or(0.0);

        comparison.push(ReplayVariantResult {
            variant: name,
            separation_score: separation.overall_score,
            gates_passed,
            riskon_return_60d,
            riskoff_return_60d,
            churn_rate: audit_report.persistence.churn_rate,
            imbalance_ratio: audit_report.coverage.imbalance_ratio,
        });
    }

    // Generate recommendation: best separation + most gates passed
    let recommendation = if let Some(best) = comparison
        .iter()
        .max_by(|a, b| {
            a.separation_score
                .partial_cmp(&b.separation_score)
                .unwrap()
                .then(a.gates_passed.cmp(&b.gates_passed))
        })
        .filter(|r| r.separation_score > 0.0)
    {
        let gate_str = if best.gates_passed >= 3 {
            "with strong gate confirmation"
        } else if best.gates_passed >= 2 {
            "with some gate confirmation"
        } else {
            "but gate performance is weak"
        };
        format!("{} variant shows best separation {} (separation={:.1}, gates={}/4)",
            best.variant, gate_str, best.separation_score, best.gates_passed)
    } else {
        "insufficient data to determine best variant".to_string()
    };

    SensitivityReplayReport {
        scope: scope.to_string(),
        window_from,
        window_to,
        comparison,
        recommendation,
    }
}

// ============================================================
// TASK-022: Persistence Filter Sensitivity Audit
// Tests different confirmation_days × min_days combinations
// to find optimal persistence parameters for economic separation.
// ============================================================

/// Per-parameter-comparison result.
#[derive(Debug, Clone)]
pub struct PersistenceVariantResult {
    pub confirmation_days: usize,
    pub min_days: usize,
    pub separation_score: f64,
    pub gates_passed: usize,
    pub riskon_return_60d: f64,
    pub riskoff_return_60d: f64,
    pub neutral_return_60d: f64,
    pub churn_rate: f64,
    pub imbalance_ratio: f64,
    pub riskon_pct: f64,
    pub neutral_pct: f64,
    pub riskoff_pct: f64,
}

/// Overall persistence sensitivity report.
#[derive(Debug, Clone)]
pub struct PersistenceSensitivityReport {
    pub scope: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub comparison: Vec<PersistenceVariantResult>,
    pub recommendation: String,
}

/// Run persistence filter sensitivity audit.
///
/// Scans confirmation_days: [5, 7, 10, 12, 15]
/// Scans min_days: [3, 5, 7, 10]
///
/// For each combination, runs the full GT pipeline and reports
/// economic separation metrics.
pub fn run_persistence_sensitivity_audit(
    bars: &[DailyBar],
    indicators: &[IndicatorSnapshot],
    scope: &str,
    anchor_symbol: &str,
) -> PersistenceSensitivityReport {
    let window_from = bars.first().map(|b| b.date).unwrap_or(NaiveDate::MIN);
    let window_to = bars.last().map(|b| b.date).unwrap_or(NaiveDate::MAX);

    let confirmation_days_options = [5, 7, 10, 12, 15];
    let min_days_options = [3, 5, 7, 10];

    let mut comparison = Vec::new();

    // Use baseline TrendDirection (current behavior)
    let observations = market_state_extractor::build_market_state_observations(
        bars,
        indicators,
        scope,
    );

    for &confirmation_days in &confirmation_days_options {
        for &min_days in &min_days_options {
            let config = PersistenceConfig {
                confirmation_days,
                min_days,
            };

            let mut pipeline = RegimePipeline::with_config(scope, config);
            let labels = pipeline.process_sequence(&observations);

            if labels.is_empty() {
                comparison.push(PersistenceVariantResult {
                    confirmation_days,
                    min_days,
                    separation_score: 0.0,
                    gates_passed: 0,
                    riskon_return_60d: 0.0,
                    riskoff_return_60d: 0.0,
                    neutral_return_60d: 0.0,
                    churn_rate: 0.0,
                    imbalance_ratio: 0.0,
                    riskon_pct: 0.0,
                    neutral_pct: 0.0,
                    riskoff_pct: 0.0,
                });
                continue;
            }

            let audit_report = crate::audit_regime_labels_default(&labels);
            let val_report =
                external_validation::validate_regimes_economically(&labels, bars, scope, anchor_symbol);

            let separation = &val_report.separation_score;
            let gates_passed = separation.gate_results.values().filter(|&v| *v).count();

            let riskon_return_60d = val_report
                .stats
                .get("riskon")
                .map(|s| s.forward_return_60d_mean)
                .unwrap_or(0.0);
            let riskoff_return_60d = val_report
                .stats
                .get("riskoff")
                .map(|s| s.forward_return_60d_mean)
                .unwrap_or(0.0);
            let neutral_return_60d = val_report
                .stats
                .get("neutral")
                .map(|s| s.forward_return_60d_mean)
                .unwrap_or(0.0);

            let total = labels.len() as f64;
            let riskon_count = labels.iter().filter(|l| l.regime == gt_regime_generator::Regime::RiskOn).count() as f64;
            let neutral_count = labels.iter().filter(|l| l.regime == gt_regime_generator::Regime::Neutral).count() as f64;
            let riskoff_count = labels.iter().filter(|l| l.regime == gt_regime_generator::Regime::RiskOff).count() as f64;

            comparison.push(PersistenceVariantResult {
                confirmation_days,
                min_days,
                separation_score: separation.overall_score,
                gates_passed,
                riskon_return_60d,
                riskoff_return_60d,
                neutral_return_60d,
                churn_rate: audit_report.persistence.churn_rate,
                imbalance_ratio: audit_report.coverage.imbalance_ratio,
                riskon_pct: riskon_count / total,
                neutral_pct: neutral_count / total,
                riskoff_pct: riskoff_count / total,
            });
        }
    }

    // Generate recommendation: best separation + reasonable balance
    let recommendation = if let Some(best) = comparison
        .iter()
        .filter(|r| r.separation_score > 0.0)
        .max_by(|a, b| {
            a.separation_score
                .partial_cmp(&b.separation_score)
                .unwrap()
                .then(a.gates_passed.cmp(&b.gates_passed))
        })
    {
        let gate_str = if best.gates_passed >= 3 {
            "with strong gate confirmation"
        } else if best.gates_passed >= 2 {
            "with some gate confirmation"
        } else {
            "but gate performance is weak"
        };
        format!(
            "confirmation_days={} + min_days={} shows best separation {} (separation={:.1}, gates={}/4, churn={:.1}%)",
            best.confirmation_days,
            best.min_days,
            gate_str,
            best.separation_score,
            best.gates_passed,
            best.churn_rate * 100.0
        )
    } else {
        "insufficient data to determine best parameters".to_string()
    };

    PersistenceSensitivityReport {
        scope: scope.to_string(),
        window_from,
        window_to,
        comparison,
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_name_mapping() {
        assert_eq!(TrendDirectionStrategy::Baseline.name(), "Baseline");
        assert_eq!(TrendDirectionStrategy::RelativeSlope.name(), "RelativeSlope");
        assert_eq!(TrendDirectionStrategy::Percentile.name(), "Percentile");
        assert_eq!(TrendDirectionStrategy::ZScore.name(), "ZScore");
    }

    #[test]
    fn test_strategy_to_method() {
        assert_eq!(
            TrendDirectionStrategy::Baseline.to_method(),
            TrendDirectionMethod::Baseline
        );
        assert_eq!(
            TrendDirectionStrategy::Percentile.to_method(),
            TrendDirectionMethod::Percentile
        );
    }

    #[test]
    fn test_all_variants() {
        let all = TrendDirectionStrategy::all();
        assert_eq!(all.len(), 4);
    }
}
