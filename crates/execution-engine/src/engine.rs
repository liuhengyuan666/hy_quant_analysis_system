use crate::types::{ExecutionDecision, ExecutionState, IntradaySnapshot, ReasonTag};

// ─────────────────────────────────────────────────────────────
// 3 Pattern functions (Phase1)
// ─────────────────────────────────────────────────────────────

/// ① NoChase: gap up overextended.
fn check_no_chase(s: &IntradaySnapshot) -> Option<Vec<ReasonTag>> {
    if s.today_return > 0.02 && s.distance_ma5 > 0.04 && s.volume_ratio > 1.8 {
        Some(vec![
            ReasonTag::GapUpOverextended,
            ReasonTag::VolumeSpike,
            ReasonTag::FarFromMA5,
        ])
    } else {
        None
    }
}

/// ② Distribution: heavy selling on volume.
fn check_distribution(s: &IntradaySnapshot) -> Option<Vec<ReasonTag>> {
    if s.today_return < -0.015 && s.volume_ratio > 1.5 && s.close_position < 0.2 {
        Some(vec![
            ReasonTag::DistributionDay,
            ReasonTag::VolumeSurgeDecline,
        ])
    } else {
        None
    }
}

/// ③ StrongClose: close at daily high on volume.
fn check_strong_close(s: &IntradaySnapshot) -> Option<Vec<ReasonTag>> {
    if s.today_return > 0.01 && s.close_position > 0.8 && s.volume_ratio > 1.3 {
        Some(vec![
            ReasonTag::StrongClose,
            ReasonTag::HighVolume,
        ])
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────
// Engine entry
// ─────────────────────────────────────────────────────────────

/// Analyze a single symbol's intraday snapshot.
pub fn analyze(symbol: impl Into<String>, snapshot: &IntradaySnapshot) -> ExecutionDecision {
    let symbol = symbol.into();

    // Order matters: checked in priority order.
    if let Some(reasons) = check_no_chase(snapshot) {
        return ExecutionDecision {
            symbol,
            state: ExecutionState::Avoid,
            reasons,
        };
    }
    if let Some(reasons) = check_distribution(snapshot) {
        return ExecutionDecision {
            symbol,
            state: ExecutionState::Reduce,
            reasons,
        };
    }
    if let Some(reasons) = check_strong_close(snapshot) {
        return ExecutionDecision {
            symbol,
            state: ExecutionState::Increase,
            reasons,
        };
    }

    // Default: no pattern matched
    ExecutionDecision {
        symbol,
        state: ExecutionState::Maintain,
        reasons: vec![],
    }
}

/// Batch analyze multiple symbols.
pub fn analyze_batch(
    snapshots: &std::collections::HashMap<String, IntradaySnapshot>,
) -> Vec<ExecutionDecision> {
    let mut decisions: Vec<ExecutionDecision> = snapshots
        .iter()
        .map(|(symbol, snapshot)| analyze(symbol.clone(), snapshot))
        .collect();
    // Sort by symbol for deterministic output
    decisions.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    decisions
}
