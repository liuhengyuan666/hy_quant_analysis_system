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
/// Rotation rank row. Persisted to ClickHouse as JSON; fields added later MUST
/// carry `#[serde(default)]` to avoid breaking deserialization of old rows.
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
/// Signal snapshot row. Persisted to ClickHouse as JSON; fields added later MUST
/// carry `#[serde(default)]` to avoid breaking deserialization of old rows.
pub struct SignalSnapshot {
    pub date: NaiveDate,
    pub symbol: String,
    pub final_score: f64,
    pub signal_label: SignalLabel,
    pub analysis_scope: String,
    pub regime_basis_scope: String,
    pub reason: SignalReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalBuildStats {
    pub total: usize,
    pub regime_missing: usize,
    pub rotation_missing: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RefreshJobRecord {
    pub id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub stages_json: String,
    pub last_successful_stage: Option<String>,
    pub error: Option<String>,
    pub refresh_from: Option<String>,
    pub refresh_to: Option<String>,
}

/// Structured breakdown of how a signal score was derived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeReason {
    pub trend_score: f64,
    pub risk_score: f64,
    pub combined_score: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationReason {
    pub momentum_score: f64,
    pub rank: Option<u32>,
    pub combined_score: f64,
    pub contribution: f64,
}

/// Signal reason breakdown. Stored as JSON blob in the `explanation` column.
/// Any new field MUST carry `#[serde(default)]` (or a struct-level Default impl)
/// to avoid breaking deserialization of old stored rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalReason {
    pub best_strategy: StrategyKind,
    pub strategy_score: f64,
    pub strategy_contribution: f64,
    pub alignment: u8,
    pub aligned_strategies: Vec<StrategyKind>,
    pub alignment_contribution: f64,
    pub regime: RegimeReason,
    pub rotation: RotationReason,
    pub final_score: f64,
    pub label: SignalLabel,
    pub summary: String,
}

/// Strategy state machine: represents the current market-phase recommendation
/// for a given scope (GLOBAL / CN / HK).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StrategyState {
    NoTrade,
    LeftProbe,
    ConfirmAdd,
    FullTrend,
    DeRisk,
}

impl StrategyState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoTrade => "NO_TRADE",
            Self::LeftProbe => "LEFT_PROBE",
            Self::ConfirmAdd => "CONFIRM_ADD",
            Self::FullTrend => "FULL_TREND",
            Self::DeRisk => "DE_RISK",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::NoTrade => "市场状态不明或风险极高，全面观望",
            Self::LeftProbe => "市场可能触底，适合小仓位试探",
            Self::ConfirmAdd => "趋势初步确认，可逐步加仓",
            Self::FullTrend => "趋势明确，风险可控，满仓操作",
            Self::DeRisk => "趋势减弱或风险上升，降低仓位",
        }
    }

    pub fn recommended_position_pct(&self) -> f64 {
        match self {
            Self::NoTrade => 0.0,
            Self::LeftProbe => 20.0,
            Self::ConfirmAdd => 60.0,
            Self::FullTrend => 100.0,
            Self::DeRisk => 30.0,
        }
    }
}

impl std::fmt::Display for StrategyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStateSnapshot {
    pub date: NaiveDate,
    pub scope: String,
    pub state: StrategyState,
    pub state_score: f64,
    pub transition_reason: String,
    pub recommended_position_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

// ============================================================
// TOML-based LLM Configuration (Phase 1: Config Migration)
// ============================================================

/// LLM 文件配置结构体（从 TOML 加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFileConfig {
    pub llm: LlmSection,
}

/// LLM 配置主段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSection {
    pub base_url: String,
    pub model: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub defaults: DefaultsSection,
}

/// 认证配置段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthSection {
    #[serde(default)]
    pub api_key: Option<String>,
}

/// 默认参数段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSection {
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    pub seed: Option<u64>,
}

fn default_timeout() -> u64 {
    60
}
fn default_temperature() -> f64 {
    0.7
}
fn default_max_tokens() -> usize {
    4096
}

impl Default for DefaultsSection {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            seed: None,
        }
    }
}

impl Default for LlmSection {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            timeout_secs: 60,
            auth: AuthSection::default(),
            defaults: DefaultsSection::default(),
        }
    }
}

impl Default for LlmFileConfig {
    fn default() -> Self {
        Self {
            llm: LlmSection::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAnalysisResult {
    pub report_date: String,
    pub scope: String,
    pub output_path: String,
    pub analysis_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreference {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

// ============================================================
// LLM Desktop Integration DTOs (ADR-048 Phase 2)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStatus {
    pub configured: bool,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileSummary {
    pub name: String,
    pub description: String,
    pub risk_tolerance: String,
    pub output_depth: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub version: String,
}
