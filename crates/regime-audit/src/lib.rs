use chrono::NaiveDate;
use gt_regime_generator::{Regime, RegimeLabel};

// ============================================================
// Regime Audit (Wave 7.3D)
// Validates regime label quality via persistence and coverage metrics.
// ============================================================

// ------------------------------------------------------------------
// Episode: a contiguous run of the same regime
// ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Episode {
    pub regime: Regime,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub duration_days: usize,
}

impl Episode {
    pub fn duration(&self) -> usize {
        self.duration_days
    }
}

// ------------------------------------------------------------------
// Episode Distribution (percentiles)
// ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeDistribution {
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
}

// ------------------------------------------------------------------
// Persistence Score
// ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PersistenceScore {
    pub avg_episode_days: f64,
    pub median_episode_days: f64,
    pub distribution: EpisodeDistribution,
    pub churn_rate: f64,
    pub transition_stability: f64,
}

// ------------------------------------------------------------------
// Coverage Score
// ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageScore {
    pub risk_on_pct: f64,
    pub neutral_pct: f64,
    pub risk_off_pct: f64,
    pub imbalance_ratio: f64,
}

// ------------------------------------------------------------------
// Regime Audit Report
// ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RegimeAuditReport {
    pub persistence: PersistenceScore,
    pub coverage: CoverageScore,
    pub total_days: usize,
    pub episode_count: usize,
    pub transition_count: usize,
    pub direct_swing_count: usize,
    pub passed: bool,
    pub violations: Vec<String>,
}

// ------------------------------------------------------------------
// Audit Gates (hard thresholds)
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AuditGates {
    pub min_avg_episode_days: f64,
    pub min_median_episode_days: f64,
    pub max_churn_rate: f64,
    pub min_transition_stability: f64,
    pub max_imbalance_ratio: f64,
    pub min_risk_on_pct: f64,
    pub min_risk_off_pct: f64,
}

impl Default for AuditGates {
    fn default() -> Self {
        Self {
            min_avg_episode_days: 20.0,
            min_median_episode_days: 15.0,
            max_churn_rate: 0.05,
            min_transition_stability: 0.70,
            max_imbalance_ratio: 5.0,
            min_risk_on_pct: 0.05,
            min_risk_off_pct: 0.05,
        }
    }
}

// ============================================================
// Episode Extraction
// ============================================================

/// Extract episodes from a sequence of RegimeLabels.
/// An episode is a contiguous run of the same stable regime.
pub fn extract_episodes(labels: &[RegimeLabel]) -> Vec<Episode> {
    if labels.is_empty() {
        return Vec::new();
    }

    let mut episodes = Vec::new();
    let mut current_regime = labels[0].regime;
    let mut start_date = labels[0].date;
    let mut start_index = 0;

    for (index, label) in labels.iter().enumerate().skip(1) {
        if label.regime != current_regime {
            // Close current episode
            episodes.push(Episode {
                regime: current_regime,
                start_date,
                end_date: labels[index - 1].date,
                duration_days: index - start_index,
            });
            // Start new episode
            current_regime = label.regime;
            start_date = label.date;
            start_index = index;
        }
    }

    // Close final episode
    episodes.push(Episode {
        regime: current_regime,
        start_date,
        end_date: labels.last().unwrap().date,
        duration_days: labels.len() - start_index,
    });

    episodes
}

// ============================================================
// Persistence Score Calculation
// ============================================================

pub fn calculate_persistence_score(
    labels: &[RegimeLabel],
    episodes: &[Episode],
) -> PersistenceScore {
    let total_days = labels.len();

    // Episode duration statistics
    let mut durations: Vec<f64> = episodes.iter().map(|e| e.duration_days as f64).collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg_episode_days = if !durations.is_empty() {
        durations.iter().sum::<f64>() / durations.len() as f64
    } else {
        0.0
    };

    let median_episode_days = percentile(&durations, 0.50);

    let distribution = EpisodeDistribution {
        p25: percentile(&durations, 0.25),
        p50: percentile(&durations, 0.50),
        p75: percentile(&durations, 0.75),
        p95: percentile(&durations, 0.95),
    };

    // Churn rate: fraction of days where regime changed from previous day
    let mut transitions = 0usize;
    for window in labels.windows(2) {
        if window[0].regime != window[1].regime {
            transitions += 1;
        }
    }
    let churn_rate = if total_days > 1 {
        transitions as f64 / (total_days - 1) as f64
    } else {
        0.0
    };

    // Transition stability: 1 - (direct swings / total transitions)
    // Direct swing = RiskOn ↔ RiskOff (skipping Neutral)
    let mut direct_swings = 0usize;
    for window in labels.windows(2) {
        let from = window[0].regime;
        let to = window[1].regime;
        match (from, to) {
            (Regime::RiskOn, Regime::RiskOff) | (Regime::RiskOff, Regime::RiskOn) => {
                direct_swings += 1;
            }
            _ => {}
        }
    }

    let transition_stability = if transitions > 0 {
        1.0 - (direct_swings as f64 / transitions as f64)
    } else {
        1.0 // No transitions = perfect stability
    };

    PersistenceScore {
        avg_episode_days,
        median_episode_days,
        distribution,
        churn_rate,
        transition_stability,
    }
}

// ============================================================
// Coverage Score Calculation
// ============================================================

pub fn calculate_coverage_score(labels: &[RegimeLabel]) -> CoverageScore {
    let total = labels.len() as f64;
    if total == 0.0 {
        return CoverageScore {
            risk_on_pct: 0.0,
            neutral_pct: 0.0,
            risk_off_pct: 0.0,
            imbalance_ratio: 1.0,
        };
    }

    let risk_on_count = labels.iter().filter(|l| l.regime == Regime::RiskOn).count() as f64;
    let risk_off_count = labels.iter().filter(|l| l.regime == Regime::RiskOff).count() as f64;
    let neutral_count = labels.iter().filter(|l| l.regime == Regime::Neutral).count() as f64;

    let risk_on_pct = risk_on_count / total;
    let neutral_pct = neutral_count / total;
    let risk_off_pct = risk_off_count / total;

    // Imbalance ratio: max_pct / min_nonzero_pct
    let mut non_zero_pcts = Vec::new();
    if risk_on_pct > 0.0 {
        non_zero_pcts.push(risk_on_pct);
    }
    if neutral_pct > 0.0 {
        non_zero_pcts.push(neutral_pct);
    }
    if risk_off_pct > 0.0 {
        non_zero_pcts.push(risk_off_pct);
    }

    let imbalance_ratio = if non_zero_pcts.len() >= 2 {
        let max_pct = non_zero_pcts.iter().cloned().fold(0.0, f64::max);
        let min_pct = non_zero_pcts.iter().cloned().fold(1.0, f64::min);
        if min_pct > 0.0 {
            max_pct / min_pct
        } else {
            999.0
        }
    } else {
        999.0
    };

    CoverageScore {
        risk_on_pct,
        neutral_pct,
        risk_off_pct,
        imbalance_ratio,
    }
}

// ============================================================
// Full Audit Report
// ============================================================

pub fn audit_regime_labels(
    labels: &[RegimeLabel],
    gates: &AuditGates,
) -> RegimeAuditReport {
    let episodes = extract_episodes(labels);
    let persistence = calculate_persistence_score(labels, &episodes);
    let coverage = calculate_coverage_score(labels);

    let total_days = labels.len();
    let episode_count = episodes.len();

    let transition_count = if total_days > 1 {
        labels
            .windows(2)
            .filter(|w| w[0].regime != w[1].regime)
            .count()
    } else {
        0
    };

    let direct_swing_count = if total_days > 1 {
        labels
            .windows(2)
            .filter(|w| matches!(
                (w[0].regime, w[1].regime),
                (Regime::RiskOn, Regime::RiskOff) | (Regime::RiskOff, Regime::RiskOn)
            ))
            .count()
    } else {
        0
    };

    let mut violations = Vec::new();

    if persistence.avg_episode_days < gates.min_avg_episode_days {
        violations.push(format!(
            "avg_episode_days = {:.1} < threshold {:.1}",
            persistence.avg_episode_days, gates.min_avg_episode_days
        ));
    }

    if persistence.median_episode_days < gates.min_median_episode_days {
        violations.push(format!(
            "median_episode_days = {:.1} < threshold {:.1}",
            persistence.median_episode_days, gates.min_median_episode_days
        ));
    }

    if persistence.churn_rate > gates.max_churn_rate {
        violations.push(format!(
            "churn_rate = {:.2}% > threshold {:.2}%",
            persistence.churn_rate * 100.0,
            gates.max_churn_rate * 100.0
        ));
    }

    if persistence.transition_stability < gates.min_transition_stability {
        violations.push(format!(
            "transition_stability = {:.2} < threshold {:.2}",
            persistence.transition_stability, gates.min_transition_stability
        ));
    }

    if coverage.imbalance_ratio > gates.max_imbalance_ratio {
        violations.push(format!(
            "imbalance_ratio = {:.1}x > threshold {:.1}x",
            coverage.imbalance_ratio, gates.max_imbalance_ratio
        ));
    }

    if coverage.risk_on_pct < gates.min_risk_on_pct {
        violations.push(format!(
            "risk_on_pct = {:.1}% < threshold {:.1}%",
            coverage.risk_on_pct * 100.0,
            gates.min_risk_on_pct * 100.0
        ));
    }

    if coverage.risk_off_pct < gates.min_risk_off_pct {
        violations.push(format!(
            "risk_off_pct = {:.1}% < threshold {:.1}%",
            coverage.risk_off_pct * 100.0,
            gates.min_risk_off_pct * 100.0
        ));
    }

    let passed = violations.is_empty();

    RegimeAuditReport {
        persistence,
        coverage,
        total_days,
        episode_count,
        transition_count,
        direct_swing_count,
        passed,
        violations,
    }
}

/// Run audit with default gates.
pub fn audit_regime_labels_default(labels: &[RegimeLabel]) -> RegimeAuditReport {
    audit_regime_labels(labels, &AuditGates::default())
}

// ============================================================
// Helpers
// ============================================================

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

// ============================================================
// Transition Audit (TASK-020)
// Diagnoses Direct Swing events and candidate/regime distributions.
// ============================================================

use gt_regime_generator::RegimeCandidate;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DirectSwingEvent {
    pub date: NaiveDate,
    pub from_regime: Regime,
    pub to_regime: Regime,
    pub from_candidate: RegimeCandidate,
    pub to_candidate: RegimeCandidate,
    pub days_in_previous_regime: usize,
}

#[derive(Debug, Clone)]
pub struct TransitionPath {
    pub from: String,
    pub to: String,
    pub count: usize,
    pub pct: f64,
}

#[derive(Debug, Clone)]
pub struct TransitionAuditReport {
    pub candidate_distribution: HashMap<String, f64>,
    pub regime_distribution: HashMap<String, f64>,
    pub direct_swings: Vec<DirectSwingEvent>,
    pub transition_paths: Vec<TransitionPath>,
    pub total_transitions: usize,
    pub direct_swing_count: usize,
    pub direct_swing_ratio: f64,
}

pub fn audit_transitions(labels: &[RegimeLabel]) -> TransitionAuditReport {
    let total = labels.len() as f64;

    // Candidate distribution (before persistence)
    let mut candidate_counts: HashMap<String, usize> = HashMap::new();
    for label in labels {
        let key = format!("{:?}", label.candidate).to_lowercase();
        *candidate_counts.entry(key).or_insert(0) += 1;
    }
    let candidate_distribution: HashMap<String, f64> = candidate_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v as f64 / total))
        .collect();

    // Regime distribution (after persistence)
    let mut regime_counts: HashMap<String, usize> = HashMap::new();
    for label in labels {
        let key = format!("{:?}", label.regime).to_lowercase();
        *regime_counts.entry(key).or_insert(0) += 1;
    }
    let regime_distribution: HashMap<String, f64> = regime_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v as f64 / total))
        .collect();

    // Transition paths
    let mut path_counts: HashMap<(String, String), usize> = HashMap::new();
    let mut direct_swings = Vec::new();

    let episodes = extract_episodes(labels);
    let mut episode_index = 0;
    let mut days_in_current_episode = 0usize;

    for window in labels.windows(2) {
        let from_label = &window[0];
        let to_label = &window[1];

        if from_label.regime != to_label.regime {
            let from_key = format!("{:?}", from_label.regime).to_lowercase();
            let to_key = format!("{:?}", to_label.regime).to_lowercase();
            *path_counts.entry((from_key, to_key)).or_insert(0) += 1;

            // Check for direct swing: RiskOn <-> RiskOff
            let is_direct_swing = matches!(
                (from_label.regime, to_label.regime),
                (Regime::RiskOn, Regime::RiskOff) | (Regime::RiskOff, Regime::RiskOn)
            );

            if is_direct_swing {
                // Find days in previous regime (current episode length so far)
                let days_in_regime = if episode_index < episodes.len() {
                    episodes[episode_index].duration_days
                } else {
                    days_in_current_episode
                };

                direct_swings.push(DirectSwingEvent {
                    date: to_label.date,
                    from_regime: from_label.regime,
                    to_regime: to_label.regime,
                    from_candidate: from_label.candidate,
                    to_candidate: to_label.candidate,
                    days_in_previous_regime: days_in_regime,
                });

                episode_index += 1;
            }
        }

        days_in_current_episode += 1;
    }

    let total_transitions: usize = path_counts.values().sum();
    let direct_swing_count = direct_swings.len();
    let direct_swing_ratio = if total_transitions > 0 {
        direct_swing_count as f64 / total_transitions as f64
    } else {
        0.0
    };

    // Sort transition paths by count desc
    let mut paths: Vec<TransitionPath> = path_counts
        .into_iter()
        .map(|((from, to), count)| TransitionPath {
            from,
            to,
            count,
            pct: if total_transitions > 0 {
                count as f64 / total_transitions as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();
    paths.sort_by(|a, b| b.count.cmp(&a.count));

    TransitionAuditReport {
        candidate_distribution,
        regime_distribution,
        direct_swings,
        transition_paths: paths,
        total_transitions,
        direct_swing_count,
        direct_swing_ratio,
    }
}

// ============================================================
// Factor Attribution Audit (TASK-021)
// Diagnoses which observation dimension compresses Neutral.
// ============================================================

use market_state_extractor::{
    MarketStateObservation, TrendDirection, VolatilityRegime,
};

#[derive(Debug, Clone)]
pub struct DrawdownStats {
    pub avg: f64,
    pub median: f64,
    pub p10: f64,
    pub p90: f64,
    pub max: f64,
}

#[derive(Debug, Clone)]
pub struct FactorAttributionReport {
    pub trend_short_distribution: HashMap<String, f64>,
    pub trend_medium_distribution: HashMap<String, f64>,
    pub volatility_distribution: HashMap<String, f64>,
    pub liquidity_distribution: HashMap<String, f64>,
    pub drawdown_stats: DrawdownStats,
    pub risk_on_trigger_breakdown: HashMap<String, f64>,
    pub risk_off_trigger_breakdown: HashMap<String, f64>,
}

pub fn audit_factor_attribution(observations: &[MarketStateObservation]) -> FactorAttributionReport {
    let total = observations.len() as f64;
    if total == 0.0 {
        return FactorAttributionReport {
            trend_short_distribution: HashMap::new(),
            trend_medium_distribution: HashMap::new(),
            volatility_distribution: HashMap::new(),
            liquidity_distribution: HashMap::new(),
            drawdown_stats: DrawdownStats {
                avg: 0.0,
                median: 0.0,
                p10: 0.0,
                p90: 0.0,
                max: 0.0,
            },
            risk_on_trigger_breakdown: HashMap::new(),
            risk_off_trigger_breakdown: HashMap::new(),
        };
    }

    // Trend distributions
    let mut trend_short_counts: HashMap<String, usize> = HashMap::new();
    let mut trend_medium_counts: HashMap<String, usize> = HashMap::new();
    for obs in observations {
        let short_key = format!("{:?}", obs.trend.short_term).to_lowercase();
        let medium_key = format!("{:?}", obs.trend.medium_term).to_lowercase();
        *trend_short_counts.entry(short_key).or_insert(0) += 1;
        *trend_medium_counts.entry(medium_key).or_insert(0) += 1;
    }

    // Volatility distribution
    let mut vol_counts: HashMap<String, usize> = HashMap::new();
    for obs in observations {
        let key = format!("{:?}", obs.volatility.volatility_regime).to_lowercase();
        *vol_counts.entry(key).or_insert(0) += 1;
    }

    // Liquidity distribution
    let mut liq_counts: HashMap<String, usize> = HashMap::new();
    for obs in observations {
        let key = format!("{:?}", obs.liquidity.volume_regime).to_lowercase();
        *liq_counts.entry(key).or_insert(0) += 1;
    }

    // Drawdown stats
    let mut drawdowns: Vec<f64> = observations.iter().map(|o| o.drawdown_pct).collect();
    drawdowns.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let drawdown_stats = DrawdownStats {
        avg: drawdowns.iter().sum::<f64>() / drawdowns.len() as f64,
        median: percentile(&drawdowns, 0.50),
        p10: percentile(&drawdowns, 0.10),
        p90: percentile(&drawdowns, 0.90),
        max: drawdowns.iter().cloned().fold(0.0, f64::max),
    };

    // RiskOn/RiskOff trigger breakdown (which factor pushed it over the edge)
    let mut risk_on_triggers: HashMap<String, usize> = HashMap::new();
    let mut risk_off_triggers: HashMap<String, usize> = HashMap::new();

    for obs in observations {
        // RiskOn triggers: trend_ok, vol_ok, drawdown_ok
        let trend_ok = matches!(
            obs.trend.short_term,
            TrendDirection::StrongUptrend | TrendDirection::Uptrend
        ) && matches!(
            obs.trend.medium_term,
            TrendDirection::StrongUptrend | TrendDirection::Uptrend
        );
        let vol_ok = matches!(
            obs.volatility.volatility_regime,
            VolatilityRegime::Low | VolatilityRegime::Normal
        );
        let drawdown_ok = obs.drawdown_pct > -10.0;

        if trend_ok {
            *risk_on_triggers.entry("trend".to_string()).or_insert(0) += 1;
        }
        if vol_ok {
            *risk_on_triggers.entry("volatility".to_string()).or_insert(0) += 1;
        }
        if drawdown_ok {
            *risk_on_triggers.entry("drawdown".to_string()).or_insert(0) += 1;
        }

        // RiskOff triggers
        let trend_bad = matches!(
            obs.trend.short_term,
            TrendDirection::StrongDowntrend | TrendDirection::Downtrend
        ) || matches!(
            obs.trend.medium_term,
            TrendDirection::StrongDowntrend | TrendDirection::Downtrend
        );
        let vol_bad = matches!(
            obs.volatility.volatility_regime,
            VolatilityRegime::Elevated | VolatilityRegime::Spike
        );
        let drawdown_bad = obs.drawdown_pct < -20.0;

        if trend_bad {
            *risk_off_triggers.entry("trend".to_string()).or_insert(0) += 1;
        }
        if vol_bad {
            *risk_off_triggers.entry("volatility".to_string()).or_insert(0) += 1;
        }
        if drawdown_bad {
            *risk_off_triggers.entry("drawdown".to_string()).or_insert(0) += 1;
        }
    }

    FactorAttributionReport {
        trend_short_distribution: trend_short_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect(),
        trend_medium_distribution: trend_medium_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect(),
        volatility_distribution: vol_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect(),
        liquidity_distribution: liq_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect(),
        drawdown_stats,
        risk_on_trigger_breakdown: risk_on_triggers
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect(),
        risk_off_trigger_breakdown: risk_off_triggers
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect(),
    }
}

pub mod common;

pub mod alignment_redesign;
pub mod allocation_prototype;
pub mod attribution;
pub mod dual_layer_validation;
pub mod economic_attribution;
pub mod economic_regime_prototype;
pub mod economic_replay;
pub mod episode_survival;
pub mod external_validation;
pub mod factor_alignment;
pub mod forward_return_distribution;
pub mod ground_truth_audit;
pub mod ground_truth_generator;
pub mod label_distribution;
pub mod lead_lag_analysis;
pub mod market_structure;
pub mod pareto_frontier;
pub mod persistence_frontier;
pub mod persistence_mechanics;
pub mod score_distribution;
pub mod sensitivity_replay;
pub mod state_alignment;
pub mod state_gt_validation;
pub mod state_persistence_economics;
pub mod state_signal_decomposition;
pub mod wave8_revalidation;

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use gt_regime_generator::{RegimeCandidate, RegimeLabel};

    fn make_label(date: NaiveDate, regime: Regime, candidate: RegimeCandidate) -> RegimeLabel {
        RegimeLabel {
            regime,
            candidate,
            confidence: 80.0,
            days_in_regime: 1,
            date,
            scope: "TEST".to_string(),
        }
    }

    #[test]
    fn test_extract_episodes() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let labels: Vec<RegimeLabel> = (0..30)
            .map(|i| {
                let regime = if i < 10 {
                    Regime::RiskOn
                } else if i < 25 {
                    Regime::Neutral
                } else {
                    Regime::RiskOff
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let episodes = extract_episodes(&labels);
        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[0].regime, Regime::RiskOn);
        assert_eq!(episodes[0].duration_days, 10);
        assert_eq!(episodes[1].regime, Regime::Neutral);
        assert_eq!(episodes[1].duration_days, 15);
        assert_eq!(episodes[2].regime, Regime::RiskOff);
        assert_eq!(episodes[2].duration_days, 5);
    }

    #[test]
    fn test_persistence_score() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let labels: Vec<RegimeLabel> = (0..100)
            .map(|i| {
                let regime = if i < 30 {
                    Regime::RiskOn
                } else if i < 70 {
                    Regime::Neutral
                } else {
                    Regime::RiskOff
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let episodes = extract_episodes(&labels);
        let score = calculate_persistence_score(&labels, &episodes);

        assert!(score.avg_episode_days > 30.0, "avg should be ~33.3");
        assert_eq!(score.median_episode_days, 30.0);
        assert!(score.churn_rate < 0.05, "Low churn across episode boundaries: {}", score.churn_rate);
        assert_eq!(score.transition_stability, 1.0, "No direct swings");
    }

    #[test]
    fn test_coverage_score() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let labels: Vec<RegimeLabel> = (0..100)
            .map(|i| {
                let regime = if i < 20 {
                    Regime::RiskOn
                } else if i < 30 {
                    Regime::RiskOff
                } else {
                    Regime::Neutral
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let coverage = calculate_coverage_score(&labels);
        assert!((coverage.risk_on_pct - 0.20).abs() < 0.01);
        assert!((coverage.risk_off_pct - 0.10).abs() < 0.01);
        assert!((coverage.neutral_pct - 0.70).abs() < 0.01);
        assert!(coverage.imbalance_ratio > 1.0);
    }

    #[test]
    fn test_churn_rate() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        // Alternating every day: high churn
        let labels: Vec<RegimeLabel> = (0..20)
            .map(|i| {
                let regime = if i % 2 == 0 {
                    Regime::RiskOn
                } else {
                    Regime::RiskOff
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let episodes = extract_episodes(&labels);
        let score = calculate_persistence_score(&labels, &episodes);

        assert!(score.churn_rate > 0.9, "churn should be very high");
        assert_eq!(score.transition_stability, 0.0, "all transitions are direct swings");
    }

    #[test]
    fn test_audit_passes() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        // Well-balanced, long episodes
        let labels: Vec<RegimeLabel> = (0..300)
            .map(|i| {
                let regime = if i < 100 {
                    Regime::RiskOn
                } else if i < 200 {
                    Regime::Neutral
                } else {
                    Regime::RiskOff
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let report = audit_regime_labels_default(&labels);
        assert!(report.passed, "Should pass with 100-day episodes and 33% each");
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_audit_fails_imbalance() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        // 94% neutral, 5% risk_on, 1% risk_off — old GT problem
        let labels: Vec<RegimeLabel> = (0..100)
            .map(|i| {
                let regime = if i < 5 {
                    Regime::RiskOn
                } else if i < 6 {
                    Regime::RiskOff
                } else {
                    Regime::Neutral
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let report = audit_regime_labels_default(&labels);
        assert!(!report.passed, "Should fail due to imbalance and low coverage");
        assert!(
            report.violations.iter().any(|v| v.contains("imbalance_ratio")),
            "Should flag imbalance"
        );
        assert!(
            report.violations.iter().any(|v| v.contains("risk_off_pct")),
            "Should flag low risk_off coverage"
        );
    }

    #[test]
    fn test_audit_fails_churn() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        // High churn: switches every 3 days
        let mut labels = Vec::new();
        for i in 0..100 {
            let regime = match (i / 3) % 3 {
                0 => Regime::RiskOn,
                1 => Regime::Neutral,
                _ => Regime::RiskOff,
            };
            labels.push(make_label(
                start + chrono::Duration::days(i),
                regime,
                RegimeCandidate::Neutral,
            ));
        }

        let report = audit_regime_labels_default(&labels);
        assert!(!report.passed, "Should fail due to high churn");
        assert!(
            report.violations.iter().any(|v| v.contains("churn_rate")),
            "Should flag high churn"
        );
        assert!(
            report.violations.iter().any(|v| v.contains("avg_episode")),
            "Should flag short episodes"
        );
    }

    #[test]
    fn test_episode_distribution() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let labels: Vec<RegimeLabel> = (0..100)
            .map(|i| {
                let regime = if i < 10 {
                    Regime::RiskOn
                } else if i < 30 {
                    Regime::Neutral
                } else if i < 80 {
                    Regime::RiskOff
                } else {
                    Regime::Neutral
                };
                make_label(start + chrono::Duration::days(i), regime, RegimeCandidate::Neutral)
            })
            .collect();

        let episodes = extract_episodes(&labels);
        let score = calculate_persistence_score(&labels, &episodes);

        assert_eq!(score.distribution.p50, 20.0); // median of [10, 20, 50, 20]
        assert!(score.distribution.p25 <= score.distribution.p50);
        assert!(score.distribution.p75 >= score.distribution.p50);
        assert!(score.distribution.p95 >= score.distribution.p75);
    }
}
