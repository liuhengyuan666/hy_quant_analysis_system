// ============================================================
// Common audit utilities
// Extracted from multiple audit modules to eliminate duplication
// while preserving standalone reproducibility per module.
// ============================================================

/// Apply persistence smoothing to a sequence of raw regime labels.
///
/// A regime label only becomes "active" after `days` consecutive occurrences.
/// Until then, the previous persisted label is carried forward.
/// This is the core mechanism tested across all Wave 7/8 audit modules.
pub fn apply_persistence(raw_labels: &[String], days: usize) -> Vec<String> {
    if days == 0 {
        return raw_labels.to_vec();
    }
    let mut persisted = Vec::with_capacity(raw_labels.len());
    let mut current_regime = "neutral".to_string();
    let mut streak = 0;

    for label in raw_labels {
        if label == &current_regime {
            streak += 1;
        } else {
            streak = 1;
            current_regime = label.clone();
        }

        if streak >= days {
            persisted.push(current_regime.clone());
        } else {
            if persisted.is_empty() {
                persisted.push("neutral".to_string());
            } else {
                persisted.push(persisted.last().unwrap().clone());
            }
        }
    }

    persisted
}
