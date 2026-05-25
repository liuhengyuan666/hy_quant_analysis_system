use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Macro linkage context (external factors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroContext {
    pub spread_10y: Option<f64>,   // 中美利差（目前无数据）
    pub dxy_index: Option<f64>,    // 美元指数（目前无数据）
    pub foreign_flow: Option<f64>, // 外资流向（目前无数据）
    pub vix: Option<f64>,          // VIX波动率（目前无数据）
}

/// Risk / tail context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskContext {
    pub skewness: Option<f64>,     // 偏度（目前无数据）
    pub kurtosis: Option<f64>,     // 峰度（目前无数据）
    pub tail_index: Option<f64>,   // 尾部指数（目前无数据）
}

/// Main ResearchContext - modular composition, NOT a god object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContext {
    pub market: MarketContext,
    pub liquidity: LiquidityContext,
    pub breadth: BreadthContext,
    pub rotation: RotationContext,
    pub regime: RegimeContext,
    pub signals: SignalsContext,
    pub macro_: MacroContext,
    pub risk: RiskContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContext {
    pub current_state: String,
    pub previous_state: Option<String>,  // None = not yet tracked
    pub confidence: f64,
    pub drivers: Vec<String>,
    pub transition: Option<RegimeTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityContext {
    pub pressure: LiquidityPressure,
    pub spread: Option<f64>,                  // 资金利差（目前无数据）
    pub yield_curve_status: Option<String>,  // None = not yet available
    pub dollar_strength: Option<f64>,         // None = not yet available
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiquidityPressure {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadthContext {
    pub condition: BreadthCondition,
    pub breadth_pct: f64,
    pub breadth_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BreadthCondition {
    Strong,
    Weakening,
    Collapsed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationContext {
    pub state: RotationState,
    pub top_sectors: Vec<String>,
    pub bottom_sectors: Vec<String>,
    pub leadership_stability: f64,
    pub momentum_factor: Option<f64>,          // 动量因子（目前无数据）
    pub value_factor: Option<f64>,             // 价值因子（目前无数据）
    pub quality_factor: Option<f64>,           // 质量因子（目前无数据）
    pub crowding_factor: Option<f64>,          // 拥挤因子（目前无数据）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RotationState {
    Broad,
    Concentrated,
    Divergent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeContext {
    pub current: String,
    pub confidence: f64,
    pub macro_stale_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalsContext {
    pub bullish_count: usize,
    pub defensive_count: usize,
    pub data_starved_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeTransition {
    pub from: String,
    pub to: String,
    pub trigger_date: NaiveDate,
    pub confidence: f64,
}
