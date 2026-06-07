use chrono::NaiveDate;
use market_state_extractor::{
    MarketStateObservation, TrendDirection, VolatilityRegime,
};

// ============================================================
// GT Regime Generator (Wave 7.3C)
// Four-layer pipeline:
//   Observation → Candidate → Persistence → Regime Label
// ============================================================

// ------------------------------------------------------------------
// Layer 1: Candidate (what does today look like?)
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegimeCandidate {
    RiskOn,
    Neutral,
    RiskOff,
}

// ------------------------------------------------------------------
// Layer 3: Stable Regime (after persistence filtering)
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    RiskOn,
    Neutral,
    RiskOff,
}

// ------------------------------------------------------------------
// Layer 4: Rich Label (for audit and downstream consumption)
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RegimeLabel {
    /// The stable regime after persistence filtering
    pub regime: Regime,
    /// The raw candidate for this day (before persistence)
    pub candidate: RegimeCandidate,
    /// Confidence score: 0-100
    pub confidence: f64,
    /// How many consecutive days the current regime has held
    pub days_in_regime: usize,
    /// Date of this observation
    pub date: NaiveDate,
    /// Scope label
    pub scope: String,
}

// ------------------------------------------------------------------
// Layer 2: Candidate Generator (pure function, no history)
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateVariant {
    /// Original asymmetric scoring (RiskOn=AND, RiskOff=OR)
    Baseline,
    /// Symmetric scoring (both use weighted average)
    Symmetric,
}

impl Default for CandidateVariant {
    fn default() -> Self {
        CandidateVariant::Baseline
    }
}

// ------------------------------------------------------------------
// Scope-Aware Candidate Configuration (TASK-023)
// ------------------------------------------------------------------

/// How RiskOff score is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskOffMode {
    /// Current: max(trend, vol, drawdown) + boost for multiple factors
    Max,
    /// Weighted average: same structure as RiskOn
    WeightedAverage,
}

/// Per-scope candidate generator configuration.
/// 
/// This allows different markets (CN, HK, GLOBAL) to use different
/// thresholds and scoring modes without hard-coding `if scope == HK`.
#[derive(Debug, Clone)]
pub struct CandidateConfig {
    /// Threshold for RiskOn (default: 60.0)
    pub risk_on_threshold: f64,
    /// Threshold for RiskOff (default: 60.0)
    pub risk_off_threshold: f64,
    /// Weight for trend dimension in RiskOn (default: 0.40)
    pub risk_on_trend_weight: f64,
    /// Weight for volatility dimension in RiskOn (default: 0.30)
    pub risk_on_vol_weight: f64,
    /// Weight for drawdown dimension in RiskOn (default: 0.30)
    pub risk_on_drawdown_weight: f64,
    /// Weight for trend dimension in RiskOff (default: 0.40)
    pub risk_off_trend_weight: f64,
    /// Weight for volatility dimension in RiskOff (default: 0.30)
    pub risk_off_vol_weight: f64,
    /// Weight for drawdown dimension in RiskOff (default: 0.30)
    pub risk_off_drawdown_weight: f64,
    /// How to compute RiskOff score
    pub risk_off_mode: RiskOffMode,
    /// Minimum confidence for Neutral (default: 30.0)
    pub neutral_min_confidence: f64,
}

impl Default for CandidateConfig {
    fn default() -> Self {
        Self {
            risk_on_threshold: 60.0,
            risk_off_threshold: 60.0,
            risk_on_trend_weight: 0.40,
            risk_on_vol_weight: 0.30,
            risk_on_drawdown_weight: 0.30,
            risk_off_trend_weight: 0.40,
            risk_off_vol_weight: 0.30,
            risk_off_drawdown_weight: 0.30,
            risk_off_mode: RiskOffMode::Max,
            neutral_min_confidence: 30.0,
        }
    }
}

impl CandidateConfig {
    /// Get the default configuration for a given scope.
    /// 
    /// Currently all scopes use the same defaults, but this is the
    /// hook for scope-specific tuning (e.g., HK can override thresholds).
    pub fn for_scope(scope: &str) -> Self {
        match scope {
            // HK can have independent config in the future
            // "HK" => Self { risk_off_threshold: 70.0, ..Default::default() },
            _ => Self::default(),
        }
    }
}

pub struct CandidateGenerator;

impl CandidateGenerator {
    /// Generate a RegimeCandidate from a single day's MarketStateObservation.
    /// No persistence, no history — just "what does today look like?"
    pub fn generate(obs: &MarketStateObservation) -> (RegimeCandidate, f64) {
        Self::generate_with_config(obs, &CandidateConfig::default())
    }

    /// Generate with scope-aware configuration.
    pub fn generate_for_scope(obs: &MarketStateObservation, scope: &str) -> (RegimeCandidate, f64) {
        Self::generate_with_config(obs, &CandidateConfig::for_scope(scope))
    }

    /// Generate with explicit configuration.
    pub fn generate_with_config(
        obs: &MarketStateObservation,
        config: &CandidateConfig,
    ) -> (RegimeCandidate, f64) {
        let risk_on_score = Self::score_risk_on(obs, config);
        let risk_off_score = Self::score_risk_off(obs, config);

        if risk_on_score > config.risk_on_threshold && risk_on_score > risk_off_score {
            let confidence = risk_on_score.min(100.0);
            (RegimeCandidate::RiskOn, confidence)
        } else if risk_off_score > config.risk_off_threshold && risk_off_score > risk_on_score {
            let confidence = risk_off_score.min(100.0);
            (RegimeCandidate::RiskOff, confidence)
        } else {
            // Neutral: confidence is inverse of how close we are to either extreme
            let max_extreme = risk_on_score.max(risk_off_score);
            let confidence = (100.0 - max_extreme).max(config.neutral_min_confidence);
            (RegimeCandidate::Neutral, confidence)
        }
    }

    /// Backward compatibility: generate with variant.
    #[deprecated(since = "0.2.0", note = "Use generate_with_config instead")]
    pub fn generate_with_variant(
        obs: &MarketStateObservation,
        variant: CandidateVariant,
    ) -> (RegimeCandidate, f64) {
        let risk_on_score = Self::score_risk_on_legacy(obs);
        let risk_off_score = match variant {
            CandidateVariant::Baseline => Self::score_risk_off_legacy(obs),
            CandidateVariant::Symmetric => Self::score_risk_off_symmetric_legacy(obs),
        };

        if risk_on_score > 60.0 && risk_on_score > risk_off_score {
            let confidence = risk_on_score.min(100.0);
            (RegimeCandidate::RiskOn, confidence)
        } else if risk_off_score > 60.0 && risk_off_score > risk_on_score {
            let confidence = risk_off_score.min(100.0);
            (RegimeCandidate::RiskOff, confidence)
        } else {
            let max_extreme = risk_on_score.max(risk_off_score);
            let confidence = (100.0 - max_extreme).max(30.0);
            (RegimeCandidate::Neutral, confidence)
        }
    }

    /// RiskOn scoring with config-aware weights.
    fn score_risk_on(obs: &MarketStateObservation, config: &CandidateConfig) -> f64 {
        let trend_ok = match (obs.trend.short_term, obs.trend.medium_term) {
            (TrendDirection::StrongUptrend, TrendDirection::StrongUptrend) => 100.0,
            (TrendDirection::StrongUptrend, TrendDirection::Uptrend)
            | (TrendDirection::Uptrend, TrendDirection::StrongUptrend) => 85.0,
            (TrendDirection::Uptrend, TrendDirection::Uptrend) => 70.0,
            (TrendDirection::Uptrend, _) | (_, TrendDirection::Uptrend) => 50.0,
            _ => 0.0,
        };

        let vol_ok = match obs.volatility.volatility_regime {
            VolatilityRegime::Low => 100.0,
            VolatilityRegime::Normal => 80.0,
            VolatilityRegime::Elevated => 40.0,
            VolatilityRegime::Spike => 0.0,
        };

        let drawdown_ok = if obs.drawdown_pct > -5.0 {
            100.0
        } else if obs.drawdown_pct > -10.0 {
            80.0
        } else if obs.drawdown_pct > -15.0 {
            50.0
        } else {
            0.0
        };

        trend_ok * config.risk_on_trend_weight
            + vol_ok * config.risk_on_vol_weight
            + drawdown_ok * config.risk_on_drawdown_weight
    }

    /// RiskOff scoring with config-aware mode and weights.
    fn score_risk_off(obs: &MarketStateObservation, config: &CandidateConfig) -> f64 {
        let trend_bad = match (obs.trend.short_term, obs.trend.medium_term) {
            (TrendDirection::StrongDowntrend, _) | (_, TrendDirection::StrongDowntrend) => 100.0,
            (TrendDirection::Downtrend, _) | (_, TrendDirection::Downtrend) => 70.0,
            _ => 0.0,
        };

        let vol_bad = match obs.volatility.volatility_regime {
            VolatilityRegime::Spike => 100.0,
            VolatilityRegime::Elevated => 70.0,
            VolatilityRegime::Normal => 20.0,
            VolatilityRegime::Low => 0.0,
        };

        let drawdown_bad = if obs.drawdown_pct < -25.0 {
            100.0
        } else if obs.drawdown_pct < -20.0 {
            85.0
        } else if obs.drawdown_pct < -15.0 {
            60.0
        } else if obs.drawdown_pct < -10.0 {
            30.0
        } else {
            0.0
        };

        match config.risk_off_mode {
            RiskOffMode::Max => {
                let max_factor = f64::max(f64::max(trend_bad, vol_bad), drawdown_bad);
                let factor_count = (if trend_bad > 50.0 { 1 } else { 0 })
                    + (if vol_bad > 50.0 { 1 } else { 0 })
                    + (if drawdown_bad > 50.0 { 1 } else { 0 });

                let boost = match factor_count {
                    2 => 10.0,
                    3 => 20.0,
                    _ => 0.0,
                };

                (max_factor + boost).min(100.0)
            }
            RiskOffMode::WeightedAverage => {
                trend_bad * config.risk_off_trend_weight
                    + vol_bad * config.risk_off_vol_weight
                    + drawdown_bad * config.risk_off_drawdown_weight
            }
        }
    }

    // Legacy scoring functions (kept for backward compatibility)

    /// RiskOn scoring: strong trend + low vol + shallow drawdown
    fn score_risk_on_legacy(obs: &MarketStateObservation) -> f64 {
        Self::score_risk_on(obs, &CandidateConfig::default())
    }

    /// RiskOff scoring: downtrend OR deep drawdown OR high volatility
    fn score_risk_off_legacy(obs: &MarketStateObservation) -> f64 {
        Self::score_risk_off(obs, &CandidateConfig::default())
    }

    /// Symmetric RiskOff scoring: weighted average like RiskOn
    fn score_risk_off_symmetric_legacy(obs: &MarketStateObservation) -> f64 {
        let mut config = CandidateConfig::default();
        config.risk_off_mode = RiskOffMode::WeightedAverage;
        Self::score_risk_off(obs, &config)
    }
}

// ------------------------------------------------------------------
// Layer 3: Persistence Filter (state machine)
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Number of consecutive days a candidate must persist before switching.
    /// ADR-058: Changed from 10 to 1 based on TASK-034C Episode Survival Audit.
    /// CN median episode=2.0d, HK median=3.0d. 10d exceeded state lifetime.
    pub confirmation_days: usize,
    /// Minimum days a regime must hold before any transition is considered.
    /// Default: 5 days.
    pub min_days: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            confirmation_days: 1,
            min_days: 5,
        }
    }
}

pub struct PersistenceFilter {
    config: PersistenceConfig,
    current_regime: Option<Regime>,
    current_days: usize,
    candidate_regime: Option<Regime>,
    candidate_days: usize,
}

impl PersistenceFilter {
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            current_regime: None,
            current_days: 0,
            candidate_regime: None,
            candidate_days: 0,
        }
    }

    /// Feed a new candidate and receive the stable regime (if any).
    ///
    /// Returns `Some(regime)` when a stable regime is established or maintained.
    /// Returns `None` during initial warm-up or when transitioning.
    pub fn update(&mut self, candidate: RegimeCandidate) -> Option<Regime> {
        let candidate_regime = Self::candidate_to_regime(candidate);

        // If no current regime, establish immediately
        if self.current_regime.is_none() {
            self.current_regime = Some(candidate_regime);
            self.current_days = 1;
            self.candidate_regime = None;
            self.candidate_days = 0;
            return Some(candidate_regime);
        }

        let current = self.current_regime.unwrap();

        // If candidate matches current, extend current regime
        if candidate_regime == current {
            self.current_days += 1;
            self.candidate_regime = None;
            self.candidate_days = 0;
            return Some(current);
        }

        // If this is a new candidate (different from current and previous candidate)
        if self.candidate_regime != Some(candidate_regime) {
            self.candidate_regime = Some(candidate_regime);
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
            self.current_regime = Some(candidate_regime);
            self.current_days = 1;
            self.candidate_regime = None;
            self.candidate_days = 0;
            return Some(candidate_regime);
        }

        // Not enough confirmation yet, stay in current regime
        self.current_days += 1;
        Some(current)
    }

    /// Get the current stable regime without updating.
    pub fn current_regime(&self) -> Option<Regime> {
        self.current_regime
    }

    /// Get the number of days the current regime has held.
    pub fn current_days(&self) -> usize {
        self.current_days
    }

    fn candidate_to_regime(candidate: RegimeCandidate) -> Regime {
        match candidate {
            RegimeCandidate::RiskOn => Regime::RiskOn,
            RegimeCandidate::Neutral => Regime::Neutral,
            RegimeCandidate::RiskOff => Regime::RiskOff,
        }
    }
}

// ------------------------------------------------------------------
// Layer 4: Full Pipeline
// ------------------------------------------------------------------

pub struct RegimePipeline {
    filter: PersistenceFilter,
    scope: String,
    candidate_config: CandidateConfig,
}

impl RegimePipeline {
    pub fn new(scope: &str) -> Self {
        Self {
            filter: PersistenceFilter::new(PersistenceConfig::default()),
            scope: scope.to_string(),
            candidate_config: CandidateConfig::for_scope(scope),
        }
    }

    pub fn with_config(scope: &str, config: PersistenceConfig) -> Self {
        Self {
            filter: PersistenceFilter::new(config),
            scope: scope.to_string(),
            candidate_config: CandidateConfig::for_scope(scope),
        }
    }

    /// Process a single observation and return the full RegimeLabel.
    pub fn process(&mut self, obs: &MarketStateObservation) -> Option<RegimeLabel> {
        let (candidate, confidence) =
            CandidateGenerator::generate_with_config(obs, &self.candidate_config);
        let regime = self.filter.update(candidate)?;

        Some(RegimeLabel {
            regime,
            candidate,
            confidence,
            days_in_regime: self.filter.current_days(),
            date: obs.date,
            scope: self.scope.clone(),
        })
    }

    /// Batch process a sequence of observations.
    pub fn process_sequence(
        &mut self,
        observations: &[MarketStateObservation],
    ) -> Vec<RegimeLabel> {
        observations
            .iter()
            .filter_map(|obs| self.process(obs))
            .collect()
    }
}

// ------------------------------------------------------------------
// Convenience: batch pipeline without manual state management
// ------------------------------------------------------------------

/// Run the full Observation → RegimeLabel pipeline on a sequence.
pub fn generate_regime_labels(
    observations: &[MarketStateObservation],
    scope: &str,
) -> Vec<RegimeLabel> {
    let mut pipeline = RegimePipeline::new(scope);
    pipeline.process_sequence(observations)
}

/// Run with custom persistence config.
pub fn generate_regime_labels_with_config(
    observations: &[MarketStateObservation],
    scope: &str,
    config: PersistenceConfig,
) -> Vec<RegimeLabel> {
    let mut pipeline = RegimePipeline::with_config(scope, config);
    pipeline.process_sequence(observations)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use market_state_extractor::{
        LiquidityObservation, MarketStateObservation, TrendObservation, VolatilityObservation,
    };
    #[allow(unused_imports)]
    use market_state_extractor::VolumeRegime;

    fn make_obs(
        date: NaiveDate,
        short_term: TrendDirection,
        medium_term: TrendDirection,
        vol_regime: VolatilityRegime,
        drawdown_pct: f64,
    ) -> MarketStateObservation {
        MarketStateObservation {
            date,
            scope: "TEST".to_string(),
            trend: TrendObservation {
                short_term,
                medium_term,
                momentum_strength: 50.0,
                trend_consistency: 80.0,
            },
            liquidity: LiquidityObservation {
                volume_regime: VolumeRegime::Neutral,
                turnover_strength: 50.0,
            },
            volatility: VolatilityObservation {
                volatility_regime: vol_regime,
                realized_vol: 0.15,
            },
            breadth: None,
            drawdown_pct,
        }
    }

    #[test]
    fn test_risk_on_candidate() {
        let obs = make_obs(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            TrendDirection::StrongUptrend,
            TrendDirection::StrongUptrend,
            VolatilityRegime::Low,
            -2.0,
        );
        let (candidate, confidence) = CandidateGenerator::generate(&obs);
        assert_eq!(candidate, RegimeCandidate::RiskOn);
        assert!(confidence > 80.0, "RiskOn confidence should be high: {}", confidence);
    }

    #[test]
    fn test_risk_off_candidate() {
        let obs = make_obs(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            TrendDirection::StrongDowntrend,
            TrendDirection::Downtrend,
            VolatilityRegime::Spike,
            -25.0,
        );
        let (candidate, confidence) = CandidateGenerator::generate(&obs);
        assert_eq!(candidate, RegimeCandidate::RiskOff);
        assert!(confidence > 80.0, "RiskOff confidence should be high: {}", confidence);
    }

    #[test]
    fn test_neutral_candidate() {
        let obs = make_obs(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            TrendDirection::Sideways,
            TrendDirection::Sideways,
            VolatilityRegime::Normal,
            -5.0,
        );
        let (candidate, _confidence) = CandidateGenerator::generate(&obs);
        assert_eq!(candidate, RegimeCandidate::Neutral);
    }

    #[test]
    fn test_persistence_filter_switches_after_10_days() {
        let mut filter = PersistenceFilter::new(PersistenceConfig {
            confirmation_days: 10,
            min_days: 5,
        });

        // Establish initial RiskOn regime
        filter.update(RegimeCandidate::RiskOn);
        assert_eq!(filter.current_regime(), Some(Regime::RiskOn));

        // 9 days of RiskOff candidate — should NOT switch yet
        for _ in 0..9 {
            let regime = filter.update(RegimeCandidate::RiskOff);
            assert_eq!(regime, Some(Regime::RiskOn), "Should not switch before 10 days");
        }

        // 10th day of RiskOff — NOW should switch
        let regime = filter.update(RegimeCandidate::RiskOff);
        assert_eq!(regime, Some(Regime::RiskOff), "Should switch after 10 consecutive days");
    }

    #[test]
    fn test_persistence_filter_resets_on_interruption() {
        let mut filter = PersistenceFilter::new(PersistenceConfig {
            confirmation_days: 10,
            min_days: 5,
        });

        // Establish RiskOn
        filter.update(RegimeCandidate::RiskOn);

        // 5 days of RiskOff candidate
        for _ in 0..5 {
            filter.update(RegimeCandidate::RiskOff);
        }

        // Interruption: 1 day of RiskOn candidate resets the counter
        filter.update(RegimeCandidate::RiskOn);

        // Now we need 10 more days of RiskOff
        for _ in 0..9 {
            let regime = filter.update(RegimeCandidate::RiskOff);
            assert_eq!(regime, Some(Regime::RiskOn), "Counter reset, should not switch yet");
        }

        // 10th day after reset
        let regime = filter.update(RegimeCandidate::RiskOff);
        assert_eq!(regime, Some(Regime::RiskOff));
    }

    #[test]
    fn test_full_pipeline() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut observations = Vec::new();

        // 15 days of uptrend
        for i in 0..15 {
            observations.push(make_obs(
                start + chrono::Duration::days(i),
                TrendDirection::StrongUptrend,
                TrendDirection::Uptrend,
                VolatilityRegime::Low,
                -2.0,
            ));
        }

        // 15 days of downtrend
        for i in 15..30 {
            observations.push(make_obs(
                start + chrono::Duration::days(i),
                TrendDirection::StrongDowntrend,
                TrendDirection::Downtrend,
                VolatilityRegime::Elevated,
                -20.0,
            ));
        }

        let labels = generate_regime_labels(&observations, "TEST");

        // First label should be RiskOn (established immediately)
        assert_eq!(labels.first().unwrap().regime, Regime::RiskOn);

        // After ~15 days of uptrend, all should be RiskOn
        let risk_on_count = labels.iter().filter(|l| l.regime == Regime::RiskOn).count();
        assert!(risk_on_count >= 10, "Should have at least 10 RiskOn labels");

        // After 15 days of downtrend + 10 day persistence, should switch to RiskOff
        let last = labels.last().unwrap();
        assert_eq!(last.regime, Regime::RiskOff, "Should eventually switch to RiskOff");
        assert_eq!(last.candidate, RegimeCandidate::RiskOff);
    }

    #[test]
    fn test_label_structure() {
        let obs = make_obs(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            TrendDirection::StrongUptrend,
            TrendDirection::StrongUptrend,
            VolatilityRegime::Low,
            -2.0,
        );

        let mut pipeline = RegimePipeline::new("TEST");
        let label = pipeline.process(&obs).unwrap();

        assert_eq!(label.regime, Regime::RiskOn);
        assert_eq!(label.candidate, RegimeCandidate::RiskOn);
        assert!(label.confidence > 0.0 && label.confidence <= 100.0);
        assert_eq!(label.days_in_regime, 1);
        assert_eq!(label.scope, "TEST");
    }
}
