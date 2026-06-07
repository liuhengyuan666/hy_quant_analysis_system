use chrono::NaiveDate;
use core_domain::{
    BreadthState, LiquidityState, RegimeCandidate, RegimeObservation, TrendState,
    VolatilityState,
};
use std::collections::HashMap;

// ------------------------------------------------------------------
// Regime Label Generator (Wave 7.2)
// Maps RegimeObservation → RegimeCandidate using explicit state-machine rules.
// Independent from macro-engine scoring; produces candidate labels only.
// ------------------------------------------------------------------

/// State-machine based label generator.
/// Supports both fixed-threshold and percentile-based state classification.
pub struct RegimeLabelGenerator;

#[derive(Debug, Clone)]
pub struct StateThresholds {
    pub trend: TrendThresholds,
    pub breadth: BreadthThresholds,
    pub liquidity: LiquidityThresholds,
    pub volatility: VolatilityThresholds,
}

#[derive(Debug, Clone, Default)]
pub struct TrendThresholds {
    pub strong_uptrend: f64,
    pub uptrend: f64,
    pub sideways: f64,
    pub downtrend: f64,
}

#[derive(Debug, Clone, Default)]
pub struct BreadthThresholds {
    pub expanding: f64,
    pub stable: f64,
    pub contracting: f64,
}

#[derive(Debug, Clone, Default)]
pub struct LiquidityThresholds {
    pub supportive: f64,
    pub neutral: f64,
    pub tightening: f64,
}

#[derive(Debug, Clone, Default)]
pub struct VolatilityThresholds {
    pub spike: f64,
    pub elevated: f64,
    pub normal: f64,
}

impl Default for StateThresholds {
    fn default() -> Self {
        Self {
            trend: TrendThresholds {
                strong_uptrend: 70.0,
                uptrend: 55.0,
                sideways: 45.0,
                downtrend: 30.0,
            },
            breadth: BreadthThresholds {
                expanding: 60.0,
                stable: 40.0,
                contracting: 20.0,
            },
            liquidity: LiquidityThresholds {
                supportive: 65.0,
                neutral: 45.0,
                tightening: 30.0,
            },
            volatility: VolatilityThresholds {
                spike: 70.0,
                elevated: 50.0,
                normal: 30.0,
            },
        }
    }
}

impl RegimeLabelGenerator {
    /// Classify a single observation into a regime candidate using default thresholds.
    pub fn classify(obs: &RegimeObservation) -> RegimeCandidate {
        Self::classify_with_thresholds(obs, &StateThresholds::default())
    }

    /// Classify with custom thresholds.
    pub fn classify_with_thresholds(obs: &RegimeObservation, thresholds: &StateThresholds) -> RegimeCandidate {
        let trend_state = Self::classify_trend_with_thresholds(obs.trend_strength, &thresholds.trend);
        let breadth_state = Self::classify_breadth_with_thresholds(obs.breadth_strength, &thresholds.breadth);
        let liquidity_state = Self::classify_liquidity_with_thresholds(obs.liquidity_strength, &thresholds.liquidity);
        let volatility_state = Self::classify_volatility_with_thresholds(obs.volatility_level, &thresholds.volatility);

        let trend_ok = matches!(trend_state, TrendState::StrongUptrend | TrendState::Uptrend);
        let breadth_ok = matches!(breadth_state, BreadthState::Expanding | BreadthState::Stable);
        let liquidity_ok = matches!(
            liquidity_state,
            LiquidityState::Supportive | LiquidityState::Neutral
        );
        let volatility_ok = matches!(
            volatility_state,
            VolatilityState::Low | VolatilityState::Normal
        );

        let risk_on = trend_ok && breadth_ok && liquidity_ok && volatility_ok;

        let trend_bad = matches!(
            trend_state,
            TrendState::StrongDowntrend | TrendState::Downtrend
        );
        let breadth_bad = matches!(breadth_state, BreadthState::Collapsed);
        let liquidity_bad = matches!(liquidity_state, LiquidityState::Critical);
        let volatility_bad = matches!(volatility_state, VolatilityState::Spike);

        let risk_off = trend_bad || breadth_bad || liquidity_bad || volatility_bad;

        if risk_on {
            RegimeCandidate::RiskOn
        } else if risk_off {
            RegimeCandidate::RiskOff
        } else {
            RegimeCandidate::Neutral
        }
    }

    /// Compute percentile-based thresholds from a population of observations.
    /// Ensures each state class has meaningful coverage by using data-driven cutoffs.
    pub fn compute_percentile_thresholds(observations: &[RegimeObservation]) -> StateThresholds {
        let mut trends: Vec<f64> = observations.iter().map(|o| o.trend_strength).collect();
        let mut breadths: Vec<f64> = observations.iter().map(|o| o.breadth_strength).collect();
        let mut liquidities: Vec<f64> = observations.iter().map(|o| o.liquidity_strength).collect();
        let mut volatilities: Vec<f64> = observations.iter().map(|o| o.volatility_level).collect();

        trends.sort_by(|a, b| a.partial_cmp(b).unwrap());
        breadths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        liquidities.sort_by(|a, b| a.partial_cmp(b).unwrap());
        volatilities.sort_by(|a, b| a.partial_cmp(b).unwrap());

        StateThresholds {
            trend: TrendThresholds {
                strong_uptrend: percentile(&trends, 0.80),
                uptrend: percentile(&trends, 0.60),
                sideways: percentile(&trends, 0.45),
                downtrend: percentile(&trends, 0.25),
            },
            breadth: BreadthThresholds {
                expanding: percentile(&breadths, 0.75),
                stable: percentile(&breadths, 0.50),
                contracting: percentile(&breadths, 0.25),
            },
            liquidity: LiquidityThresholds {
                supportive: percentile(&liquidities, 0.75),
                neutral: percentile(&liquidities, 0.50),
                tightening: percentile(&liquidities, 0.25),
            },
            volatility: VolatilityThresholds {
                spike: percentile(&volatilities, 0.80),
                elevated: percentile(&volatilities, 0.60),
                normal: percentile(&volatilities, 0.40),
            },
        }
    }

    /// Build RegimeObservation from raw scores.
    /// This is the Observation Layer: factual measurement before any classification.
    pub fn build_observation(
        date: NaiveDate,
        scope: &str,
        trend_score: f64,
        breadth_pct: f64,
        liquidity_score: f64,
        risk_score: f64,
    ) -> RegimeObservation {
        RegimeObservation {
            date,
            scope: scope.to_string(),
            trend_strength: trend_score,
            breadth_strength: breadth_pct,
            liquidity_strength: liquidity_score,
            volatility_level: risk_score,
            trend_state: Self::classify_trend(trend_score),
            breadth_state: Self::classify_breadth(breadth_pct),
            liquidity_state: Self::classify_liquidity(liquidity_score),
            volatility_state: Self::classify_volatility(risk_score),
        }
    }

    fn classify_trend(score: f64) -> TrendState {
        Self::classify_trend_with_thresholds(score, &TrendThresholds::default())
    }

    fn classify_trend_with_thresholds(score: f64, t: &TrendThresholds) -> TrendState {
        if score >= t.strong_uptrend {
            TrendState::StrongUptrend
        } else if score >= t.uptrend {
            TrendState::Uptrend
        } else if score >= t.sideways {
            TrendState::Sideways
        } else if score >= t.downtrend {
            TrendState::Downtrend
        } else {
            TrendState::StrongDowntrend
        }
    }

    fn classify_breadth(score: f64) -> BreadthState {
        Self::classify_breadth_with_thresholds(score, &BreadthThresholds::default())
    }

    fn classify_breadth_with_thresholds(score: f64, t: &BreadthThresholds) -> BreadthState {
        if score >= t.expanding {
            BreadthState::Expanding
        } else if score >= t.stable {
            BreadthState::Stable
        } else if score >= t.contracting {
            BreadthState::Contracting
        } else {
            BreadthState::Collapsed
        }
    }

    fn classify_liquidity(score: f64) -> LiquidityState {
        Self::classify_liquidity_with_thresholds(score, &LiquidityThresholds::default())
    }

    fn classify_liquidity_with_thresholds(score: f64, t: &LiquidityThresholds) -> LiquidityState {
        if score >= t.supportive {
            LiquidityState::Supportive
        } else if score >= t.neutral {
            LiquidityState::Neutral
        } else if score >= t.tightening {
            LiquidityState::Tightening
        } else {
            LiquidityState::Critical
        }
    }

    fn classify_volatility(score: f64) -> VolatilityState {
        Self::classify_volatility_with_thresholds(score, &VolatilityThresholds::default())
    }

    fn classify_volatility_with_thresholds(score: f64, t: &VolatilityThresholds) -> VolatilityState {
        if score >= t.spike {
            VolatilityState::Spike
        } else if score >= t.elevated {
            VolatilityState::Elevated
        } else if score >= t.normal {
            VolatilityState::Normal
        } else {
            VolatilityState::Low
        }
    }
}

// ------------------------------------------------------------------
// Persistence Filter (Wave 7.2-C)
// Enforces minimum regime duration and confirmation before transitions.
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Minimum days a regime must hold before any transition is allowed.
    pub min_days: usize,
    /// Consecutive days a candidate must persist before transition is confirmed.
    pub confirmation_days: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            min_days: 5,
            confirmation_days: 3,
        }
    }
}

/// Filters high-frequency candidate churn into stable regime labels.
pub struct PersistenceFilter {
    config: PersistenceConfig,
    thresholds: Option<StateThresholds>,
    current_regime: Option<String>,
    current_days: usize,
    candidate_regime: Option<String>,
    candidate_days: usize,
}

impl PersistenceFilter {
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            thresholds: None,
            current_regime: None,
            current_days: 0,
            candidate_regime: None,
            candidate_days: 0,
        }
    }

    pub fn with_thresholds(config: PersistenceConfig, thresholds: StateThresholds) -> Self {
        Self {
            config,
            thresholds: Some(thresholds),
            current_regime: None,
            current_days: 0,
            candidate_regime: None,
            candidate_days: 0,
        }
    }

    /// Feed a new candidate and receive the official regime (if any).
    ///
    /// Returns `Some(label)` when a stable regime is established or maintained.
    /// Returns `None` during the initial warm-up or when transitioning.
    pub fn update(&mut self, candidate: RegimeCandidate) -> Option<String> {
        let candidate_label = format!("{:?}", candidate).to_lowercase();

        // If no current regime, establish immediately
        if self.current_regime.is_none() {
            self.current_regime = Some(candidate_label.clone());
            self.current_days = 1;
            return Some(candidate_label);
        }

        let current = self.current_regime.as_ref().unwrap().clone();

        // If candidate matches current, extend current regime
        if candidate_label == current {
            self.current_days += 1;
            self.candidate_regime = None;
            self.candidate_days = 0;
            return Some(current);
        }

        // If this is a new candidate (different from current and previous candidate)
        if self.candidate_regime.as_ref() != Some(&candidate_label) {
            self.candidate_regime = Some(candidate_label.clone());
            self.candidate_days = 1;
            return Some(current);
        }

        // Same candidate as before, increment confirmation counter
        self.candidate_days += 1;

        // Check if we can transition:
        // 1. Current regime has lasted at least min_days
        // 2. Candidate has persisted for confirmation_days
        if self.current_days >= self.config.min_days
            && self.candidate_days >= self.config.confirmation_days
        {
            self.current_regime = Some(candidate_label.clone());
            self.current_days = 1;
            self.candidate_regime = None;
            self.candidate_days = 0;
            return Some(candidate_label);
        }

        // Not enough confirmation yet, stay in current regime
        self.current_days += 1;
        Some(current)
    }

    /// Batch process a sequence of observations and return stable regime labels.
    pub fn process_sequence(
        &mut self,
        observations: &[RegimeObservation],
    ) -> Vec<(NaiveDate, String)> {
        let mut results = Vec::new();
        for obs in observations {
            let candidate = if let Some(ref t) = self.thresholds {
                RegimeLabelGenerator::classify_with_thresholds(obs, t)
            } else {
                RegimeLabelGenerator::classify(obs)
            };
            if let Some(label) = self.update(candidate) {
                results.push((obs.date, label));
            }
        }
        results
    }
}

// ------------------------------------------------------------------
// Observation Builder from stored regime snapshots
// ------------------------------------------------------------------

/// Build a sequence of RegimeObservations from stored MarketRegimeSnapshot rows.
/// Falls back to default breadth/volatility when environment data is unavailable.
pub struct ObservationSequenceBuilder;

impl ObservationSequenceBuilder {
    pub fn from_regime_snapshots(
        regimes: &[core_domain::MarketRegimeSnapshot],
        environment_by_date: &HashMap<NaiveDate, core_domain::EnvironmentSnapshot>,
    ) -> Vec<RegimeObservation> {
        regimes
            .iter()
            .map(|r| {
                let env = environment_by_date.get(&r.date);
                let breadth_pct = env.map(|e| e.breadth_pct).unwrap_or(50.0);
                // Volatility proxy: use risk_score as a proxy until real volatility is available
                let volatility = r.risk_score;
                RegimeLabelGenerator::build_observation(
                    r.date,
                    &r.market,
                    r.trend_score,
                    breadth_pct,
                    r.liquidity_score,
                    volatility,
                )
            })
            .collect()
    }
}

// ------------------------------------------------------------------
// Percentile helper
// ------------------------------------------------------------------

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 50.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}
