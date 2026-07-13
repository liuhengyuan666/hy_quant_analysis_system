//! Rotation evolution metrics for market semantics.
//!
//! This module computes pure, stateless metrics describing how leadership and
//! theme concentration are evolving. It requires historical rotation ranks as
//! input and returns raw scores / enum classifications that the orchestration
//! layer can convert into human-readable labels.

use std::collections::HashSet;

/// A minimal rotation item used as input to evolution computations.
#[derive(Debug, Clone)]
pub struct RotationItemInput {
    pub symbol: String,
    pub momentum_score: f64,
    pub rank: u32,
}

/// Leadership transition classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadershipTransition {
    /// Top leaders unchanged from previous period.
    Stable,
    /// Some top leaders changed.
    Partial,
    /// Most or all top leaders changed.
    FullRotation,
}

impl LeadershipTransition {
    pub fn as_str(&self) -> &'static str {
        match self {
            LeadershipTransition::Stable => "Stable",
            LeadershipTransition::Partial => "Partial Rotation",
            LeadershipTransition::FullRotation => "Full Rotation",
        }
    }
}

/// Compute leadership transition by comparing current top-N symbols with
/// previous top-N symbols.
///
/// `overlap_ratio` is the number of shared symbols divided by N. The function
/// returns `Stable` if overlap >= 0.8, `FullRotation` if overlap <= 0.2, and
/// `Partial` otherwise.
pub fn leadership_transition(
    current_top: &[RotationItemInput],
    previous_top: &[RotationItemInput],
) -> LeadershipTransition {
    let n = current_top.len().min(previous_top.len()).max(1);
    let current_set: HashSet<&str> = current_top.iter().map(|r| r.symbol.as_str()).collect();
    let previous_set: HashSet<&str> = previous_top.iter().map(|r| r.symbol.as_str()).collect();

    let overlap = current_set.intersection(&previous_set).count();
    let ratio = overlap as f64 / n as f64;

    if ratio >= 0.8 {
        LeadershipTransition::Stable
    } else if ratio <= 0.2 {
        LeadershipTransition::FullRotation
    } else {
        LeadershipTransition::Partial
    }
}

/// Compute rotation acceleration as the ratio of current top-5 average momentum
/// to the recent historical top-5 average momentum.
///
/// Returns None if there is insufficient historical data.
/// A value > 1.0 means leadership momentum is accelerating; < 1.0 means decelerating.
pub fn rotation_acceleration(
    current: &[RotationItemInput],
    recent_history: &[Vec<RotationItemInput>],
) -> Option<f64> {
    let current_avg = top_n_average(current, 5)?;
    let historical_avg: f64 = recent_history
        .iter()
        .filter_map(|day| top_n_average(day, 5))
        .sum::<f64>() / recent_history.len() as f64;

    if historical_avg == 0.0 {
        return None;
    }
    Some(current_avg / historical_avg)
}

/// Compute theme dispersion as the normalized standard deviation of momentum
/// scores across the current universe.
///
/// Higher dispersion means leadership is more concentrated in a few outliers.
/// Lower dispersion means performance is more uniform.
/// Returns None if there are fewer than 2 symbols.
pub fn theme_dispersion(rotation: &[RotationItemInput]) -> Option<f64> {
    if rotation.len() < 2 {
        return None;
    }
    let scores: Vec<f64> = rotation.iter().map(|r| r.momentum_score).collect();
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance = scores
        .iter()
        .map(|s| (s - mean).powi(2))
        .sum::<f64>() / scores.len() as f64;
    let std_dev = variance.sqrt();

    // Normalize by mean to get coefficient of variation, scaled to 0-100.
    if mean == 0.0 {
        return None;
    }
    let cv = std_dev / mean.abs();
    Some((cv * 100.0).clamp(0.0, 100.0))
}

fn top_n_average(rotation: &[RotationItemInput], n: usize) -> Option<f64> {
    if rotation.is_empty() {
        return None;
    }
    let mut sorted = rotation.to_vec();
    sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
    let top: Vec<&RotationItemInput> = sorted.iter().take(n).collect();
    if top.is_empty() {
        return None;
    }
    Some(top.iter().map(|r| r.momentum_score).sum::<f64>() / top.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(symbol: &str, momentum: f64, rank: u32) -> RotationItemInput {
        RotationItemInput {
            symbol: symbol.to_string(),
            momentum_score: momentum,
            rank,
        }
    }

    #[test]
    fn leadership_transition_stable() {
        let current = vec![item("A", 10.0, 1), item("B", 9.0, 2), item("C", 8.0, 3)];
        let previous = vec![item("A", 10.0, 1), item("B", 9.0, 2), item("C", 8.0, 3)];
        assert_eq!(
            leadership_transition(&current, &previous),
            LeadershipTransition::Stable
        );
    }

    #[test]
    fn leadership_transition_full_rotation() {
        let current = vec![item("A", 10.0, 1), item("B", 9.0, 2), item("C", 8.0, 3)];
        let previous = vec![item("D", 10.0, 1), item("E", 9.0, 2), item("F", 8.0, 3)];
        assert_eq!(
            leadership_transition(&current, &previous),
            LeadershipTransition::FullRotation
        );
    }

    #[test]
    fn leadership_transition_partial() {
        let current = vec![item("A", 10.0, 1), item("B", 9.0, 2), item("C", 8.0, 3)];
        let previous = vec![item("A", 10.0, 1), item("D", 9.0, 2), item("C", 8.0, 3)];
        assert_eq!(
            leadership_transition(&current, &previous),
            LeadershipTransition::Partial
        );
    }

    #[test]
    fn rotation_acceleration_basic() {
        let current = vec![item("A", 12.0, 1), item("B", 10.0, 2)];
        let history = vec![
            vec![item("A", 10.0, 1), item("B", 8.0, 2)],
            vec![item("A", 10.0, 1), item("B", 8.0, 2)],
        ];
        let accel = rotation_acceleration(&current, &history).unwrap();
        assert!(accel > 1.0);
    }

    #[test]
    fn theme_dispersion_uniform() {
        let rotation = vec![
            item("A", 10.0, 1),
            item("B", 10.0, 2),
            item("C", 10.0, 3),
        ];
        let dispersion = theme_dispersion(&rotation).unwrap();
        assert!(dispersion < 1.0);
    }

    #[test]
    fn theme_dispersion_concentrated() {
        let rotation = vec![
            item("A", 100.0, 1),
            item("B", 10.0, 2),
            item("C", 5.0, 3),
        ];
        let dispersion = theme_dispersion(&rotation).unwrap();
        assert!(dispersion > 50.0);
    }
}
