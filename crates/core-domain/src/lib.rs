pub mod calendar;
pub mod portfolio;
pub mod research;

pub use portfolio::{AssetType, MappingQuality, PortfolioConfig, Position};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

// ============================================================
// Regime Observation Layer (Wave 7.2)
// Factual market-state dimensions before any regime classification.
// ============================================================

/// Observation of factual market-state dimensions on a given date.
/// This is Layer 1: pure measurement, no conclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeObservation {
    pub date: NaiveDate,
    pub scope: String,
    pub trend_strength: f64,
    pub breadth_strength: f64,
    pub liquidity_strength: f64,
    pub volatility_level: f64,
    pub trend_state: TrendState,
    pub breadth_state: BreadthState,
    pub liquidity_state: LiquidityState,
    pub volatility_state: VolatilityState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrendState {
    StrongUptrend,
    Uptrend,
    Sideways,
    Downtrend,
    StrongDowntrend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BreadthState {
    Expanding,
    Stable,
    Contracting,
    Collapsed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityState {
    Supportive,
    Neutral,
    Tightening,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VolatilityState {
    Low,
    Normal,
    Elevated,
    Spike,
}

/// Candidate regime before persistence filtering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegimeCandidate {
    RiskOn,
    Neutral,
    RiskOff,
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

impl std::fmt::Display for SignalLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StrongBuy => write!(f, "Strong Buy"),
            Self::Buy => write!(f, "Buy"),
            Self::Watch => write!(f, "Watch"),
            Self::Hold => write!(f, "Hold"),
            Self::Reduce => write!(f, "Reduce"),
            Self::Sell => write!(f, "Sell"),
        }
    }
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
    #[serde(default)]
    pub adversarial: Option<AdversarialSection>,
}

/// Shared adversarial context layer configuration (ADR-112/114).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialSection {
    /// 默认开启：所有 persona 调用自动注入博弈假设背景（除非 CLI 覆盖）。
    #[serde(default = "default_adversarial_auto_inject")]
    pub auto_inject: bool,
    /// 按 persona 的注入级别映射；未列出的 persona 默认 Full。
    #[serde(default)]
    pub inject: std::collections::HashMap<String, InjectLevel>,
    /// ADR-114 ContentPolicy: standard 级别注入的最大字符数（按字符计，非字节）。
    /// 与 InjectionLevel 解耦：级别决定内容粒度，此值是纯粹的体积保护。
    #[serde(default = "default_adversarial_max_chars")]
    pub max_chars: usize,
    /// ADR-114 ContentPolicy: full 级别注入的硬性上限（宽松保护值）。
    #[serde(default = "default_adversarial_full_max_chars")]
    pub full_max_chars: usize,
    /// ADR-114 ContentPolicy: 截断策略（目前仅段落边界）。
    #[serde(default)]
    pub truncate_strategy: TruncateStrategy,
}

fn default_adversarial_auto_inject() -> bool {
    true
}

/// ADR-114 ContentPolicy 默认上限。
pub fn default_adversarial_max_chars() -> usize {
    4000
}

/// ADR-114 ContentPolicy full 级别默认硬性上限。
pub fn default_adversarial_full_max_chars() -> usize {
    12000
}

/// ADR-114: 截断策略（ContentPolicy，与 InjectionLevel 独立）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TruncateStrategy {
    /// 段落边界截断：只在空行处截断，绝不截断句子中间
    /// （单个超长段落除外，作为兜底硬切）。
    #[default]
    ParagraphBoundary,
}

/// 共享博弈背景的注入级别（ADR-112）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum InjectLevel {
    /// 注入完整博弈分析全文
    Full,
    /// 注入默认 analysis_text 全文（当前同 full；截断策略由 TASK-215 ContentPolicy 决定）
    #[default]
    Standard,
    /// 仅注入摘要（~400 字符），弱注入防观点污染
    Compact,
    /// 不注入
    None,
}

impl InjectLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Standard => "standard",
            Self::Compact => "compact",
            Self::None => "none",
        }
    }
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
    0.3
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
            adversarial: None,
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

// ============================================================
// TOML-based FRED Configuration (ADR-064)
// ============================================================

/// FRED 文件配置结构体（从 TOML 加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FredFileConfig {
    pub fred: FredSection,
}

/// FRED 配置主段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FredSection {
    #[serde(default = "default_fred_enabled")]
    pub enabled: bool,
    #[serde(default = "default_fred_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub auth: FredAuthSection,
    #[serde(default = "default_fred_request_delay_ms")]
    pub request_delay_ms: u64,
    #[serde(default = "default_fred_timeout_secs")]
    pub timeout_secs: u64,
}

/// FRED 认证配置段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FredAuthSection {
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_fred_enabled() -> bool {
    true
}

fn default_fred_base_url() -> String {
    "https://api.stlouisfed.org/fred".to_string()
}

fn default_fred_request_delay_ms() -> u64 {
    500
}

fn default_fred_timeout_secs() -> u64 {
    30
}

impl Default for FredSection {
    fn default() -> Self {
        Self {
            enabled: default_fred_enabled(),
            base_url: default_fred_base_url(),
            auth: FredAuthSection::default(),
            request_delay_ms: default_fred_request_delay_ms(),
            timeout_secs: default_fred_timeout_secs(),
        }
    }
}

impl Default for FredFileConfig {
    fn default() -> Self {
        Self {
            fred: FredSection::default(),
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

// ============================================================
// State Alignment Score (TASK-025)
// Measures how well regime labels align with actual market states.
//
// Design principles (per ADR-052 + Wave 7 audit):
// 1. Audit PRODUCTION pipeline (macro-engine), not gt-regime-generator.
// 2. Multiple drawdown thresholds (10/20/30%) to avoid HK false positive.
// 3. Strict daily alignment + separate regime-change detection with tolerance.
// 4. Information score (entropy) to catch "69% RiskOff" collapse.
// 5. Pass gate: overall_alignment > 0.75 AND information_score > 0.60.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawdownAlignment {
    pub dd10_precision: f64,
    pub dd10_recall: f64,
    pub dd10_f1: f64,
    pub dd20_precision: f64,
    pub dd20_recall: f64,
    pub dd20_f1: f64,
    pub dd30_precision: f64,
    pub dd30_recall: f64,
    pub dd30_f1: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAlignment {
    pub riskon_precision: f64,
    pub riskon_recall: f64,
    pub riskon_f1: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectionMetrics {
    pub precision: f64,
    pub recall: f64,
    pub avg_latency_days: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeInformationScore {
    pub entropy: f64,
    pub normalized_entropy: f64,
    pub effective_states: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAlignmentScore {
    pub scope: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,

    // Daily strict alignment (no tolerance)
    pub drawdown_alignment: DrawdownAlignment,
    pub trend_alignment: TrendAlignment,

    // Regime-change detection (±tolerance_days)
    pub change_detection: ChangeDetectionMetrics,

    // Information content
    pub information_score: RegimeInformationScore,

    // Overall verdict
    pub overall_alignment: f64,
    pub overall_information: f64,
    pub overall_passed: bool,
}

// ============================================================
// TASK-026: Macro Factor Alignment Audit DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAlignment {
    pub factor_name: String,
    pub dd10_f1: f64,
    pub dd20_f1: f64,
    pub dd30_f1: f64,
    pub uptrend_f1: f64,
    pub information_score: RegimeInformationScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAlignmentReport {
    pub scope: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,

    pub trend_alignment: FactorAlignment,
    pub risk_alignment: FactorAlignment,
    pub liquidity_alignment: FactorAlignment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalsePositiveBreakdown {
    pub total_riskoff_days: usize,
    pub false_positive_days: usize,

    pub caused_by_trend_only: usize,
    pub caused_by_risk_only: usize,
    pub caused_by_both: usize,

    pub risk_fp_by_vix: usize,
    pub risk_fp_by_dollar_index: usize,
    pub risk_fp_by_both_macro: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalseNegativeBreakdown {
    pub total_dd20_days: usize,
    pub missed_by_trend: usize,
    pub missed_by_risk: usize,
    pub missed_by_liquidity: usize,
    pub missed_by_all: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualVariant {
    pub name: String,
    pub regime_distribution: HashMap<String, f64>,
    pub alignment: f64,
    pub information: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualReport {
    pub scope: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub variants: Vec<CounterfactualVariant>,
}

// ============================================================
// TASK-027: Economic Replay Validation DTOs
// Combines alignment + economic metrics per regime variant.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicRegimeStat {
    pub count: usize,
    pub pct: f64,
    pub forward_return_20d_mean: f64,
    pub forward_return_60d_mean: f64,
    pub max_drawdown_median: f64,
    pub volatility_median: f64,
    pub sharpe_median: f64,
    pub win_rate_20d: f64,
    pub win_rate_60d: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSeparationScore {
    pub overall_score: f64,
    pub gate_results: HashMap<String, bool>,
    pub rank_scores: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicReplayVariant {
    pub name: String,
    pub regime_distribution: HashMap<String, f64>,
    pub alignment: f64,
    pub information: f64,
    pub economic_stats: HashMap<String, EconomicRegimeStat>,
    pub separation_score: EconomicSeparationScore,
    pub assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicReplayReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub variants: Vec<EconomicReplayVariant>,
}

// ============================================================
// TASK-028A: Economic Attribution Audit DTOs
// Identifies which factor truly contributes economic value.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAttribution {
    pub factor_name: String,
    pub pearson_corr_20d: f64,
    pub pearson_corr_60d: f64,
    pub spearman_corr_20d: f64,
    pub spearman_corr_60d: f64,
    pub mutual_information_20d: f64,
    pub mutual_information_60d: f64,
    pub per_regime_corr: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicAttributionReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub factor_attributions: Vec<FactorAttribution>,
    pub dominant_factor: String,
    pub economic_vs_alignment_divergence: bool,
}

// ============================================================
// TASK-028B: Pareto Frontier Analysis DTOs
// Maps Alignment vs Economic Separation trade-off.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParetoPoint {
    pub variant: String,
    pub alignment: f64,
    pub separation_score: f64,
    pub information: f64,
    pub is_pareto_optimal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParetoFrontierReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub points: Vec<ParetoPoint>,
    pub correlation: f64,
    pub trade_off_detected: bool,
    pub pareto_optimal_variants: Vec<String>,
}

// ============================================================
// TASK-029: Economic Regime Prototype DTOs
// Independent economic-prediction layer (Favorable/Neutral/Unfavorable).
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EconomicState {
    Favorable,
    Neutral,
    Unfavorable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicRegimeSnapshot {
    pub date: NaiveDate,
    pub scope: String,
    pub state: EconomicState,
    pub dominant_factor: String,
    pub factor_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicRegimePrototypeReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub state_distribution: HashMap<String, f64>,
    pub economic_separation: f64,
    pub validation_status: String,
}

// ============================================================
// TASK-030: Dual Layer Validation DTOs
// Validates independence between State Layer and Economic Layer.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossMatrixCell {
    pub state_regime: String,
    pub economic_regime: String,
    pub count: usize,
    pub pct: f64,
    pub fwd_ret_20d_mean: f64,
    pub fwd_ret_60d_mean: f64,
    pub sharpe: f64,
    pub max_dd_median: f64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityWindowResult {
    pub window_label: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub economic_separation: f64,
    pub cramer_v: f64,
    pub mutual_information: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualLayerValidationReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub cross_matrix: Vec<CrossMatrixCell>,
    pub mutual_information: f64,
    pub cramer_v: f64,
    pub orthogonality_pass: bool,
    pub stability_results: Vec<StabilityWindowResult>,
    pub validation_status: String,
}

// ============================================================
// TASK-032: Allocation Prototype DTOs
// Backtest 4 strategies: Baseline, State-Only, Economic-Only, Dual-Layer.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationStrategyResult {
    pub strategy: String,
    pub cagr: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub max_drawdown: f64,
    pub turnover: f64,
    pub final_value: f64,
    pub total_return: f64,
    pub avg_position: f64,
}

// ============================================================
// TASK-033: State Signal Decomposition Audit DTOs
// Explains why State Layer performs so well in allocation.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateReturnAttribution {
    pub state: String,
    pub count: usize,
    pub pct: f64,
    pub total_return_contribution: f64,
    pub avg_daily_return: f64,
    pub avg_20d_return: f64,
    pub avg_60d_return: f64,
    pub win_rate: f64,
    pub sharpe: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceStrategyResult {
    pub confirmation_days: usize,
    pub cagr: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,
    pub turnover: f64,
    pub final_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionAlpha {
    pub from_state: String,
    pub to_state: String,
    pub count: usize,
    pub avg_20d_return: f64,
    pub avg_60d_return: f64,
    pub win_rate_20d: f64,
    pub win_rate_60d: f64,
    pub max_dd_median: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSignalDecompositionReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub state_attributions: Vec<StateReturnAttribution>,
    pub persistence_comparison: Vec<PersistenceStrategyResult>,
    pub transition_alphas: Vec<TransitionAlpha>,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationPrototypeReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub strategies: Vec<AllocationStrategyResult>,
    pub dual_better_than_baseline: bool,
    pub dual_better_than_state: bool,
    pub dual_better_than_economic: bool,
}

// ============================================================
// TASK-034: Persistence Frontier Audit DTOs
// Maps persistence days vs Alignment + Economic metrics.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceFrontierPoint {
    pub confirmation_days: usize,
    pub alignment: f64,
    pub information: f64,
    pub cagr: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub max_drawdown: f64,
    pub turnover: f64,
    pub final_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceFrontierReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub points: Vec<PersistenceFrontierPoint>,
    pub optimal_days: usize,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMechanicsDistribution {
    pub risk_on_days: usize,
    pub neutral_days: usize,
    pub risk_off_days: usize,
    pub total_days: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMechanicsEpisode {
    pub regime: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub duration_days: usize,
    pub confirmed_at_day: usize,
    pub delayed_days: usize,
    pub swallowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMechanicsPoint {
    pub confirmation_days: usize,
    pub distribution: PersistenceMechanicsDistribution,
    pub single_day_flips: usize,
    pub total_transitions: usize,
    pub episodes: Vec<PersistenceMechanicsEpisode>,
    pub avg_delay_days: f64,
    pub swallowed_regimes: usize,
    pub merged_regimes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMechanicsReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub points: Vec<PersistenceMechanicsPoint>,
    pub q1_single_day_flip_count: usize,
    pub q2_state_distribution_comparison: String,
    pub q3_delayed_confirmation_analysis: String,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSurvivalBucket {
    pub bucket_label: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSurvivalPoint {
    pub confirmation_days: usize,
    pub survival_rate: f64,
    pub swallowed_count: usize,
    pub survived_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSurvivalReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub total_episodes: usize,
    pub avg_episode_days: f64,
    pub median_episode_days: f64,
    pub p25_episode_days: f64,
    pub p75_episode_days: f64,
    pub p95_episode_days: f64,
    pub buckets: Vec<EpisodeSurvivalBucket>,
    pub survival_curve: Vec<EpisodeSurvivalPoint>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelDistributionPoint {
    pub persistence_days: usize,
    pub risk_on_pct: f64,
    pub neutral_pct: f64,
    pub risk_off_pct: f64,
    pub effective_states: usize,
    pub information_score: f64,
    pub episode_count: usize,
    pub median_episode_days: f64,
    pub avg_episode_days: f64,
    pub alignment_score: f64,
    pub transition_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelDistributionReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub points: Vec<LabelDistributionPoint>,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreHistogramBucket {
    pub range: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreThresholdHit {
    pub condition: String,
    pub days_met: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistribution {
    pub metric: String,
    pub mean: f64,
    pub median: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub buckets: Vec<ScoreHistogramBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistributionReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub trend_distribution: ScoreDistribution,
    pub risk_distribution: ScoreDistribution,
    pub liquidity_distribution: ScoreDistribution,
    pub threshold_hits: Vec<ScoreThresholdHit>,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave8ComparisonPoint {
    pub persistence_days: usize,
    pub alignment_score: f64,
    pub information_score: f64,
    pub economic_separation: f64,
    pub state_only_cagr: f64,
    pub state_only_sharpe: f64,
    pub dual_layer_cagr: f64,
    pub dual_layer_sharpe: f64,
    pub baseline_cagr: f64,
    pub baseline_sharpe: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave8RevalidationReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub comparisons: Vec<Wave8ComparisonPoint>,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthConfusionCell {
    pub predicted: String,
    pub actual: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthClassMetrics {
    pub class: String,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub support: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroundTruthDistribution {
    pub label: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub predicted_distribution: Vec<GroundTruthDistribution>,
    pub actual_distribution: Vec<GroundTruthDistribution>,
    pub confusion_matrix: Vec<GroundTruthConfusionCell>,
    pub class_metrics: Vec<GroundTruthClassMetrics>,
    pub overall_accuracy: f64,
    pub macro_f1: f64,
    pub conclusion: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTriggerResult {
    pub name: String,
    pub description: String,
    pub triggered: bool,
    pub weight: f64,
}

// ============================================================
// V5 Startup Freshness Check DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupFreshnessCheck {
    pub has_data: bool,
    pub latest_db_date: Option<NaiveDate>,
    pub expected_date: Option<NaiveDate>,
    pub gap_days: i64,
    pub auto_ingest_eligible: bool,
    pub requires_manual_action: bool,
    pub message: String,
}
