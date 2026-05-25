use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Main ResearchContext - modular composition, NOT a god object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContext {
    pub market: MarketContext,
    pub liquidity: LiquidityContext,
    pub breadth: BreadthContext,
    pub rotation: RotationContext,
    pub regime: RegimeContext,
    pub signals: SignalsContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContext {
    pub current_state: String,      // "risk_off_transition"
    pub previous_state: String,     // "risk_on"
    pub confidence: f64,            // [0, 1]
    pub drivers: Vec<String>,
    pub transition: Option<RegimeTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityContext {
    pub pressure: LiquidityPressure,
    pub yield_curve_status: String,  // "normal" / "flat" / "inverted"
    pub dollar_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub enum BreadthCondition {
    Strong,
    Weakening,
    Collapsed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationContext {
    pub state: RotationState,
    pub top_sectors: Vec<String>,
    pub leadership_stability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
