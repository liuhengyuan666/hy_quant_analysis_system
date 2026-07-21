//! EXPERIMENT — 2B-1 one-off analysis tool.
//!
//! Research-phase diagnostic; not part of the production Shadow Validation
//! path. Retained for evidence composition inspection.

use std::collections::HashMap;

use chrono::NaiveDate;
use execution_engine::v2::assessment::RiskLevel;
use execution_engine::v2::evidence::EvidenceKind;
use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// 2B-1: Bearish Evidence Analysis.
///
/// Analyzes bearish Assessment candidates (dominant_direction < reduce_threshold)
/// and their Evidence composition against historical outcomes. The goal is to
/// discover which Evidence combinations distinguish exit-requiring states from
/// temporary risk states, before any new EvidenceKind is introduced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearishAnalysis {
    pub total_records: usize,
    pub bearish_candidates: usize,
    pub baseline_negative_t20_rate: f64,
    pub baseline_negative_t60_rate: f64,
    pub evidence_matrix: Vec<EvidenceCombinationRow>,
    pub evidence_lift: Vec<EvidenceLiftRow>,
    pub recovery_conflict: RecoveryConflict,
    pub false_reduce_analysis: FalseReduceAnalysis,
    pub risk_expansion_coverage: RiskExpansionCoverage,
    pub recommendation: String,
}

/// RiskExpansion coverage exploration (TASK-153.5).
///
/// RiskExpansion is produced by the VolatilityExpansion observation, which fires
/// when `amplitude_pct > 0.05`. This struct diagnoses whether that threshold is
/// too strict, and whether RiskExpansion is scarce alpha or just under-covered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskExpansionCoverage {
    pub current_threshold: f64,
    pub total_records: usize,
    pub triggered_count: usize,
    pub coverage_pct: f64,
    pub triggered_in_bearish_count: usize,
    pub bearish_candidates: usize,
    pub amplitude_percentiles: PercentileSummary,
    pub threshold_sensitivity: Vec<RiskExpansionThresholdRow>,
    pub near_miss_analysis: Vec<RiskExpansionThresholdRow>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskExpansionThresholdRow {
    pub threshold: f64,
    pub count: usize,
    pub negative_t20_count: usize,
    pub negative_t20_rate: f64,
    pub lift_vs_baseline: f64,
    pub avg_t20: f64,
    pub avg_t60: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileSummary {
    pub min: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub max: f64,
    pub mean: f64,
}

/// A single row in the Evidence combination outcome matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCombinationRow {
    pub combination: Vec<String>,
    pub count: usize,
    pub negative_t20_count: usize,
    pub negative_t60_count: usize,
    pub negative_t20_rate: f64,
    pub negative_t60_rate: f64,
    pub avg_t20: f64,
    pub avg_t60: f64,
}

/// Evidence lift: how much a given evidence (or combination) increases the
/// probability of a negative T+20 outcome relative to the bearish baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLiftRow {
    pub evidence: String,
    pub count: usize,
    pub negative_t20_rate: f64,
    pub baseline_negative_t20_rate: f64,
    pub lift: f64,
}

/// Recovery conflict analysis: bearish candidates with Recovery evidence vs
/// without Recovery evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConflict {
    pub with_recovery: ConflictGroup,
    pub without_recovery: ConflictGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictGroup {
    pub count: usize,
    pub negative_t20_count: usize,
    pub negative_t20_rate: f64,
    pub avg_t20: f64,
    pub avg_t60: f64,
}

/// Analysis of false Reduce actions produced by the C3 calibration experiment
/// (confidence threshold 0.45). These are records that would be Reduce under
/// C3 but had non-negative T+20 outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalseReduceAnalysis {
    pub c3_reduce_count: usize,
    pub false_reduce_count: usize,
    pub false_reduce_rate: f64,
    pub avg_t20: f64,
    pub top_evidence_combinations: Vec<EvidenceCombinationRow>,
}

/// Per-record detail for a bearish candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearishCandidateRecord {
    pub execution_id: String,
    pub symbol: String,
    pub date: NaiveDate,
    pub dominant_direction: f64,
    pub confidence: f64,
    pub consensus: f64,
    pub risk: RiskLevel,
    pub evidences: Vec<String>,
    pub t20_return: Option<f64>,
    pub t60_return: Option<f64>,
    pub is_c3_reduce: bool,
    pub is_false_reduce: bool,
}

/// Computes the bearish evidence analysis over a set of records.
///
/// This is a read-only research tool: it does not modify Evidence, Assessment,
/// Decision, or Policy.
pub fn compute_bearish_analysis(records: &[ExecutionResearchRecord]) -> BearishAnalysis {
    let total_records = records.len();
    let bearish_candidates: Vec<&ExecutionResearchRecord> = records
        .iter()
        .filter(|r| {
            r.event.decision.assessment.dominant_direction < r.event.policy.reduce_threshold
        })
        .collect();
    let bearish_count = bearish_candidates.len();

    let (base_neg_t20, base_neg_t60, _sum_t20, _sum_t60) =
        aggregate_outcomes(&bearish_candidates);
    let baseline_negative_t20_rate = safe_rate(base_neg_t20, bearish_count);
    let baseline_negative_t60_rate = safe_rate(base_neg_t60, bearish_count);

    // Evidence lift for single evidence kinds.
    let evidence_lift = compute_evidence_lift(
        &bearish_candidates,
        baseline_negative_t20_rate,
    );

    // Evidence combination matrix for selected combinations.
    let evidence_matrix = compute_evidence_matrix(&bearish_candidates);

    // Recovery conflict.
    let recovery_conflict = compute_recovery_conflict(&bearish_candidates);

    // C3 false reduce analysis.
    let false_reduce_analysis = compute_false_reduce_analysis(&bearish_candidates);

    // TASK-153.5: RiskExpansion coverage exploration.
    let risk_expansion_coverage = compute_risk_expansion_coverage(records, &bearish_candidates, baseline_negative_t20_rate);

    let recommendation = build_recommendation(
        &evidence_lift,
        &evidence_matrix,
        &recovery_conflict,
        &false_reduce_analysis,
        &risk_expansion_coverage,
        baseline_negative_t20_rate,
    );

    BearishAnalysis {
        total_records,
        bearish_candidates: bearish_count,
        baseline_negative_t20_rate,
        baseline_negative_t60_rate,
        evidence_matrix,
        evidence_lift,
        recovery_conflict,
        false_reduce_analysis,
        risk_expansion_coverage,
        recommendation,
    }
}

fn aggregate_outcomes(records: &[&ExecutionResearchRecord]) -> (usize, usize, f64, f64) {
    let mut neg_t20 = 0usize;
    let mut neg_t60 = 0usize;
    let mut sum_t20 = 0.0;
    let mut sum_t60 = 0.0;
    let mut count_t20 = 0usize;
    let mut count_t60 = 0usize;

    for r in records {
        if let Some(t20) = r.outcome.t20_return {
            sum_t20 += t20;
            count_t20 += 1;
            if t20 < 0.0 {
                neg_t20 += 1;
            }
        }
        if let Some(t60) = r.outcome.t60_return {
            sum_t60 += t60;
            count_t60 += 1;
            if t60 < 0.0 {
                neg_t60 += 1;
            }
        }
    }

    (
        neg_t20,
        neg_t60,
        if count_t20 > 0 { sum_t20 } else { 0.0 },
        if count_t60 > 0 { sum_t60 } else { 0.0 },
    )
}

fn safe_rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn safe_avg(sum: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn evidence_kinds(record: &ExecutionResearchRecord) -> Vec<EvidenceKind> {
    record
        .event
        .decision
        .evidences
        .iter()
        .map(|e| e.kind)
        .collect()
}

fn has_evidence(record: &ExecutionResearchRecord, kind: EvidenceKind) -> bool {
    evidence_kinds(record).contains(&kind)
}

fn evidence_names(kinds: &[EvidenceKind]) -> Vec<String> {
    kinds
        .iter()
        .map(|k| format!("{:?}", k))
        .collect()
}

fn compute_evidence_lift(
    records: &[&ExecutionResearchRecord],
    baseline_negative_t20_rate: f64,
) -> Vec<EvidenceLiftRow> {
    let all_kinds = vec![
        EvidenceKind::Distribution,
        EvidenceKind::RiskExpansion,
        EvidenceKind::MomentumFailure,
        EvidenceKind::LiquidityConfirmation,
        EvidenceKind::Recovery,
        EvidenceKind::Breadth,
        EvidenceKind::LeadershipRotation,
        EvidenceKind::Confirmation,
        EvidenceKind::StrategyState,
        EvidenceKind::SignalStrength,
    ];

    let mut rows = Vec::new();
    for kind in all_kinds {
        let subset: Vec<_> = records
            .iter()
            .filter(|r| has_evidence(r, kind))
            .copied()
            .collect();
        let (neg_t20, _neg_t60, sum_t20, _sum_t60) = aggregate_outcomes(&subset);
        let count = subset.len();
        let negative_t20_rate = safe_rate(neg_t20, count);
        let _avg_t20 = safe_avg(sum_t20, count);
        let lift = if baseline_negative_t20_rate > 0.0 {
            negative_t20_rate / baseline_negative_t20_rate
        } else {
            0.0
        };
        rows.push(EvidenceLiftRow {
            evidence: format!("{:?}", kind),
            count,
            negative_t20_rate,
            baseline_negative_t20_rate,
            lift,
        });
    }

    // Sort by lift descending, then count descending.
    rows.sort_by(|a, b| {
        b.lift
            .partial_cmp(&a.lift)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.count.cmp(&a.count))
    });

    rows
}

fn compute_evidence_matrix(records: &[&ExecutionResearchRecord]) -> Vec<EvidenceCombinationRow> {
    let mut combinations: Vec<Vec<EvidenceKind>> = Vec::new();

    // Single evidence kinds.
    let singles = vec![
        EvidenceKind::Distribution,
        EvidenceKind::RiskExpansion,
        EvidenceKind::MomentumFailure,
        EvidenceKind::LiquidityConfirmation,
        EvidenceKind::Recovery,
    ];
    for k in &singles {
        combinations.push(vec![*k]);
    }

    // Pairs of interest.
    let pairs = vec![
        (EvidenceKind::Distribution, EvidenceKind::RiskExpansion),
        (EvidenceKind::Distribution, EvidenceKind::Recovery),
        (EvidenceKind::Distribution, EvidenceKind::LiquidityConfirmation),
        (EvidenceKind::RiskExpansion, EvidenceKind::Recovery),
        (EvidenceKind::RiskExpansion, EvidenceKind::LiquidityConfirmation),
        (EvidenceKind::Distribution, EvidenceKind::Breadth),
    ];
    for (a, b) in pairs {
        combinations.push(vec![a, b]);
    }

    // Triples of interest.
    let triples = vec![
        (
            EvidenceKind::Distribution,
            EvidenceKind::RiskExpansion,
            EvidenceKind::Recovery,
        ),
        (
            EvidenceKind::Distribution,
            EvidenceKind::RiskExpansion,
            EvidenceKind::LiquidityConfirmation,
        ),
    ];
    for (a, b, c) in triples {
        combinations.push(vec![a, b, c]);
    }

    let mut rows = Vec::new();
    for combo in combinations {
        let subset: Vec<_> = records
            .iter()
            .filter(|r| {
                let kinds = evidence_kinds(r);
                combo.iter().all(|k| kinds.contains(k))
            })
            .copied()
            .collect();
        let (neg_t20, neg_t60, sum_t20, sum_t60) = aggregate_outcomes(&subset);
        let count = subset.len();
        rows.push(EvidenceCombinationRow {
            combination: evidence_names(&combo),
            count,
            negative_t20_count: neg_t20,
            negative_t60_count: neg_t60,
            negative_t20_rate: safe_rate(neg_t20, count),
            negative_t60_rate: safe_rate(neg_t60, count),
            avg_t20: safe_avg(sum_t20, count),
            avg_t60: safe_avg(sum_t60, count),
        });
    }

    // Sort by negative_t20_rate descending, then count descending.
    rows.sort_by(|a, b| {
        b.negative_t20_rate
            .partial_cmp(&a.negative_t20_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.count.cmp(&a.count))
    });

    rows
}

fn compute_recovery_conflict(records: &[&ExecutionResearchRecord]) -> RecoveryConflict {
    let with_recovery: Vec<_> = records
        .iter()
        .filter(|r| has_evidence(r, EvidenceKind::Recovery))
        .copied()
        .collect();
    let without_recovery: Vec<_> = records
        .iter()
        .filter(|r| !has_evidence(r, EvidenceKind::Recovery))
        .copied()
        .collect();

    let make_group = |subset: &[&ExecutionResearchRecord]| {
        let (neg_t20, _, sum_t20, sum_t60) = aggregate_outcomes(subset);
        ConflictGroup {
            count: subset.len(),
            negative_t20_count: neg_t20,
            negative_t20_rate: safe_rate(neg_t20, subset.len()),
            avg_t20: safe_avg(sum_t20, subset.len()),
            avg_t60: safe_avg(sum_t60, subset.len()),
        }
    };

    RecoveryConflict {
        with_recovery: make_group(&with_recovery),
        without_recovery: make_group(&without_recovery),
    }
}

fn compute_false_reduce_analysis(
    records: &[&ExecutionResearchRecord],
) -> FalseReduceAnalysis {
    // Simulate C3 experiment: uniform confidence threshold 0.45.
    let c3_threshold = 0.45;
    let c3_reduces: Vec<_> = records
        .iter()
        .filter(|r| would_be_reduce(r, c3_threshold))
        .copied()
        .collect();

    let false_reduces: Vec<_> = c3_reduces
        .iter()
        .filter(|r| {
            r.outcome
                .t20_return
                .map(|t20| t20 >= 0.0)
                .unwrap_or(false)
        })
        .copied()
        .collect();

    let c3_reduce_count = c3_reduces.len();
    let false_reduce_count = false_reduces.len();
    let false_reduce_rate = safe_rate(false_reduce_count, c3_reduce_count);

    let sum_t20: f64 = c3_reduces
        .iter()
        .filter_map(|r| r.outcome.t20_return)
        .sum();
    let avg_t20 = safe_avg(sum_t20, c3_reduce_count);

    // Top evidence combinations among false reduces.
    let mut combo_counts: HashMap<Vec<String>, usize> = HashMap::new();
    for r in &false_reduces {
        let mut kinds = evidence_kinds(r);
        kinds.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        let names = evidence_names(&kinds);
        *combo_counts.entry(names).or_insert(0) += 1;
    }
    let mut top_combos: Vec<EvidenceCombinationRow> = combo_counts
        .into_iter()
        .map(|(combination, count)| EvidenceCombinationRow {
            combination,
            count,
            negative_t20_count: 0,
            negative_t60_count: 0,
            negative_t20_rate: 0.0,
            negative_t60_rate: 0.0,
            avg_t20: 0.0,
            avg_t60: 0.0,
        })
        .collect();
    top_combos.sort_by(|a, b| b.count.cmp(&a.count));
    top_combos.truncate(10);

    FalseReduceAnalysis {
        c3_reduce_count,
        false_reduce_count,
        false_reduce_rate,
        avg_t20,
        top_evidence_combinations: top_combos,
    }
}

fn would_be_reduce(record: &ExecutionResearchRecord, confidence_threshold: f64) -> bool {
    let assessment = &record.event.decision.assessment;
    let policy = &record.event.policy;

    if assessment.risk == RiskLevel::Critical || assessment.risk == RiskLevel::High {
        return false;
    }
    if assessment.confidence < confidence_threshold {
        return false;
    }
    if assessment.consensus < policy.consensus_threshold {
        return false;
    }
    if assessment.dominant_direction > policy.buy_threshold {
        return false;
    }
    if assessment.dominant_direction < policy.reduce_threshold {
        return true;
    }
    false
}

/// TASK-153.5: RiskExpansion coverage exploration.
///
/// RiskExpansion is produced by VolatilityExpansion observation when
/// `amplitude_pct > 0.05`. This function explores whether that threshold is
/// too strict, and whether RiskExpansion is scarce alpha or under-covered.
fn compute_risk_expansion_coverage(
    all_records: &[ExecutionResearchRecord],
    bearish_candidates: &[&ExecutionResearchRecord],
    baseline_negative_t20_rate: f64,
) -> RiskExpansionCoverage {
    const CURRENT_THRESHOLD: f64 = 0.05;

    let total_records = all_records.len();
    let bearish_count = bearish_candidates.len();

    let all_amplitudes: Vec<f64> = all_records
        .iter()
        .map(|r| r.event.features.amplitude_pct)
        .collect();
    let amplitude_percentiles = compute_percentiles(&all_amplitudes);

    let triggered_count = all_records
        .iter()
        .filter(|r| r.event.features.amplitude_pct > CURRENT_THRESHOLD)
        .count();
    let coverage_pct = safe_rate(triggered_count, total_records);

    let triggered_in_bearish_count = bearish_candidates
        .iter()
        .filter(|r| r.event.features.amplitude_pct > CURRENT_THRESHOLD)
        .count();

    // Threshold sensitivity: test a range of thresholds.
    let thresholds = vec![0.010, 0.020, 0.025, 0.030, 0.035, 0.040, 0.045, 0.050, 0.060, 0.080, 0.100, 0.150];
    let mut threshold_sensitivity = Vec::new();
    for threshold in thresholds {
        let subset: Vec<_> = all_records
            .iter()
            .filter(|r| r.event.features.amplitude_pct >= threshold)
            .collect();
        let (neg_t20, _neg_t60, sum_t20, sum_t60) = aggregate_outcomes(&subset);
        let count = subset.len();
        let negative_t20_rate = safe_rate(neg_t20, count);
        let lift = if baseline_negative_t20_rate > 0.0 {
            negative_t20_rate / baseline_negative_t20_rate
        } else {
            0.0
        };
        threshold_sensitivity.push(RiskExpansionThresholdRow {
            threshold,
            count,
            negative_t20_count: neg_t20,
            negative_t20_rate,
            lift_vs_baseline: lift,
            avg_t20: safe_avg(sum_t20, count),
            avg_t60: safe_avg(sum_t60, count),
        });
    }

    // Near-miss analysis: records in [threshold, current_threshold) for thresholds below current.
    let near_miss_thresholds = vec![0.03, 0.035, 0.04, 0.045];
    let mut near_miss_analysis = Vec::new();
    for lower in near_miss_thresholds {
        let subset: Vec<_> = all_records
            .iter()
            .filter(|r| {
                let a = r.event.features.amplitude_pct;
                a >= lower && a < CURRENT_THRESHOLD
            })
            .collect();
        let (neg_t20, _neg_t60, sum_t20, sum_t60) = aggregate_outcomes(&subset);
        let count = subset.len();
        let negative_t20_rate = safe_rate(neg_t20, count);
        let lift = if baseline_negative_t20_rate > 0.0 {
            negative_t20_rate / baseline_negative_t20_rate
        } else {
            0.0
        };
        near_miss_analysis.push(RiskExpansionThresholdRow {
            threshold: lower,
            count,
            negative_t20_count: neg_t20,
            negative_t20_rate,
            lift_vs_baseline: lift,
            avg_t20: safe_avg(sum_t20, count),
            avg_t60: safe_avg(sum_t60, count),
        });
    }

    let recommendation = build_risk_expansion_recommendation(
        coverage_pct,
        triggered_in_bearish_count,
        bearish_count,
        &threshold_sensitivity,
    );

    RiskExpansionCoverage {
        current_threshold: CURRENT_THRESHOLD,
        total_records,
        triggered_count,
        coverage_pct,
        triggered_in_bearish_count,
        bearish_candidates: bearish_count,
        amplitude_percentiles,
        threshold_sensitivity,
        near_miss_analysis,
        recommendation,
    }
}

fn compute_percentiles(values: &[f64]) -> PercentileSummary {
    if values.is_empty() {
        return PercentileSummary {
            min: 0.0,
            p10: 0.0,
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
            p90: 0.0,
            max: 0.0,
            mean: 0.0,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    PercentileSummary {
        min: sorted[0],
        p10: percentile(&sorted, 0.10),
        p25: percentile(&sorted, 0.25),
        p50: percentile(&sorted, 0.50),
        p75: percentile(&sorted, 0.75),
        p90: percentile(&sorted, 0.90),
        max: sorted[n - 1],
        mean: sorted.iter().sum::<f64>() / n as f64,
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let idx = (p * (n as f64 - 1.0)).round() as usize;
    sorted[idx.clamp(0, n - 1)]
}

fn build_risk_expansion_recommendation(
    coverage_pct: f64,
    triggered_in_bearish_count: usize,
    bearish_count: usize,
    threshold_sensitivity: &[RiskExpansionThresholdRow],
) -> String {
    if coverage_pct < 0.01 {
        format!(
            "RiskExpansion is very rare ({:.2}% coverage). Among bearish candidates, only {} / {} have RiskExpansion. Threshold sensitivity should be reviewed to determine whether the condition is too strict.",
            coverage_pct * 100.0,
            triggered_in_bearish_count,
            bearish_count
        )
    } else if let Some(best) = threshold_sensitivity
        .iter()
        .max_by(|a, b| a.lift_vs_baseline.partial_cmp(&b.lift_vs_baseline).unwrap_or(std::cmp::Ordering::Equal))
    {
        format!(
            "RiskExpansion coverage is {:.2}%. Best sensitivity threshold {:.3} yields {} records with lift={:.2} and negative T+20={:.1}%. Evaluate whether lowering the threshold increases coverage without destroying precision.",
            coverage_pct * 100.0,
            best.threshold,
            best.count,
            best.lift_vs_baseline,
            best.negative_t20_rate * 100.0
        )
    } else {
        "No threshold sensitivity data available.".into()
    }
}

fn build_recommendation(
    evidence_lift: &[EvidenceLiftRow],
    evidence_matrix: &[EvidenceCombinationRow],
    recovery_conflict: &RecoveryConflict,
    false_reduce_analysis: &FalseReduceAnalysis,
    risk_expansion_coverage: &RiskExpansionCoverage,
    baseline_negative_t20_rate: f64,
) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "Bearish baseline negative T+20 rate: {:.1}%.",
        baseline_negative_t20_rate * 100.0
    ));

    if let Some(top) = evidence_lift.first() {
        parts.push(format!(
            "Highest single-evidence lift: {} (lift={:.2}, negative T+20={:.1}%, n={}).",
            top.evidence,
            top.lift,
            top.negative_t20_rate * 100.0,
            top.count
        ));
    }

    if let Some(top) = evidence_matrix.first() {
        parts.push(format!(
            "Highest combination: {} (negative T+20={:.1}%, n={}).",
            top.combination.join(" + "),
            top.negative_t20_rate * 100.0,
            top.count
        ));
    }

    parts.push(format!(
        "Recovery conflict: bearish WITH Recovery has negative T+20={:.1}%; WITHOUT Recovery has negative T+20={:.1}%.",
        recovery_conflict.with_recovery.negative_t20_rate * 100.0,
        recovery_conflict.without_recovery.negative_t20_rate * 100.0
    ));

    parts.push(format!(
        "C3 false reduce rate: {:.1}% ({} / {}).",
        false_reduce_analysis.false_reduce_rate * 100.0,
        false_reduce_analysis.false_reduce_count,
        false_reduce_analysis.c3_reduce_count
    ));

    parts.push(format!(
        "RiskExpansion coverage: {:.2}% of all records; threshold sensitivity shows {} candidates at threshold {:.3} with negative T+20={:.1}%.",
        risk_expansion_coverage.coverage_pct * 100.0,
        risk_expansion_coverage
            .threshold_sensitivity
            .first()
            .map(|r| r.count)
            .unwrap_or(0),
        risk_expansion_coverage.current_threshold,
        risk_expansion_coverage
            .threshold_sensitivity
            .first()
            .map(|r| r.negative_t20_rate * 100.0)
            .unwrap_or(0.0)
    ));

    parts.push(
        "Next step: evaluate whether the highest-lift combinations are stable enough to form a new Exit-specific evidence condition."
            .into(),
    );

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use execution_engine::ExecutionState;
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::evidence::{Evidence, EvidencePayload, EvidenceSource};
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record(
        direction: f64,
        confidence: f64,
        t20_return: f64,
        evidences: Vec<EvidenceKind>,
    ) -> ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence,
            consensus: 0.6,
            coverage: 1.0,
            risk: RiskLevel::Medium,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: ExecutionState::Wait,
            confidence,
            risk: RiskLevel::Medium,
            evidences: evidences
                .into_iter()
                .map(|k| Evidence {
                    kind: k,
                    confidence: 0.8,
                    direction: -1.0,
                    source: EvidenceSource::IntradayObservation,
                    payload: EvidencePayload::Empty,
                })
                .collect(),
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };
        let policy = ExecutionPolicy::default();
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                symbol: "000001".into(),
                final_score: 70.0,
                signal_label: SignalLabel::Buy,
                analysis_scope: "CN".into(),
                regime_basis_scope: "CN".into(),
                reason: core_domain::SignalReason {
                    best_strategy: StrategyKind::MomentumRight,
                    strategy_score: 0.0,
                    strategy_contribution: 0.0,
                    alignment: 0,
                    aligned_strategies: vec![],
                    alignment_contribution: 0.0,
                    regime: core_domain::RegimeReason {
                        trend_score: 0.0,
                        risk_score: 0.0,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    rotation: core_domain::RotationReason {
                        momentum_score: 0.0,
                        rank: None,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    final_score: 70.0,
                    label: SignalLabel::Buy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                scope: "CN".into(),
                state: StrategyState::NoTrade,
                state_score: 50.0,
                transition_reason: "test".into(),
                recommended_position_pct: 0.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.5,
                close: 10.5,
                volume: 1_000_000.0,
                prev_close: 10.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    participation: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    risk: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    overall: "Moderate".into(),
                },
                breadth: BreadthSummary {
                    breadth_pct: 50.0,
                    sma5: None,
                    delta_5d: None,
                    condition: "moderate".into(),
                },
                recovery: RecoverySummary {
                    score: 50.0,
                    drivers: vec![],
                },
                rotation_state: "mixed".into(),
                leadership_stability: 0.5,
            },
            policy,
        };
        let event = ExecutionEvent::new(request, Default::default(), vec![], vec![], assessment, decision);
        ExecutionResearchRecord {
            event,
            outcome: crate::ExecutionOutcome {
                t20_return: Some(t20_return),
                ..Default::default()
            },
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn bearish_analysis_counts_candidates() {
        let records = vec![
            make_record(-0.5, 0.55, -0.05, vec![EvidenceKind::Distribution]),
        ];
        let analysis = compute_bearish_analysis(&records);
        assert_eq!(analysis.bearish_candidates, 1);
        assert_eq!(analysis.baseline_negative_t20_rate, 1.0);
    }

    #[test]
    fn distribution_lift_is_computed() {
        let records = vec![
            make_record(-0.5, 0.55, -0.05, vec![EvidenceKind::Distribution]),
            make_record(-0.5, 0.55, 0.05, vec![EvidenceKind::RiskExpansion]),
        ];
        let analysis = compute_bearish_analysis(&records);
        let dist_lift = analysis
            .evidence_lift
            .iter()
            .find(|r| r.evidence == "Distribution")
            .expect("distribution lift");
        assert_eq!(dist_lift.count, 1);
        assert_eq!(dist_lift.negative_t20_rate, 1.0);
    }

    #[test]
    fn c3_false_reduce_detected() {
        let records = vec![
            // confidence 0.55 >= 0.45, bearish, positive outcome -> false reduce
            make_record(-0.5, 0.55, 0.05, vec![EvidenceKind::Distribution]),
        ];
        let analysis = compute_bearish_analysis(&records);
        assert_eq!(analysis.false_reduce_analysis.c3_reduce_count, 1);
        assert_eq!(analysis.false_reduce_analysis.false_reduce_count, 1);
    }
}
