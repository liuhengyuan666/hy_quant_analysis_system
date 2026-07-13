//! Research Calibration framework.
//!
//! Provides pure, stateless helpers to evaluate the behavior quality of the
//! Research Layer over a historical window. This is intended to be the
//! Research Layer's equivalent of a regression test: every time a research
//! semantic changes, re-run calibration to check for behavioral degradation.
//!
//! The framework is structured around four report chapters:
//!
//! 1. Behavior Distribution — do metrics have enough discrimination?
//! 2. Semantic Consistency — do outputs align with market intuition?
//! 3. Historical Evidence Quality — are analogue matches actually similar?
//! 4. Findings — summary of observations and recommended adjustments.

use chrono::NaiveDate;
use std::collections::BTreeMap;

/// Current Calibration Baseline Version.
///
/// Increment when the calibration methodology, report structure, or underlying
/// Evidence semantics change in a non-backwards-compatible way. Implementation
/// optimizations (e.g., bulk fetch) do NOT increment this version.
pub const CURRENT_CALIBRATION_BASELINE_VERSION: u32 = 1;

/// Daily confirmation observation extracted from ResearchContext.
#[derive(Debug, Clone)]
pub struct ConfirmationObservation {
    pub date: NaiveDate,
    pub overall: String,
    pub trend_score: f64,
    pub participation_score: f64,
    pub risk_score: f64,
    pub breadth_pct: f64,
}

/// Daily recovery observation extracted from ResearchContext.
#[derive(Debug, Clone)]
pub struct RecoveryObservation {
    pub date: NaiveDate,
    pub score: f64,
    pub label: String,
    pub drivers: Vec<String>,
    pub breadth_5d_delta: f64,
}

/// A single top historical match from an analogue search.
#[derive(Debug, Clone)]
pub struct MatchObservation {
    pub date: NaiveDate,
    pub level: String,
}

/// Daily analogue search observation.
#[derive(Debug, Clone)]
pub struct AnaloguesObservation {
    pub date: NaiveDate,
    pub searched_days: usize,
    pub filtered_days: usize,
    pub average_distance: f64,
    pub top_matches: Vec<MatchObservation>,
    pub outcome_median: Option<f64>,
    pub outcome_win_rate: Option<f64>,
    /// All distances from the target day to every searched historical day.
    /// Used to build the distance distribution histogram.
    pub all_distances: Vec<f64>,
}


/// Input to the calibration computation.
#[derive(Debug, Clone)]
pub struct CalibrationInput {
    pub scope: String,
    pub window_start: NaiveDate,
    pub window_end: NaiveDate,
    /// Trading days that are expected to have data within the window.
    pub expected_dates: Vec<NaiveDate>,
    /// Baseline version. Increment when the calibration methodology or
    /// report structure changes in a non-backwards-compatible way.
    pub baseline_version: u32,
    /// When this calibration report was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub confirmations: Vec<ConfirmationObservation>,
    pub recoveries: Vec<RecoveryObservation>,
    pub analogues: Vec<AnaloguesObservation>,
}


/// Distribution of confirmation levels over the window.
#[derive(Debug, Clone)]
pub struct ConfirmationDistribution {
    pub counts: BTreeMap<String, usize>,
    pub total: usize,
}

/// Distribution of recovery score buckets over the window.
#[derive(Debug, Clone)]
pub struct RecoveryDistribution {
    pub counts: BTreeMap<String, usize>,
    pub total: usize,
}

/// Distribution of analogue search metrics over the window.
#[derive(Debug, Clone)]
pub struct AnaloguesDistribution {
    pub average_distance: f64,
    pub std_distance: f64,
    pub top_match_level_counts: BTreeMap<String, usize>,
    pub total: usize,
}

/// Chapter 1: Behavior Distribution.
#[derive(Debug, Clone)]
pub struct BehaviorDistribution {
    pub confirmation: ConfirmationDistribution,
    pub recovery: RecoveryDistribution,
    pub analogues: AnaloguesDistribution,
}

/// A single semantic consistency flag.
#[derive(Debug, Clone)]
pub struct SemanticFlag {
    pub date: NaiveDate,
    pub category: String,
    pub description: String,
}

/// Chapter 2: Semantic Consistency.
#[derive(Debug, Clone)]
pub struct SemanticConsistency {
    pub flags: Vec<SemanticFlag>,
}

/// Chapter 3: Historical Evidence Quality.
#[derive(Debug, Clone)]
pub struct EvidenceQuality {
    pub average_distance: f64,
    pub std_distance: f64,
    pub median_top_matches_distance: f64,
    pub match_level_distribution: BTreeMap<String, usize>,
    pub distance_distribution: BTreeMap<String, usize>,
}

/// Coverage of each research tool over the window.
#[derive(Debug, Clone)]
pub struct Coverage {
    pub total_days: usize,
    pub confirmation_days: usize,
    pub recovery_days: usize,
    pub analogues_days: usize,
    pub confirmation_pct: f64,
    pub recovery_pct: f64,
    pub analogues_pct: f64,
}

/// Complete calibration report.
#[derive(Debug, Clone)]
pub struct CalibrationReport {
    pub scope: String,
    pub window_start: NaiveDate,
    pub window_end: NaiveDate,
    pub baseline_version: u32,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub behavior_distribution: BehaviorDistribution,
    pub semantic_consistency: SemanticConsistency,
    pub evidence_quality: EvidenceQuality,
    pub coverage: Coverage,
    pub findings: Vec<String>,
}

impl CalibrationReport {
    /// True if all three dimensions are available on at least 80% of days.
    pub fn is_coverage_acceptable(&self) -> bool {
        self.coverage.confirmation_pct >= 0.80
            && self.coverage.recovery_pct >= 0.80
            && self.coverage.analogues_pct >= 0.80
    }

    /// True if the most common confirmation level accounts for < 70% of days.
    pub fn has_confirmation_discrimination(&self) -> bool {
        let max_count = self.behavior_distribution.confirmation
            .counts
            .values()
            .max()
            .copied()
            .unwrap_or(0);
        let total = self.behavior_distribution.confirmation.total.max(1);
        (max_count as f64 / total as f64) < 0.70
    }

    /// True if the most common recovery bucket accounts for < 70% of days.
    pub fn has_recovery_discrimination(&self) -> bool {
        let max_count = self.behavior_distribution.recovery
            .counts
            .values()
            .max()
            .copied()
            .unwrap_or(0);
        let total = self.behavior_distribution.recovery.total.max(1);
        (max_count as f64 / total as f64) < 0.70
    }
}

/// Compute the full calibration report from daily observations.
pub fn calibrate(input: CalibrationInput) -> CalibrationReport {
    let behavior_distribution = BehaviorDistribution {
        confirmation: confirmation_distribution(&input.confirmations),
        recovery: recovery_distribution(&input.recoveries),
        analogues: analogues_distribution(&input.analogues),
    };

    let semantic_consistency = check_semantic_consistency(
        &input.confirmations,
        &input.recoveries,
        &input.analogues,
    );

    let evidence_quality = evidence_quality(&input.analogues);
    let coverage = coverage(
        &input.expected_dates,
        &input.confirmations,
        &input.recoveries,
        &input.analogues,
    );

    let findings = generate_findings(
        &behavior_distribution,
        &semantic_consistency,
        &evidence_quality,
        &coverage,
    );

    CalibrationReport {
        scope: input.scope,
        window_start: input.window_start,
        window_end: input.window_end,
        baseline_version: input.baseline_version,
        generated_at: input.generated_at,
        behavior_distribution,
        semantic_consistency,
        evidence_quality,
        coverage,
        findings,
    }
}

fn confirmation_distribution(obs: &[ConfirmationObservation]) -> ConfirmationDistribution {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for o in obs {
        *counts.entry(o.overall.clone()).or_insert(0) += 1;
    }
    // Ensure all expected levels are present for readable reports.
    for level in ["Very Weak", "Weak", "Moderate", "Strong", "Very Strong"] {
        counts.entry(level.to_string()).or_insert(0);
    }
    ConfirmationDistribution {
        total: obs.len(),
        counts,
    }
}

fn recovery_bucket(score: f64) -> String {
    match score {
        s if s >= 80.0 => "80-100".to_string(),
        s if s >= 60.0 => "60-80".to_string(),
        s if s >= 40.0 => "40-60".to_string(),
        s if s >= 20.0 => "20-40".to_string(),
        _ => "0-20".to_string(),
    }
}

fn recovery_distribution(obs: &[RecoveryObservation]) -> RecoveryDistribution {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for o in obs {
        *counts.entry(recovery_bucket(o.score)).or_insert(0) += 1;
    }
    for bucket in ["0-20", "20-40", "40-60", "60-80", "80-100"] {
        counts.entry(bucket.to_string()).or_insert(0);
    }
    RecoveryDistribution {
        total: obs.len(),
        counts,
    }
}

fn analogues_distribution(obs: &[AnaloguesObservation]) -> AnaloguesDistribution {
    let total = obs.len();
    let avg = if total > 0 {
        obs.iter().map(|o| o.average_distance).sum::<f64>() / total as f64
    } else {
        0.0
    };
    let std = if total > 1 {
        let variance = obs
            .iter()
            .map(|o| (o.average_distance - avg).powi(2))
            .sum::<f64>()
            / total as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let mut top_match_level_counts: BTreeMap<String, usize> = BTreeMap::new();
    for o in obs {
        if let Some(top) = o.top_matches.first() {
            *top_match_level_counts.entry(top.level.clone()).or_insert(0) += 1;
        }
    }
    for level in ["Very High", "High", "Moderate", "Weak"] {
        top_match_level_counts.entry(level.to_string()).or_insert(0);
    }

    AnaloguesDistribution {
        average_distance: avg,
        std_distance: std,
        top_match_level_counts,
        total,
    }
}

fn check_semantic_consistency(
    confirmations: &[ConfirmationObservation],
    recoveries: &[RecoveryObservation],
    analogues: &[AnaloguesObservation],
) -> SemanticConsistency {
    let mut flags = Vec::new();

    for c in confirmations {
        if c.overall == "Strong" || c.overall == "Very Strong" {
            if c.breadth_pct < 20.0 {
                flags.push(SemanticFlag {
                    date: c.date,
                    category: "Confirmation".to_string(),
                    description: format!(
                        "Confirmation = {} but breadth_pct = {:.1}%, suggesting weak participation",
                        c.overall, c.breadth_pct
                    ),
                });
            }
        }
        if c.overall == "Very Weak" && c.trend_score > 60.0 {
            flags.push(SemanticFlag {
                date: c.date,
                category: "Confirmation".to_string(),
                description: format!(
                    "Confirmation = Very Weak but trend_score = {:.1}, suggesting score/label mismatch",
                    c.trend_score
                ),
            });
        }
    }

    for r in recoveries {
        if r.score >= 80.0 && r.breadth_5d_delta < -5.0 {
            flags.push(SemanticFlag {
                date: r.date,
                category: "Recovery".to_string(),
                description: format!(
                    "Recovery score = {:.0} but breadth_5d_delta = {:.1}%, indicating deteriorating breadth",
                    r.score, r.breadth_5d_delta
                ),
            });
        }
        if r.score < 20.0 && r.drivers.contains(&"Price recovering".to_string()) {
            flags.push(SemanticFlag {
                date: r.date,
                category: "Recovery".to_string(),
                description: "Recovery score < 20 but driver says 'Price recovering'".to_string(),
            });
        }
    }

    for a in analogues {
        if let Some(outcome_median) = a.outcome_median {
            if let Some(top) = a.top_matches.first() {
                if top.level == "Very High" && outcome_median < -0.05 {
                    flags.push(SemanticFlag {
                        date: a.date,
                        category: "Analogues".to_string(),
                        description: format!(
                            "Top match = Very High but forward median return = {:+.1}%",
                            outcome_median * 100.0
                        ),
                    });
                }
            }
        }
    }

    SemanticConsistency { flags }
}

fn evidence_quality(obs: &[AnaloguesObservation]) -> EvidenceQuality {
    let distances: Vec<f64> = obs.iter().map(|o| o.average_distance).collect();
    let top_distances: Vec<f64> = obs
        .iter()
        .filter_map(|o| o.top_matches.first().map(|_| o.average_distance))
        .collect();
    let all_distances: Vec<f64> = obs.iter().flat_map(|o| o.all_distances.clone()).collect();

    let avg = mean(&distances);
    let std = std_dev(&distances, avg);
    let median = median(&top_distances);

    let mut match_level_distribution: BTreeMap<String, usize> = BTreeMap::new();
    for o in obs {
        for m in &o.top_matches {
            *match_level_distribution.entry(m.level.clone()).or_insert(0) += 1;
        }
    }
    for level in ["Very High", "High", "Moderate", "Weak"] {
        match_level_distribution.entry(level.to_string()).or_insert(0);
    }

    let mut distance_distribution: BTreeMap<String, usize> = BTreeMap::new();
    for d in &all_distances {
        *distance_distribution.entry(distance_bucket(*d)).or_insert(0) += 1;
    }
    for bucket in ["0.0-0.2", "0.2-0.4", "0.4-0.6", "0.6-0.8", "0.8-1.0"] {
        distance_distribution.entry(bucket.to_string()).or_insert(0);
    }

    EvidenceQuality {
        average_distance: avg,
        std_distance: std,
        median_top_matches_distance: median,
        match_level_distribution,
        distance_distribution,
    }
}

fn distance_bucket(dist: f64) -> String {
    match dist.clamp(0.0, 1.0) {
        d if d <= 0.2 => "0.0-0.2".to_string(),
        d if d <= 0.4 => "0.2-0.4".to_string(),
        d if d <= 0.6 => "0.4-0.6".to_string(),
        d if d <= 0.8 => "0.6-0.8".to_string(),
        _ => "0.8-1.0".to_string(),
    }
}

fn coverage(
    expected_dates: &[NaiveDate],
    confirmations: &[ConfirmationObservation],
    recoveries: &[RecoveryObservation],
    analogues: &[AnaloguesObservation],
) -> Coverage {
    let total_days = expected_dates.len();
    let confirmation_days = confirmations.len();
    let recovery_days = recoveries.len();
    let analogues_days = analogues.len();

    Coverage {
        total_days,
        confirmation_days,
        recovery_days,
        analogues_days,
        confirmation_pct: if total_days > 0 {
            confirmation_days as f64 / total_days as f64
        } else {
            0.0
        },
        recovery_pct: if total_days > 0 {
            recovery_days as f64 / total_days as f64
        } else {
            0.0
        },
        analogues_pct: if total_days > 0 {
            analogues_days as f64 / total_days as f64
        } else {
            0.0
        },
    }
}

fn generate_findings(
    behavior: &BehaviorDistribution,
    consistency: &SemanticConsistency,
    evidence: &EvidenceQuality,
    coverage: &Coverage,
) -> Vec<String> {
    let mut findings = Vec::new();

    // Coverage
    if coverage.confirmation_pct < 0.80 {
        findings.push(format!(
            "Confirmation coverage is {:.1}%, below 80% threshold — check missing data or gate dates.",
            coverage.confirmation_pct * 100.0
        ));
    }
    if coverage.recovery_pct < 0.80 {
        findings.push(format!(
            "Recovery coverage is {:.1}%, below 80% threshold — check missing data or gate dates.",
            coverage.recovery_pct * 100.0
        ));
    }
    if coverage.analogues_pct < 0.80 {
        findings.push(format!(
            "Analogues coverage is {:.1}%, below 80% threshold — check missing data or lookback window.",
            coverage.analogues_pct * 100.0
        ));
    }

    // Confirmation discrimination
    let max_confirmation = behavior.confirmation
        .counts
        .values()
        .max()
        .copied()
        .unwrap_or(0);
    if behavior.confirmation.total > 0 {
        let max_pct = max_confirmation as f64 / behavior.confirmation.total as f64;
        if max_pct >= 0.70 {
            findings.push(format!(
                "Confirmation is stuck: {:.1}% of days fall into one level. Consider revising weights or thresholds.",
                max_pct * 100.0
            ));
        }
    }

    // Recovery discrimination
    let max_recovery = behavior.recovery
        .counts
        .values()
        .max()
        .copied()
        .unwrap_or(0);
    if behavior.recovery.total > 0 {
        let max_pct = max_recovery as f64 / behavior.recovery.total as f64;
        if max_pct >= 0.70 {
            findings.push(format!(
                "Recovery is stuck: {:.1}% of days fall into one bucket. Consider revising weights or thresholds.",
                max_pct * 100.0
            ));
        }
    }

    // Analogues discrimination
    if behavior.analogues.total > 0 && evidence.std_distance < 0.05 {
        findings.push(format!(
            "Analogue distances are too homogeneous (std = {:.2}). Distance metric or feature weights may lack discrimination.",
            evidence.std_distance
        ));
    }
    if evidence.average_distance > 0.80 {
        findings.push(format!(
            "Average analogue distance is high ({:.2}), suggesting most historical days are dissimilar. Feature selection or normalization may need review.",
            evidence.average_distance
        ));
    }

    // Semantic consistency
    if !consistency.flags.is_empty() {
        findings.push(format!(
            "{} semantic consistency flags detected. Review flagged dates for score/label or score/market mismatch.",
            consistency.flags.len()
        ));
    }

    if findings.is_empty() {
        findings.push("No major findings. Research Layer appears behaviorally stable over the calibration window.".to_string());
    }

    findings
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance = values
        .iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Render a calibration report as markdown with the four standard chapters.
pub fn render_markdown(report: &CalibrationReport) -> String {
    let mut md = String::new();
    md.push_str("# Research Calibration Report\n\n");
    md.push_str(&format!("**Calibration Baseline Version:** {}\n\n", report.baseline_version));
    md.push_str(&format!(
        "**Generated At:** {}\n\n",
        report.generated_at.to_rfc3339()
    ));
    md.push_str(&format!("**Scope:** {}\n\n", report.scope));
    md.push_str(&format!(
        "**Window:** {} ~ {}\n\n",
        report.window_start, report.window_end
    ));

    md.push_str("---\n\n");

    // 1. Behavior Distribution
    md.push_str("## 1. Behavior Distribution\n\n");
    md.push_str("### Confirmation Level Distribution\n\n");
    md.push_str("| Level | Days | Percentage |\n");
    md.push_str("|-------|------|------------|\n");
    for (level, count) in &report.behavior_distribution.confirmation.counts {
        let pct = if report.behavior_distribution.confirmation.total > 0 {
            *count as f64 / report.behavior_distribution.confirmation.total as f64 * 100.0
        } else {
            0.0
        };
        md.push_str(&format!("| {} | {} | {:.1}% |\n", level, count, pct));
    }
    md.push('\n');

    md.push_str("### Recovery Score Distribution\n\n");
    md.push_str("| Bucket | Days | Percentage |\n");
    md.push_str("|--------|------|------------|\n");
    for (bucket, count) in &report.behavior_distribution.recovery.counts {
        let pct = if report.behavior_distribution.recovery.total > 0 {
            *count as f64 / report.behavior_distribution.recovery.total as f64 * 100.0
        } else {
            0.0
        };
        md.push_str(&format!("| {} | {} | {:.1}% |\n", bucket, count, pct));
    }
    md.push('\n');

    md.push_str("### Analogues Distribution\n\n");
    md.push_str(&format!(
        "- Average distance: {:.2}\n",
        report.behavior_distribution.analogues.average_distance
    ));
    md.push_str(&format!(
        "- Std distance: {:.2}\n",
        report.behavior_distribution.analogues.std_distance
    ));
    md.push_str("\n**Top Match Level Distribution**\n\n");
    md.push_str("| Level | Days | Percentage |\n");
    md.push_str("|-------|------|------------|\n");
    for (level, count) in &report.behavior_distribution.analogues.top_match_level_counts {
        let pct = if report.behavior_distribution.analogues.total > 0 {
            *count as f64 / report.behavior_distribution.analogues.total as f64 * 100.0
        } else {
            0.0
        };
        md.push_str(&format!("| {} | {} | {:.1}% |\n", level, count, pct));
    }
    md.push('\n');

    // 2. Semantic Consistency
    md.push_str("## 2. Semantic Consistency\n\n");
    if report.semantic_consistency.flags.is_empty() {
        md.push_str("No semantic consistency flags detected.\n\n");
    } else {
        md.push_str(&format!(
            "**{} flags detected**\n\n",
            report.semantic_consistency.flags.len()
        ));
        md.push_str("| Date | Category | Description |\n");
        md.push_str("|------|----------|-------------|\n");
        for flag in &report.semantic_consistency.flags {
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                flag.date, flag.category, flag.description
            ));
        }
        md.push('\n');
    }

    // 3. Historical Evidence Quality
    md.push_str("## 3. Historical Evidence Quality\n\n");
    md.push_str(&format!(
        "- Average distance across all searched days: {:.2}\n",
        report.evidence_quality.average_distance
    ));
    md.push_str(&format!(
        "- Std of distance: {:.2}\n",
        report.evidence_quality.std_distance
    ));
    md.push_str(&format!(
        "- Median top-match distance: {:.2}\n",
        report.evidence_quality.median_top_matches_distance
    ));
    md.push_str("\n**Match Level Distribution (all top matches)**\n\n");
    md.push_str("| Level | Occurrences | Percentage |\n");
    md.push_str("|-------|-------------|------------|\n");
    let total_matches: usize = report.evidence_quality.match_level_distribution.values().sum();
    for (level, count) in &report.evidence_quality.match_level_distribution {
        let pct = if total_matches > 0 {
            *count as f64 / total_matches as f64 * 100.0
        } else {
            0.0
        };
        md.push_str(&format!("| {} | {} | {:.1}% |\n", level, count, pct));
    }
    md.push('\n');

    md.push_str("\n**Distance Distribution (all target-vs-historical pairs)**\n\n");
    md.push_str("| Distance Bucket | Pairs | Percentage |\n");
    md.push_str("|-----------------|-------|------------|\n");
    let total_pairs: usize = report.evidence_quality.distance_distribution.values().sum();
    for (bucket, count) in &report.evidence_quality.distance_distribution {
        let pct = if total_pairs > 0 {
            *count as f64 / total_pairs as f64 * 100.0
        } else {
            0.0
        };
        md.push_str(&format!(
            "| {} | {} | {:.1}% |\n",
            bucket, count, pct
        ));
    }
    md.push('\n');

    // 4. Findings
    md.push_str("## 4. Findings & Recommended Adjustments\n\n");
    for finding in &report.findings {
        md.push_str(&format!("- {}\n", finding));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_confirmation(level: &str) -> ConfirmationObservation {
        ConfirmationObservation {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            overall: level.to_string(),
            trend_score: 50.0,
            participation_score: 50.0,
            risk_score: 50.0,
            breadth_pct: 50.0,
        }
    }

    fn make_recovery(score: f64) -> RecoveryObservation {
        RecoveryObservation {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            score,
            label: recovery_bucket(score),
            drivers: Vec::new(),
            breadth_5d_delta: 0.0,
        }
    }

    fn make_analogues(avg_distance: f64) -> AnaloguesObservation {
        AnaloguesObservation {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            searched_days: 100,
            filtered_days: 10,
            average_distance: avg_distance,
            top_matches: vec![MatchObservation {
                date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
                level: "High".to_string(),
            }],
            outcome_median: Some(0.02),
            outcome_win_rate: Some(0.6),
            all_distances: vec![avg_distance],
        }
    }

    #[test]
    fn confirmation_distribution_counts_all_levels() {
        let obs = vec![
            make_confirmation("Strong"),
            make_confirmation("Strong"),
            make_confirmation("Weak"),
        ];
        let dist = confirmation_distribution(&obs);
        assert_eq!(dist.counts.get("Strong").copied(), Some(2));
        assert_eq!(dist.counts.get("Weak").copied(), Some(1));
        assert_eq!(dist.counts.get("Very Strong").copied(), Some(0));
    }

    #[test]
    fn recovery_distribution_buckets_correctly() {
        let obs = vec![
            make_recovery(85.0),
            make_recovery(45.0),
            make_recovery(15.0),
        ];
        let dist = recovery_distribution(&obs);
        assert_eq!(dist.counts.get("80-100").copied(), Some(1));
        assert_eq!(dist.counts.get("40-60").copied(), Some(1));
        assert_eq!(dist.counts.get("0-20").copied(), Some(1));
    }

    #[test]
    fn calibration_generates_findings() {
        let expected_dates = vec![
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
        ];
        let input = CalibrationInput {
            scope: "GLOBAL".to_string(),
            window_start: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            window_end: NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
            expected_dates: expected_dates.clone(),
            baseline_version: 1,
            generated_at: chrono::Utc::now(),
            confirmations: vec![
                make_confirmation("Strong"),
                make_confirmation("Strong"),
                make_confirmation("Strong"),
            ],
            recoveries: vec![
                make_recovery(50.0),
                make_recovery(50.0),
                make_recovery(50.0),
            ],
            analogues: vec![
                make_analogues(0.5),
                make_analogues(0.5),
                make_analogues(0.5),
            ],
        };
        let report = calibrate(input);
        assert!(!report.has_confirmation_discrimination());
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn markdown_contains_four_chapters() {
        let expected_dates = vec![
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
        ];
        let input = CalibrationInput {
            scope: "GLOBAL".to_string(),
            window_start: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            window_end: NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
            expected_dates,
            baseline_version: 1,
            generated_at: chrono::Utc::now(),
            confirmations: Vec::new(),
            recoveries: Vec::new(),
            analogues: Vec::new(),
        };
        let report = calibrate(input);
        let md = render_markdown(&report);
        assert!(md.contains("## 1. Behavior Distribution"));
        assert!(md.contains("## 2. Semantic Consistency"));
        assert!(md.contains("## 3. Historical Evidence Quality"));
        assert!(md.contains("## 4. Findings"));
    }
}
