use chrono::NaiveDate;
use core_domain::AnalysisScope;
use serde::{Deserialize, Serialize};

/// Trust level for data quality assessment.
///
/// Semantic model field: MUST use this enum instead of String.
/// Consumers should never encounter a trust level outside these variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// Trust has not been evaluated yet (default for non-dashboard paths).
    Unassessed,
    /// Data quality is below acceptable threshold.
    Low,
    /// Data quality is acceptable but not ideal.
    Medium,
    /// Data quality is high.
    High,
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unassessed => write!(f, "Unassessed"),
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
        }
    }
}

/// 统一研究语义聚合。
///
/// ResearchContext 只包含跨消费者共享的研究结论 Summary，不包含原始数据或展示相关字段。
/// 新增字段优先采用 additive 方式；避免修改已有字段语义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContext {
    pub version: u32,
    pub scope: AnalysisScope,
    pub date: NaiveDate,
    pub market_state: MarketStateSummary,
    pub breadth: BreadthSummary,
    pub rotation: RotationSummary,
    pub signal: SignalSummary,
    pub divergence: DivergenceSummary,
    pub trust: TrustSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStateSummary {
    pub label: String,
    pub trend_score: f64,
    pub liquidity_score: f64,
    pub risk_score: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadthSummary {
    pub breadth_pct: f64,
    pub sma5: Option<f64>,
    pub delta_5d: Option<f64>,
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSummary {
    pub top: Vec<RotationItem>,
    pub bottom: Vec<RotationItem>,
    pub rotation_state: String,
    pub leadership_stability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationItem {
    pub rank: i32,
    pub symbol: String,
    pub momentum_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSummary {
    pub signals: Vec<SignalItem>,
    pub bullish_count: usize,
    pub strong_buy_count: usize,
    pub average_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalItem {
    pub symbol: String,
    pub final_score: f64,
    pub signal_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceSummary {
    pub divergence_duration: i64,
    pub samples: Vec<DivergenceSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceSample {
    pub date: NaiveDate,
    pub state_label: String,
    pub signal_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSummary {
    pub level: TrustLevel,
    pub headline: String,
    pub is_data_complete: bool,
}
