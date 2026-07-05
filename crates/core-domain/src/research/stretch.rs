/// Stretch aggregation helpers.

fn level_to_score(level: &str) -> f64 {
    match level {
        "Extreme" => 2.0,
        "Elevated" => 1.0,
        _ => 0.0,
    }
}

/// Weighted Stretch Overall: Momentum 40%, Crowding 30%, Breadth 20%, Leverage 10%.
pub fn weighted_stretch_overall(
    crowding: &str,
    breadth: &str,
    momentum: &str,
    leverage: &str,
) -> (&'static str, f64) {
    let score = level_to_score(momentum) * 0.40
        + level_to_score(crowding) * 0.30
        + level_to_score(breadth) * 0.20
        + level_to_score(leverage) * 0.10;
    let level = if score >= 1.2 {
        "Extreme"
    } else if score >= 0.5 {
        "Elevated"
    } else {
        "Normal"
    };
    (level, score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_stretch_overall_all_normal() {
        let (level, score) = weighted_stretch_overall("Normal", "Normal", "Normal", "Normal");
        assert_eq!(level, "Normal");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn weighted_stretch_overall_extreme_momentum_only() {
        // Momentum Extreme contributes 0.40 * 2.0 = 0.80 -> Elevated threshold 0.5
        let (level, score) = weighted_stretch_overall("Normal", "Normal", "Extreme", "Normal");
        assert_eq!(level, "Elevated");
        assert!((score - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn weighted_stretch_overall_all_extreme() {
        // 2.0 * (0.4 + 0.3 + 0.2 + 0.1) = 2.0
        let (level, score) = weighted_stretch_overall("Extreme", "Extreme", "Extreme", "Extreme");
        assert_eq!(level, "Extreme");
        assert!((score - 2.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_stretch_overall_extreme_crowding_and_momentum() {
        // 0.3*2 + 0.4*2 = 1.4 -> Extreme threshold 1.2
        let (level, score) = weighted_stretch_overall("Extreme", "Normal", "Extreme", "Normal");
        assert_eq!(level, "Extreme");
        assert!((score - 1.4).abs() < 1e-9);
    }
}
