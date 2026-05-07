pub mod calendar;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstrumentType {
    Index,
    Etf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Market {
    Cn,
    Hk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnalysisScope {
    Global,
    Cn,
    Hk,
}

impl AnalysisScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "GLOBAL",
            Self::Cn => "CN",
            Self::Hk => "HK",
        }
    }

    pub fn matches_market(self, market: &Market) -> bool {
        match self {
            Self::Global => true,
            Self::Cn => market == &Market::Cn,
            Self::Hk => market == &Market::Hk,
        }
    }
}

impl std::fmt::Display for AnalysisScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrategyKind {
    ValueLeft,
    TrendPullback,
    TrendBreakout,
    MomentumRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub symbol: String,
    pub name: String,
    pub display_symbol: Option<String>,
    pub instrument_type: InstrumentType,
    pub market: Market,
    pub category: String,
    pub eastmoney_secid: String,
    pub tencent_symbol: Option<String>,
    pub enabled: bool,
    pub latest_gate_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBar {
    pub date: NaiveDate,
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub turnover: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSnapshot {
    pub date: NaiveDate,
    pub symbol: String,
    pub ma10: Option<f64>,
    pub ma20: Option<f64>,
    pub ma30: Option<f64>,
    pub ma60: Option<f64>,
    pub ma120: Option<f64>,
    pub ema12: Option<f64>,
    pub ema26: Option<f64>,
    pub macd: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_hist: Option<f64>,
    pub rsi14: Option<f64>,
    pub atr14: Option<f64>,
    pub vol_ma20: Option<f64>,
    pub vol_ma60: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroSnapshot {
    pub date: NaiveDate,
    pub factor_name: String,
    pub factor_value: f64,
    pub factor_score: f64,
    pub factor_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRegimeSnapshot {
    pub date: NaiveDate,
    pub macro_as_of_date: NaiveDate,
    pub market: String,
    pub trend_score: f64,
    pub liquidity_score: f64,
    pub risk_score: f64,
    pub regime_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub date: NaiveDate,
    pub scope: String,
    pub regime_as_of_date: NaiveDate,
    pub breadth_as_of_date: NaiveDate,
    pub stress_as_of_date: NaiveDate,
    pub breadth_eligible_count: usize,
    pub breadth_above_count: usize,
    pub breadth_pct: f64,
    pub breadth_pct_sma5: Option<f64>,
    pub breadth_5d_delta: Option<f64>,
    pub breadth_state: String,
    pub volume_expansion_pct: Option<f64>,
    pub turnover_coverage_pct: Option<f64>,
    pub liquidity_proxy_score: f64,
    pub stress_proxy_score: f64,
    pub environment_score: f64,
    pub environment_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationRankSnapshot {
    pub date: NaiveDate,
    pub symbol: String,
    pub rs_20: f64,
    pub rs_60: f64,
    pub rs_120: f64,
    pub momentum_score: f64,
    pub rank: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPreferenceSnapshot {
    pub date: NaiveDate,
    pub symbol: String,
    pub analysis_scope: String,
    pub regime_basis_scope: String,
    pub value_left_score: f64,
    pub trend_pullback_score: f64,
    pub trend_breakout_score: f64,
    pub momentum_right_score: f64,
    pub best_strategy: StrategyKind,
    pub confidence: f64,
    pub alignment: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalLabel {
    StrongBuy,
    Buy,
    Watch,
    Hold,
    Reduce,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSnapshot {
    pub date: NaiveDate,
    pub symbol: String,
    pub final_score: f64,
    pub signal_label: SignalLabel,
    pub analysis_scope: String,
    pub regime_basis_scope: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalBuildStats {
    pub total: usize,
    pub regime_missing: usize,
    pub rotation_missing: usize,
}
