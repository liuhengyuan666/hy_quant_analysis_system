use std::collections::HashMap;

use super::labeler::{compare_prediction, GroundTruthRegime, LabeledRegime, PredictionOutcome};

/// Result of validating skill predictions against ground truth.
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub total: usize,
    pub correct: usize,
    pub per_regime: HashMap<GroundTruthRegime, RegimeMetrics>,
    pub outcomes: Vec<PredictionOutcome>,
}

#[derive(Debug, Clone, Default)]
pub struct RegimeMetrics {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

/// Historical validator: compare skill outputs to ground truth labels.
pub struct HistoricalValidator;

impl HistoricalValidator {
    /// Validate predictions against ground truth.
    ///
    /// `predictions` is a map of (date, symbol) -> predicted regime string.
    pub fn validate(
        ground_truth: &[LabeledRegime],
        predictions: &HashMap<(NaiveDate, String), String>,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();

        for gt in ground_truth {
            let key = (gt.date, gt.symbol.clone());
            if let Some(pred) = predictions.get(&key) {
                let outcome = compare_prediction(pred, gt.regime);
                result.total += 1;
                if matches!(outcome, PredictionOutcome::TruePositive(_)) {
                    result.correct += 1;
                }
                result.outcomes.push(outcome);

                // Per-regime metrics
                let metrics = result.per_regime.entry(gt.regime).or_default();
                match outcome {
                    PredictionOutcome::TruePositive(_) => metrics.true_positives += 1,
                    PredictionOutcome::FalsePositive { predicted, actual } => {
                        metrics.false_negatives += 1;
                        // Also increment false_positives for the *predicted* regime
                        let pred_metrics = result.per_regime.entry(predicted).or_default();
                        pred_metrics.false_positives += 1;
                        // Restore actual metrics reference
                        let _ = result.per_regime.entry(actual).or_default();
                    }
                    PredictionOutcome::ParseFailure(_) => {
                        metrics.false_negatives += 1;
                    }
                }
            }
        }

        result
    }
}

use chrono::NaiveDate;
