use chrono::NaiveDate;
use core_domain::{DailyBar, IndicatorSnapshot};

// ============================================================
// Semantic Observation Layer (ADR-053)
// Market-State Extractor: converts OHLCV + Indicators into
// semantic market-state observations independent from macro-engine.
// ============================================================

// ------------------------------------------------------------------
// Trend Dimension
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    StrongUptrend,
    Uptrend,
    Sideways,
    Downtrend,
    StrongDowntrend,
}

impl TrendDirection {
    /// Returns a signed score: +2 / +1 / 0 / -1 / -2
    pub fn signed_score(&self) -> i8 {
        match self {
            Self::StrongUptrend => 2,
            Self::Uptrend => 1,
            Self::Sideways => 0,
            Self::Downtrend => -1,
            Self::StrongDowntrend => -2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrendObservation {
    /// Short-term trend (MA20-based)
    pub short_term: TrendDirection,
    /// Medium-term trend (MA60-based)
    pub medium_term: TrendDirection,
    /// Momentum strength: 0-100, derived from 6-month return
    pub momentum_strength: f64,
    /// Trend consistency: 0-100, alignment between short and medium term
    pub trend_consistency: f64,
}

// ------------------------------------------------------------------
// Liquidity Dimension
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeRegime {
    Expansion,
    Neutral,
    Contraction,
}

#[derive(Debug, Clone)]
pub struct LiquidityObservation {
    /// Volume regime relative to its MA
    pub volume_regime: VolumeRegime,
    /// Turnover strength: 0-100, derived from turnover change if available
    pub turnover_strength: f64,
}

// ------------------------------------------------------------------
// Volatility Dimension
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityRegime {
    Low,
    Normal,
    Elevated,
    Spike,
}

#[derive(Debug, Clone)]
pub struct VolatilityObservation {
    /// Volatility regime classification
    pub volatility_regime: VolatilityRegime,
    /// Realized volatility (20-day, annualized)
    pub realized_vol: f64,
}

// ------------------------------------------------------------------
// Breadth Dimension (Optional — never faked)
// ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct BreadthObservation {
    /// Participation rate: % of constituents above key MA (0-100)
    pub participation_rate: f64,
    /// Expansion rate: breadth trend direction and strength (0-100)
    pub expansion_rate: f64,
}

// ------------------------------------------------------------------
// Market State Observation (top-level)
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MarketStateObservation {
    pub date: NaiveDate,
    pub scope: String,
    pub trend: TrendObservation,
    pub liquidity: LiquidityObservation,
    pub volatility: VolatilityObservation,
    /// Breadth is Optional — never synthesize from single-index data.
    /// It requires multi-constituent universe to compute meaningfully.
    pub breadth: Option<BreadthObservation>,
    /// Current drawdown from recent high (%)
    pub drawdown_pct: f64,
}

// ============================================================
// Extractor Implementation
// ============================================================

/// Build a sequence of MarketStateObservation from aligned DailyBar + IndicatorSnapshot series.
///
/// Preconditions:
/// - `bars` and `indicators` must be aligned 1:1 by date (same length, same order).
/// - `scope` is the market scope label (e.g. "CN", "HK", "GLOBAL").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirectionMethod {
    /// Original absolute threshold (0.001 price units/day)
    Baseline,
    /// Relative slope threshold (slope / close)
    RelativeSlope,
    /// Percentile-based dynamic thresholds
    Percentile,
    /// Z-score based dynamic thresholds
    ZScore,
}

impl Default for TrendDirectionMethod {
    fn default() -> Self {
        TrendDirectionMethod::Baseline
    }
}

pub fn extract_market_state_observations(
    bars: &[DailyBar],
    indicators: &[IndicatorSnapshot],
    scope: &str,
) -> Vec<MarketStateObservation> {
    extract_market_state_observations_with_method(bars, indicators, scope, TrendDirectionMethod::Baseline)
}

pub fn extract_market_state_observations_with_method(
    bars: &[DailyBar],
    indicators: &[IndicatorSnapshot],
    scope: &str,
    method: TrendDirectionMethod,
) -> Vec<MarketStateObservation> {
    assert_eq!(
        bars.len(),
        indicators.len(),
        "bars and indicators must be aligned 1:1"
    );

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let _volumes: Vec<f64> = bars.iter().map(|b| b.volume).collect();

    let mut observations = Vec::with_capacity(bars.len());
    let mut rolling_highs: Vec<f64> = Vec::with_capacity(bars.len());

    // Pre-compute rolling 60-day high for drawdown
    for index in 0..bars.len() {
        let start = index.saturating_sub(59);
        let window_high = bars[start..=index]
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        rolling_highs.push(window_high);
    }

    // Pre-compute slope distributions for Percentile and ZScore methods
    let short_slopes: Vec<f64> = (0..bars.len())
        .map(|i| slope_approx(&closes, i, 20))
        .collect();
    let medium_slopes: Vec<f64> = (0..bars.len())
        .map(|i| slope_approx(&closes, i, 60))
        .collect();

    let short_stats = SlopeStats::from_slice(&short_slopes);
    let medium_stats = SlopeStats::from_slice(&medium_slopes);

    for (index, (bar, indicator)) in bars.iter().zip(indicators.iter()).enumerate() {
        let trend = extract_trend_with_method(
            index,
            bar,
            indicator,
            &closes,
            method,
            &short_stats,
            &medium_stats,
        );
        let liquidity = extract_liquidity_from_bars(index, bars, indicator);
        let volatility = extract_volatility(index, bar, indicator, &closes);
        let drawdown_pct = calculate_drawdown(bar.close, rolling_highs[index]);

        observations.push(MarketStateObservation {
            date: bar.date,
            scope: scope.to_string(),
            trend,
            liquidity,
            volatility,
            breadth: None, // Intentionally None — breadth requires multi-constituent data
            drawdown_pct,
        });
    }

    observations
}

// ------------------------------------------------------------------
// Trend Extraction
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
struct SlopeStats {
    mean: f64,
    std: f64,
    p05: f64,
    p25: f64,
    p50: f64,
    p75: f64,
    p95: f64,
}

impl SlopeStats {
    fn from_slice(values: &[f64]) -> Self {
        let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let n = sorted.len() as f64;
        let mean = sorted.iter().sum::<f64>() / n.max(1.0);
        let variance = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n.max(1.0);
        let std = variance.sqrt();
        
        Self {
            mean,
            std,
            p05: percentile(&sorted, 0.05),
            p25: percentile(&sorted, 0.25),
            p50: percentile(&sorted, 0.50),
            p75: percentile(&sorted, 0.75),
            p95: percentile(&sorted, 0.95),
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p) as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn extract_trend(
    index: usize,
    bar: &DailyBar,
    indicator: &IndicatorSnapshot,
    closes: &[f64],
) -> TrendObservation {
    extract_trend_with_method(
        index,
        bar,
        indicator,
        closes,
        TrendDirectionMethod::Baseline,
        &SlopeStats::from_slice(&[]),
        &SlopeStats::from_slice(&[]),
    )
}

fn extract_trend_with_method(
    index: usize,
    bar: &DailyBar,
    indicator: &IndicatorSnapshot,
    closes: &[f64],
    method: TrendDirectionMethod,
    short_stats: &SlopeStats,
    medium_stats: &SlopeStats,
) -> TrendObservation {
    let short_slope = slope_approx(closes, index, 20);
    let medium_slope = slope_approx(closes, index, 60);

    let short_term = classify_trend_direction_with_method(
        bar.close,
        indicator.ma20,
        short_slope,
        method,
        short_stats,
    );

    let medium_term = classify_trend_direction_with_method(
        bar.close,
        indicator.ma60,
        medium_slope,
        method,
        medium_stats,
    );

    let momentum_strength = calculate_momentum_strength(closes, index);
    let trend_consistency = calculate_trend_consistency(short_term, medium_term);

    TrendObservation {
        short_term,
        medium_term,
        momentum_strength,
        trend_consistency,
    }
}

fn classify_trend_direction(
    close: f64,
    ma: Option<f64>,
    ma_slope: f64,
) -> TrendDirection {
    classify_trend_direction_with_method(
        close,
        ma,
        ma_slope,
        TrendDirectionMethod::Baseline,
        &SlopeStats::from_slice(&[]),
    )
}

fn classify_trend_direction_with_method(
    close: f64,
    ma: Option<f64>,
    ma_slope: f64,
    method: TrendDirectionMethod,
    stats: &SlopeStats,
) -> TrendDirection {
    let Some(ma) = ma else {
        return TrendDirection::Sideways;
    };

    let above_ma = close > ma;

    match method {
        TrendDirectionMethod::Baseline => {
            let slope_threshold = 0.001; // 0.1% per day (absolute)
            match (above_ma, ma_slope) {
                (true, s) if s > slope_threshold * 2.0 => TrendDirection::StrongUptrend,
                (true, s) if s > -slope_threshold => TrendDirection::Uptrend,
                (false, s) if s < -slope_threshold * 2.0 => TrendDirection::StrongDowntrend,
                (false, s) if s < slope_threshold => TrendDirection::Downtrend,
                _ => TrendDirection::Sideways,
            }
        }
        TrendDirectionMethod::RelativeSlope => {
            let slope_pct = if close.abs() < f64::EPSILON {
                0.0
            } else {
                ma_slope / close
            };
            let threshold = 0.001; // 0.1% per day (relative to price)
            match (above_ma, slope_pct) {
                (true, s) if s > threshold * 2.0 => TrendDirection::StrongUptrend,
                (true, s) if s > -threshold => TrendDirection::Uptrend,
                (false, s) if s < -threshold * 2.0 => TrendDirection::StrongDowntrend,
                (false, s) if s < threshold => TrendDirection::Downtrend,
                _ => TrendDirection::Sideways,
            }
        }
        TrendDirectionMethod::Percentile => {
            match (above_ma, ma_slope) {
                (true, s) if s > stats.p95 => TrendDirection::StrongUptrend,
                (true, s) if s > stats.p75 => TrendDirection::Uptrend,
                (false, s) if s < stats.p05 => TrendDirection::StrongDowntrend,
                (false, s) if s < stats.p25 => TrendDirection::Downtrend,
                _ => TrendDirection::Sideways,
            }
        }
        TrendDirectionMethod::ZScore => {
            let z = if stats.std < f64::EPSILON {
                0.0
            } else {
                (ma_slope - stats.mean) / stats.std
            };
            match (above_ma, z) {
                (true, z) if z > 1.5 => TrendDirection::StrongUptrend,
                (true, z) if z > 0.5 => TrendDirection::Uptrend,
                (false, z) if z < -1.5 => TrendDirection::StrongDowntrend,
                (false, z) if z < -0.5 => TrendDirection::Downtrend,
                _ => TrendDirection::Sideways,
            }
        }
    }
}

/// Approximate slope of the closing price over `period` days ending at `index`.
/// Uses simple linear regression slope (change per day).
pub fn slope_approx(values: &[f64], index: usize, period: usize) -> f64 {
    let start = index.saturating_sub(period - 1);
    let window = &values[start..=index];
    if window.len() < 2 {
        return 0.0;
    }

    let n = window.len() as f64;
    let sum_x: f64 = (0..window.len()).map(|i| i as f64).sum();
    let sum_y: f64 = window.iter().sum();
    let sum_xy: f64 = window
        .iter()
        .enumerate()
        .map(|(i, y)| i as f64 * y)
        .sum();
    let sum_x2: f64 = (0..window.len()).map(|i| (i as f64).powi(2)).sum();

    let denominator = n * sum_x2 - sum_x * sum_x;
    if denominator.abs() < f64::EPSILON {
        return 0.0;
    }

    (n * sum_xy - sum_x * sum_y) / denominator
}

/// 6-month (126 trading days) return normalized to 0-100.
/// Maps -30% return → 0, +30% return → 100.
fn calculate_momentum_strength(closes: &[f64], index: usize) -> f64 {
    let period = 126; // ~6 months
    let start = index.saturating_sub(period);
    if start == index {
        return 50.0;
    }

    let prev_close = closes[start];
    let current_close = closes[index];
    if prev_close.abs() < f64::EPSILON {
        return 50.0;
    }

    let ret = (current_close - prev_close) / prev_close;
    // Map [-0.30, +0.30] → [0, 100]
    ((ret + 0.30) / 0.60 * 100.0).clamp(0.0, 100.0)
}

/// Consistency between short and medium term trends.
/// +2/+2 or -2/-2 → 100, +2/-2 → 0, mixed → interpolated.
fn calculate_trend_consistency(
    short: TrendDirection,
    medium: TrendDirection,
) -> f64 {
    let short_score = short.signed_score() as f64;
    let medium_score = medium.signed_score() as f64;

    if short_score.signum() == medium_score.signum() || short_score == 0.0 || medium_score == 0.0 {
        // Same direction or one is neutral
        let diff = (short_score - medium_score).abs();
        (100.0 - diff * 25.0).max(50.0)
    } else {
        // Opposite directions
        let total_diff = short_score.abs() + medium_score.abs();
        (50.0 - total_diff * 12.5).max(0.0)
    }
}

// ------------------------------------------------------------------
// Liquidity Extraction
// ------------------------------------------------------------------

fn classify_volume_regime(volume: f64, vol_ma20: Option<f64>) -> VolumeRegime {
    let Some(ma) = vol_ma20 else {
        return VolumeRegime::Neutral;
    };
    if ma.abs() < f64::EPSILON {
        return VolumeRegime::Neutral;
    }

    let ratio = volume / ma;
    if ratio > 1.5 {
        VolumeRegime::Expansion
    } else if ratio < 0.7 {
        VolumeRegime::Contraction
    } else {
        VolumeRegime::Neutral
    }
}

// ------------------------------------------------------------------
// Volatility Extraction
// ------------------------------------------------------------------

fn extract_volatility(
    index: usize,
    _bar: &DailyBar,
    _indicator: &IndicatorSnapshot,
    closes: &[f64],
) -> VolatilityObservation {
    let realized_vol = calculate_realized_volatility(closes, index);
    let volatility_regime = classify_volatility_regime(realized_vol);

    VolatilityObservation {
        volatility_regime,
        realized_vol,
    }
}

/// 20-day realized volatility, annualized.
fn calculate_realized_volatility(closes: &[f64], index: usize) -> f64 {
    let period = 20;
    let start = index.saturating_sub(period - 1);
    if index < start + 1 {
        return 0.15; // Default 15% annualized
    }

    let mut log_returns = Vec::with_capacity(index - start);
    for i in (start + 1)..=index {
        if closes[i - 1] > 0.0 {
            log_returns.push((closes[i] / closes[i - 1]).ln());
        }
    }

    if log_returns.len() < 5 {
        return 0.15;
    }

    let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
    let variance = log_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / log_returns.len() as f64;

    // Annualize: sqrt(252) * daily_std
    variance.sqrt() * (252.0_f64).sqrt()
}

fn classify_volatility_regime(realized_vol: f64) -> VolatilityRegime {
    // Thresholds based on typical equity market vol regimes
    match realized_vol {
        v if v > 0.40 => VolatilityRegime::Spike,
        v if v > 0.25 => VolatilityRegime::Elevated,
        v if v > 0.12 => VolatilityRegime::Normal,
        _ => VolatilityRegime::Low,
    }
}

// ------------------------------------------------------------------
// Drawdown
// ------------------------------------------------------------------

fn calculate_drawdown(current_close: f64, recent_high: f64) -> f64 {
    if recent_high <= 0.0 || current_close <= 0.0 {
        return 0.0;
    }
    ((current_close - recent_high) / recent_high * 100.0).clamp(-100.0, 0.0)
}

// ------------------------------------------------------------------
// Helper: get bars slice (needed for turnover calculation)
// Since we only have volumes slice in the liquidity function, we need to
// pass bars instead. Let me fix the turnover calculation.
// ------------------------------------------------------------------

/// Recalculate liquidity with full bars access.
/// This replaces the simpler version above.
pub fn extract_liquidity_from_bars(
    index: usize,
    bars: &[DailyBar],
    indicator: &IndicatorSnapshot,
) -> LiquidityObservation {
    let bar = &bars[index];
    let volume_regime = classify_volume_regime(bar.volume, indicator.vol_ma20);
    let turnover_strength = calculate_turnover_strength_v2(index, bars);

    LiquidityObservation {
        volume_regime,
        turnover_strength,
    }
}

fn calculate_turnover_strength_v2(index: usize, bars: &[DailyBar]) -> f64 {
    let period = 20;
    let start = index.saturating_sub(period - 1);
    if start >= index {
        return 50.0;
    }

    // Check if we have turnover data
    let has_turnover = bars[start..=index]
        .iter()
        .any(|b| b.turnover.is_some());

    if has_turnover {
        // Use turnover / volume ratio as liquidity proxy
        let turnover_ratios: Vec<f64> = bars[start..=index]
            .iter()
            .filter_map(|b| {
                if b.volume > 0.0 && b.turnover.is_some() {
                    Some(b.turnover.unwrap() / b.volume)
                } else {
                    None
                }
            })
            .collect();

        if turnover_ratios.len() >= 10 {
            let mid = turnover_ratios.len() / 2;
            let recent_avg = turnover_ratios[mid..].iter().sum::<f64>()
                / (turnover_ratios.len() - mid) as f64;
            let past_avg = turnover_ratios[..mid].iter().sum::<f64>() / mid as f64;

            if past_avg > 0.0 {
                let change = (recent_avg - past_avg) / past_avg;
                return ((change + 0.30) / 0.60 * 100.0).clamp(0.0, 100.0);
            }
        }
    }

    // Fallback: volume trend
    let volumes: Vec<f64> = bars[start..=index].iter().map(|b| b.volume).collect();
    let mid = volumes.len() / 2;
    if mid > 0 {
        let recent_avg = volumes[mid..].iter().sum::<f64>() / (volumes.len() - mid) as f64;
        let past_avg = volumes[..mid].iter().sum::<f64>() / mid as f64;
        if past_avg > 0.0 {
            let change = (recent_avg - past_avg) / past_avg;
            return ((change + 0.30) / 0.60 * 100.0).clamp(0.0, 100.0);
        }
    }

    50.0
}

// ============================================================
// Public API: aligned extraction using full bars array
// ============================================================

/// Build MarketStateObservation sequence with corrected liquidity extraction.
/// This is the main entry point.
pub fn build_market_state_observations(
    bars: &[DailyBar],
    indicators: &[IndicatorSnapshot],
    scope: &str,
) -> Vec<MarketStateObservation> {
    assert_eq!(
        bars.len(),
        indicators.len(),
        "bars and indicators must be aligned 1:1"
    );

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut rolling_highs: Vec<f64> = Vec::with_capacity(bars.len());

    for index in 0..bars.len() {
        let start = index.saturating_sub(59);
        let window_high = bars[start..=index]
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        rolling_highs.push(window_high);
    }

    let mut observations = Vec::with_capacity(bars.len());

    for (index, (bar, indicator)) in bars.iter().zip(indicators.iter()).enumerate() {
        let trend = extract_trend(index, bar, indicator, &closes);
        let liquidity = extract_liquidity_from_bars(index, bars, indicator);
        let volatility = extract_volatility(index, bar, indicator, &closes);
        let drawdown_pct = calculate_drawdown(bar.close, rolling_highs[index]);

        observations.push(MarketStateObservation {
            date: bar.date,
            scope: scope.to_string(),
            trend,
            liquidity,
            volatility,
            breadth: None,
            drawdown_pct,
        });
    }

    observations
}

// ------------------------------------------------------------------
// Breadth injection (to be called by upstream when multi-constituent data is available)
// ------------------------------------------------------------------

impl MarketStateObservation {
    /// Attach breadth observation if multi-constituent data is available.
    /// This is the ONLY way breadth should ever be populated.
    pub fn with_breadth(mut self, breadth: BreadthObservation) -> Self {
        self.breadth = Some(breadth);
        self
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bar(date: NaiveDate, close: f64, volume: f64) -> DailyBar {
        DailyBar {
            date,
            symbol: "TEST".to_string(),
            open: close,
            high: close * 1.01,
            low: close * 0.99,
            close,
            volume,
            turnover: Some(volume * close * 100.0),
        }
    }

    fn make_indicator(date: NaiveDate, ma20: f64, ma60: f64, vol_ma20: f64) -> IndicatorSnapshot {
        IndicatorSnapshot {
            date,
            symbol: "TEST".to_string(),
            ma10: Some(ma20),
            ma20: Some(ma20),
            ma30: Some(ma20),
            ma60: Some(ma60),
            ma120: Some(ma60),
            ema12: None,
            ema26: None,
            macd: None,
            macd_signal: None,
            macd_hist: None,
            rsi14: None,
            atr14: None,
            vol_ma20: Some(vol_ma20),
            vol_ma60: Some(vol_ma20),
        }
    }

    #[test]
    fn test_uptrend_observation() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut bars = Vec::new();
        let mut indicators = Vec::new();

        for i in 0..200 {
            let date = start + chrono::Duration::days(i);
            let close = 100.0 + i as f64 * 0.5; // steady uptrend
            bars.push(make_bar(date, close, 1_000_000.0));
            indicators.push(make_indicator(date, close - 5.0, close - 15.0, 1_000_000.0));
        }

        let obs = build_market_state_observations(&bars, &indicators, "TEST");
        let last = obs.last().unwrap();

        assert!(matches!(last.trend.short_term, TrendDirection::Uptrend | TrendDirection::StrongUptrend));
        assert!(matches!(last.trend.medium_term, TrendDirection::Uptrend | TrendDirection::StrongUptrend));
        assert!(last.trend.momentum_strength > 50.0);
        assert!(last.trend.trend_consistency > 70.0);
        assert_eq!(last.breadth, None); // Always None until injected
    }

    #[test]
    fn test_drawdown_calculation() {
        assert_eq!(calculate_drawdown(90.0, 100.0), -10.0);
        assert_eq!(calculate_drawdown(100.0, 100.0), 0.0);
        assert_eq!(calculate_drawdown(110.0, 100.0), 0.0); // clamped at 0
    }

    #[test]
    fn test_volatility_regime() {
        assert!(matches!(classify_volatility_regime(0.50), VolatilityRegime::Spike));
        assert!(matches!(classify_volatility_regime(0.30), VolatilityRegime::Elevated));
        assert!(matches!(classify_volatility_regime(0.15), VolatilityRegime::Normal));
        assert!(matches!(classify_volatility_regime(0.05), VolatilityRegime::Low));
    }

    #[test]
    fn test_volume_regime() {
        assert!(matches!(
            classify_volume_regime(2_000_000.0, Some(1_000_000.0)),
            VolumeRegime::Expansion
        ));
        assert!(matches!(
            classify_volume_regime(500_000.0, Some(1_000_000.0)),
            VolumeRegime::Contraction
        ));
        assert!(matches!(
            classify_volume_regime(1_000_000.0, Some(1_000_000.0)),
            VolumeRegime::Neutral
        ));
    }
}
