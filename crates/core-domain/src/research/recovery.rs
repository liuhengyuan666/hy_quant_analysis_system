//! Recovery index computation for market evolution semantics.
//!
//! This module computes a pure, stateless 0-100 recovery index from raw market
//! observations. Higher values indicate a market that is recovering from a
//! drawdown or stressed state. The orchestration layer derives human-readable
//! drivers from the same inputs.

use core::f64;

/// Inputs required to compute the recovery index.
///
/// All percentage values are expected in raw units (e.g. 0.15 for 15%).
/// The orchestration layer is responsible for extracting these values from
/// engine outputs and snapshots.
#[derive(Debug, Clone, Copy)]
pub struct RecoveryInputs {
    /// Current drawdown from a recent peak, as a positive ratio (0.20 = 20% drawdown).
    pub drawdown_pct: f64,
    /// 5-day change in market breadth, in percentage points (e.g. 5.0 = +5 pp).
    pub breadth_5d_delta: f64,
    /// Current realized volatility, annualized or normalized as needed by caller.
    pub realized_vol: f64,
    /// 20-day average realized volatility, same units as `realized_vol`.
    pub vol_20d_avg: f64,
    /// Price recovery from recent low, as a positive ratio (0.05 = +5%).
    pub price_recovery_pct: f64,
}

/// Compute a 0-100 recovery index.
///
/// Scoring rationale:
/// - 40% drawdown recovery: shallower drawdowns score higher.
/// - 25% breadth improvement: positive breadth deltas score higher.
/// - 20% volatility contraction: falling volatility scores higher.
/// - 15% price recovery from recent lows: more recovery scores higher.
pub fn compute_recovery_index(inputs: &RecoveryInputs) -> f64 {
    let drawdown_score = drawdown_recovery_score(inputs.drawdown_pct);
    let breadth_score = breadth_improvement_score(inputs.breadth_5d_delta);
    let vol_score = volatility_contraction_score(inputs.realized_vol, inputs.vol_20d_avg);
    let price_score = price_recovery_score(inputs.price_recovery_pct);

    let score = drawdown_score * 0.40
        + breadth_score * 0.25
        + vol_score * 0.20
        + price_score * 0.15;

    score.clamp(0.0, 100.0)
}

/// Driver predicates that the orchestration layer can use to generate
/// human-readable `RecoverySummary.drivers`.
pub fn breadth_improving(delta: f64) -> bool {
    delta > 5.0
}

pub fn volatility_contracting(vol: f64, avg: f64) -> bool {
    avg > 0.0 && vol < avg * 0.90
}

pub fn drawdown_recovering(drawdown_pct: f64) -> bool {
    drawdown_pct < 0.10
}

pub fn price_recovering(recovery_pct: f64) -> bool {
    recovery_pct > 0.0
}

fn drawdown_recovery_score(drawdown_pct: f64) -> f64 {
    // 0% drawdown -> 100; 10% -> 80; 20% -> 60; 30% -> 40; 40%+ -> 20
    let score = 100.0 - drawdown_pct * 200.0;
    score.clamp(20.0, 100.0)
}

fn breadth_improvement_score(delta: f64) -> f64 {
    // Center at 0; +10 pp -> 100; -10 pp -> 0
    let score = 50.0 + delta * 5.0;
    score.clamp(0.0, 100.0)
}

fn volatility_contraction_score(vol: f64, avg: f64) -> f64 {
    if avg <= 0.0 {
        return 50.0;
    }
    let ratio = vol / avg;
    // ratio 0.7 -> 100; 1.0 -> 50; 1.3 -> 0
    let score = 100.0 - (ratio - 0.7) / 0.6 * 100.0;
    score.clamp(0.0, 100.0)
}

fn price_recovery_score(recovery_pct: f64) -> f64 {
    // 0% -> 0; 5% -> 50; 10% -> 100
    let score = recovery_pct * 1000.0;
    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_inputs() -> RecoveryInputs {
        RecoveryInputs {
            drawdown_pct: 0.10,
            breadth_5d_delta: 0.0,
            realized_vol: 0.20,
            vol_20d_avg: 0.20,
            price_recovery_pct: 0.0,
        }
    }

    #[test]
    fn recovery_all_neutral() {
        let score = compute_recovery_index(&base_inputs());
        // drawdown=0.10→80*0.40=32, breadth=0→50*0.25=12.5, vol_ratio=1.0→50*0.20=10, recovery=0→0*0.15=0
        // total = 54.5
        assert!(score > 50.0 && score < 60.0,
            "expected score in (50,60), got {}", score);
    }

    #[test]
    fn recovery_strong() {
        let inputs = RecoveryInputs {
            drawdown_pct: 0.02,
            breadth_5d_delta: 12.0,
            realized_vol: 0.10,
            vol_20d_avg: 0.20,
            price_recovery_pct: 0.08,
        };
        let score = compute_recovery_index(&inputs);
        assert!(score >= 80.0);
    }

    #[test]
    fn recovery_weak() {
        let inputs = RecoveryInputs {
            drawdown_pct: 0.40,
            breadth_5d_delta: -12.0,
            realized_vol: 0.35,
            vol_20d_avg: 0.20,
            price_recovery_pct: 0.0,
        };
        let score = compute_recovery_index(&inputs);
        assert!(score < 30.0);
    }

    #[test]
    fn recovery_clamping() {
        let inputs = RecoveryInputs {
            drawdown_pct: 0.0,
            breadth_5d_delta: 100.0,
            realized_vol: 0.0,
            vol_20d_avg: 0.20,
            price_recovery_pct: 1.0,
        };
        let score = compute_recovery_index(&inputs);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn driver_predicates() {
        assert!(breadth_improving(6.0));
        assert!(!breadth_improving(4.0));
        assert!(volatility_contracting(0.15, 0.20));
        assert!(!volatility_contracting(0.20, 0.20));
        assert!(drawdown_recovering(0.05));
        assert!(!drawdown_recovering(0.15));
        assert!(price_recovering(0.01));
        assert!(!price_recovering(-0.01));
    }
}
