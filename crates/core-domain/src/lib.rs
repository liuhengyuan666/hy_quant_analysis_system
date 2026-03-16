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
    pub explanation: String,
}
