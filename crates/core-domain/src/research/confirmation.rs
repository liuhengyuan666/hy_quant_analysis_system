//! Confirmation scoring for market evolution semantics.
//!
//! This module computes pure, stateless confirmation scores from raw market
//! observations. It returns normalized 0-100 scores; label mapping is a
//! consumer concern and is performed upstream by the orchestration layer.

use core::f64;

/// Inputs required to compute market confirmation scores.
///
/// All values are normalized to 0-100 where applicable. The orchestration
/// layer (app-service) is responsible for extracting these values from engine
/// outputs and snapshots.
#[derive(Debug, Clone, Copy)]
pub struct ConfirmationInputs {
    pub trend_score: f64,
    pub risk_score: f64,
    pub environment_score: f64,
    pub breadth_pct: f64,
    pub volume_expansion_pct: Option<f64>,
    pub turnover_coverage_pct: Option<f64>,
    pub leadership_stability: f64,
    pub rotation_broad: bool,
}

/// Normalized confirmation scores for the three primary market dimensions.
#[derive(Debug, Clone, Copy)]
pub struct ConfirmationScores {
    pub trend: f64,
    pub participation: f64,
    pub risk: f64,
    pub overall: f64,
}

impl ConfirmationScores {
    pub fn star_rating(score: f64) -> u8 {
        match score {
            s if s >= 80.0 => 5,
            s if s >= 60.0 => 4,
            s if s >= 40.0 => 3,
            s if s >= 20.0 => 2,
            _ => 1,
        }
    }

    pub fn label(score: f64) -> &'static str {
        match score {
            s if s >= 80.0 => "Very Strong",
            s if s >= 60.0 => "Strong",
            s if s >= 40.0 => "Moderate",
            s if s >= 20.0 => "Weak",
            _ => "Very Weak",
        }
    }
}

/// Compute confirmation scores from raw market observations.
///
/// Scoring rationale:
/// - Trend: 40% trend_score, 30% leadership_stability, 30% rotation_breadth
/// - Participation: 50% breadth_pct, 30% volume_expansion, 20% turnover_coverage
/// - Risk: 40% environment_score, 40% inverse risk_score, 20% volatility_proxy
///   (volatility_proxy uses breadth_pct as a proxy when no explicit vol data is available)
/// - Overall: 40% Trend, 35% Participation, 25% Risk
pub fn compute_confirmation(inputs: &ConfirmationInputs) -> ConfirmationScores {
    let trend = trend_confirmation(
        inputs.trend_score,
        inputs.leadership_stability,
        inputs.rotation_broad,
    );
    let participation = participation_confirmation(
        inputs.breadth_pct,
        inputs.volume_expansion_pct,
        inputs.turnover_coverage_pct,
    );
    let risk = risk_confirmation(
        inputs.environment_score,
        inputs.risk_score,
        inputs.breadth_pct,
    );

    let overall = trend * 0.40 + participation * 0.35 + risk * 0.25;

    ConfirmationScores {
        trend: clamp_100(trend),
        participation: clamp_100(participation),
        risk: clamp_100(risk),
        overall: clamp_100(overall),
    }
}

fn trend_confirmation(trend_score: f64, leadership_stability: f64, rotation_broad: bool) -> f64 {
    let rotation_breadth_score = if rotation_broad { 70.0 } else { 40.0 };
    trend_score * 0.40 + leadership_stability * 100.0 * 0.30 + rotation_breadth_score * 0.30
}

fn participation_confirmation(
    breadth_pct: f64,
    volume_expansion_pct: Option<f64>,
    turnover_coverage_pct: Option<f64>,
) -> f64 {
    let breadth = clamp_100(breadth_pct);
    let volume = volume_expansion_pct.unwrap_or(0.0).clamp(0.0, 100.0);
    let turnover = turnover_coverage_pct.unwrap_or(0.0).clamp(0.0, 100.0);

    breadth * 0.50 + volume * 0.30 + turnover * 0.20
}

fn risk_confirmation(environment_score: f64, risk_score: f64, breadth_pct: f64) -> f64 {
    // Higher risk_score means more risk, so confirmation is lower.
    let risk_inverse = (100.0 - clamp_100(risk_score)).max(0.0);
    // Volatility proxy: breadth collapse raises risk, breadth expansion lowers risk.
    let volatility_proxy = clamp_100(breadth_pct);

    environment_score * 0.40 + risk_inverse * 0.40 + volatility_proxy * 0.20
}

fn clamp_100(v: f64) -> f64 {
    v.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_inputs() -> ConfirmationInputs {
        ConfirmationInputs {
            trend_score: 50.0,
            risk_score: 50.0,
            environment_score: 50.0,
            breadth_pct: 50.0,
            volume_expansion_pct: Some(50.0),
            turnover_coverage_pct: Some(50.0),
            leadership_stability: 0.5,
            rotation_broad: true,
        }
    }

    #[test]
    fn confirmation_all_neutral() {
        let scores = compute_confirmation(&base_inputs());
        assert!(scores.trend > 50.0 && scores.trend < 65.0,
            "expected trend in (50,65), got {}", scores.trend);
        assert!(scores.participation > 45.0 && scores.participation < 55.0);
        assert!(scores.risk > 45.0 && scores.risk < 55.0);
        assert!(scores.overall > 45.0 && scores.overall < 55.0);
        assert_eq!(ConfirmationScores::label(scores.overall), "Moderate");
        assert_eq!(ConfirmationScores::star_rating(scores.overall), 3);
    }

    #[test]
    fn confirmation_strong_trend() {
        let mut inputs = base_inputs();
        inputs.trend_score = 90.0;
        inputs.leadership_stability = 0.9;
        inputs.rotation_broad = true;
        let scores = compute_confirmation(&inputs);
        assert!(scores.trend >= 80.0);
        assert_eq!(ConfirmationScores::label(scores.trend), "Very Strong");
    }

    #[test]
    fn confirmation_weak_participation() {
        let mut inputs = base_inputs();
        inputs.breadth_pct = 10.0;
        inputs.volume_expansion_pct = Some(0.0);
        inputs.turnover_coverage_pct = Some(0.0);
        let scores = compute_confirmation(&inputs);
        assert!(scores.participation < 20.0);
        assert_eq!(ConfirmationScores::label(scores.participation), "Very Weak");
    }

    #[test]
    fn confirmation_risk_inverse() {
        let mut inputs = base_inputs();
        inputs.risk_score = 90.0;
        inputs.environment_score = 20.0;
        inputs.breadth_pct = 10.0;
        let scores = compute_confirmation(&inputs);
        assert!(scores.risk < 30.0);
    }

    #[test]
    fn confirmation_clamping() {
        let mut inputs = base_inputs();
        inputs.trend_score = 150.0;
        inputs.breadth_pct = -20.0;
        let scores = compute_confirmation(&inputs);
        // score = 150*0.40 + 50*0.30 + 70*0.30 = 60+15+21 = 96, clamp to 96
        assert_eq!(scores.trend, 96.0);
        // participation: breadth clamped to 0, volume=50, turnover=50
        // 0*0.50 + 50*0.30 + 50*0.20 = 15+10 = 25
        assert!((scores.participation - 25.0).abs() < 0.01,
            "expected participation ~25, got {}", scores.participation);
    }
}
