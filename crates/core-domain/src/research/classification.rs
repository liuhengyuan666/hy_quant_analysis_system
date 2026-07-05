//! Classification helpers for research metrics.

/// Classify a numeric value into one of three levels.
///
/// When `higher_is_more_extreme` is `true`, larger values are more extreme
/// (e.g. crowding, momentum).  When `false`, smaller values are more extreme
/// (e.g. breadth).
pub fn classify_level(
    value: f64,
    elevated_threshold: f64,
    extreme_threshold: f64,
    higher_is_more_extreme: bool,
) -> &'static str {
    if higher_is_more_extreme {
        if value >= extreme_threshold {
            "Extreme"
        } else if value >= elevated_threshold {
            "Elevated"
        } else {
            "Normal"
        }
    } else if value <= extreme_threshold {
        "Extreme"
    } else if value <= elevated_threshold {
        "Elevated"
    } else {
        "Normal"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_level_higher_is_more_extreme() {
        assert_eq!(classify_level(0.0, 70.0, 85.0, true), "Normal");
        assert_eq!(classify_level(70.0, 70.0, 85.0, true), "Elevated");
        assert_eq!(classify_level(75.0, 70.0, 85.0, true), "Elevated");
        assert_eq!(classify_level(85.0, 70.0, 85.0, true), "Extreme");
        assert_eq!(classify_level(95.0, 70.0, 85.0, true), "Extreme");
    }

    #[test]
    fn classify_level_lower_is_more_extreme() {
        assert_eq!(classify_level(50.0, 35.0, 20.0, false), "Normal");
        assert_eq!(classify_level(35.0, 35.0, 20.0, false), "Elevated");
        assert_eq!(classify_level(25.0, 35.0, 20.0, false), "Elevated");
        assert_eq!(classify_level(20.0, 35.0, 20.0, false), "Extreme");
        assert_eq!(classify_level(10.0, 35.0, 20.0, false), "Extreme");
    }
}
