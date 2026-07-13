use chrono::NaiveDate;
use core_domain::AnalysisScope;
use serde::{Deserialize, Serialize};

pub use core_domain::research::consensus::{Confidence, ConsensusBias, ConsensusSummary, WeightedEvidence};

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
    pub confirmation: ConfirmationSummary,
    pub recovery: RecoverySummary,
    #[serde(default)]
    pub consensus: Option<ConsensusSummary>,
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
    pub leadership_transition: String,
    #[serde(default)]
    pub rotation_acceleration: Option<f64>,
    #[serde(default)]
    pub theme_dispersion: Option<f64>,
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

/// A single dimension of market confirmation (e.g. Trend, Participation, Risk).
///
/// `score` is a normalized 0-100 index. `label` is a human-readable
/// classification derived from that score by the orchestration layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationDimension {
    pub score: f64,
    pub label: String,
}

/// Market confirmation summary.
///
/// Answers: "Has the market confirmed?" It aggregates trend, participation,
/// and risk evidence into three dimensions plus an overall verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationSummary {
    pub trend: ConfirmationDimension,
    pub participation: ConfirmationDimension,
    pub risk: ConfirmationDimension,
    pub overall: String,
}

/// Recovery summary.
///
/// Answers: "How much has the market recovered?" The `score` is the
/// user-facing Recovery Index (0-100). `drivers` lists the observable
/// factors contributing to the current recovery reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySummary {
    pub score: f64,
    pub drivers: Vec<String>,
}
