mod commands;

use anyhow::Result;
use app_service::AppContext;
use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};
use market_store::StorageConfig;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReportScopeArg {
    Global,
    Cn,
    Hk,
}

impl From<ReportScopeArg> for app_service::ReportScope {
    fn from(value: ReportScopeArg) -> Self {
        match value {
            ReportScopeArg::Global => app_service::ReportScope::Global,
            ReportScopeArg::Cn => app_service::ReportScope::Cn,
            ReportScopeArg::Hk => app_service::ReportScope::Hk,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "quant-cli")]
#[command(about = "Rust quant analysis system CLI")]
struct Cli {
    #[arg(long, global = true, help = "Suppress progress output to stderr")]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    InitStorage,
    SeedUniverse,
    IngestDaily {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
    },
    ComputeIndicators,
    ComputeMacro {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
    },
    ComputeRotation,
    ComputeStrategyPreferences,
    ComputeSignals,
    RefreshAll {
        #[arg(long)]
        to: Option<NaiveDate>,
        #[arg(
            long,
            help = "Scope used for latest-date diagnostics and gate explanation only"
        )]
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(
            long,
            default_value_t = true,
            help = "Whether to include standard-scope backtests in the aggregate refresh (default: true)"
        )]
        #[arg(long, default_value_t = true)]
        run_backtests: bool,
    },
    ExplainLatestGate {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    PipelineDates {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    CheckDataHealth,
    RunBacktest {
        #[arg(long, default_value_t = 1000000.0)]
        initial_capital: f64,
        #[arg(long, default_value_t = 3)]
        max_holdings: usize,
        #[arg(long, default_value_t = 0.001)]
        fee_rate: f64,
        #[arg(long, default_value_t = 0.0005)]
        slippage_rate: f64,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = false)]
        use_state_sizing: bool,
        #[arg(long)]
        max_drawdown: Option<f64>,
    },
    DashboardSnapshot {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    DashboardDates {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    ExportReport {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        /// Export concise daily report (Insight First format)
        #[arg(long, default_value_t = false)]
        concise: bool,
    },
    ExportDataHealthReport,
    SyncAndExport {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long)]
        to: Option<NaiveDate>,
        #[arg(long, default_value_t = true)]
        run_backtests: bool,
    },
    ResearchContext {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    SetLlmConfig {
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        model: String,
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
    },
    SetLlmApiKey {
        #[arg(long)]
        key: String,
    },
    SetFredConfig {
        #[arg(long, help = "Enable FRED fetch")]
        enabled: bool,
        #[arg(long, help = "Disable FRED fetch")]
        disabled: bool,
        #[arg(long)]
        api_key: Option<String>,
    },
    ShowFredConfig,
    AnalyzeWithLlm {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long)]
        date: Option<NaiveDate>,
    },
    /// Show current LLM configuration and its source
    ShowLlmConfig {
        /// Also validate the configuration
        #[arg(long)]
        validate: bool,
    },
    /// Migrate LLM config from SQLite/Keyring to TOML file
    MigrateLlmConfig {
        /// Force overwrite if config file already exists
        #[arg(long)]
        force: bool,
    },
    /// Analyze market using a skill
    Analyze {
        /// Skill name to use
        #[arg(long)]
        skill: String,

        /// Scope to analyze
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,

        /// Agent profile name (loads from research/agents/)
        #[arg(long)]
        agent: Option<String>,

        /// Output format (json, markdown)
        #[arg(long, default_value = "json")]
        format: String,

        /// Use deterministic mode (temperature=0, seed=42)
        #[arg(long)]
        deterministic: bool,

        /// Set random seed for deterministic mode
        #[arg(long, default_value = "42")]
        seed: u64,
    },
    /// List all available research skills
    ListSkills,
    /// Benchmark a research skill across providers
    BenchmarkSkill {
        /// Skill name to benchmark
        #[arg(long)]
        skill: String,
        /// Provider config TOML file path
        #[arg(long, default_value = "config/benchmark-providers.toml")]
        provider_config: String,
        /// Runs per provider
        #[arg(long, default_value_t = 3)]
        runs: usize,
        /// Output format: json, markdown
        #[arg(long, default_value = "json")]
        format: String,
        /// Scope for building ResearchContext
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Validate historical regime predictions against forward-return ground truth
    ValidateRegimeAccuracy {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long)]
        benchmark: Option<String>,
        #[arg(long, default_value_t = 20)]
        lookforward_days: usize,
        #[arg(long, default_value_t = 0.08)]
        risk_on_threshold: f64,
        #[arg(long, default_value_t = -0.08)]
        risk_off_threshold: f64,
        #[arg(long, default_value = "reports")]
        output_dir: String,
    },
    /// Inspect stored regime labels for class balance, transitions, duration, and persistence
    InspectGroundTruth {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Generate new stable regime labels using RegimeLabelGenerator + PersistenceFilter
    GenerateRegimeLabels {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = 5)]
        min_days: usize,
        #[arg(long, default_value_t = 3)]
        confirmation_days: usize,
        #[arg(long, default_value_t = false)]
        use_percentile: bool,
    },
    /// Run full GT chain (MarketStateExtractor → GT Regime Generator → Audit) on historical data
    AuditGtRegime {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = 10)]
        confirmation_days: usize,
        #[arg(long, default_value_t = 5)]
        min_days: usize,
    },
    /// Audit GT transitions: candidate vs regime distribution, direct swings, transition paths
    AuditGtTransitions {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = 10)]
        confirmation_days: usize,
        #[arg(long, default_value_t = 5)]
        min_days: usize,
    },
    /// Audit GT candidate factor attribution: which observation dimension drives regime labels
    AuditGtCandidates {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Validate GT regimes against forward returns and risk metrics
    ValidateGtRegimes {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = 10)]
        confirmation_days: usize,
        #[arg(long, default_value_t = 5)]
        min_days: usize,
    },
    /// Audit observation layer: extract slope distributions and threshold sensitivity
    AuditObservationLayer {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Replay GT pipeline with multiple TrendDirection classifiers and compare results
    ReplayTrendSensitivity {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Sensitivity replay with economic separation scores and gate analysis
    GtSensitivityReplay {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Regime attribution audit: candidate coverage, trigger attribution, confusion against returns
    AuditAttribution {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Persistence filter sensitivity audit: test confirmation_days × min_days matrix
    AuditPersistenceSensitivity {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Market structure audit: price regime distribution, drawdown profile, CN vs HK comparison
    AuditMarketStructure {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Regime-state alignment audit: precision/recall of RiskOff vs drawdown, RiskOn vs uptrend
    AuditRegimeAlignment {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, value_delimiter = ',', default_value = "-10,-20,-30")]
        drawdown_thresholds: Vec<f64>,
        #[arg(long, default_value_t = 2)]
        tolerance_days: usize,
    },
    /// TASK-026A: Factor alignment audit — per-factor F1 + information score
    AuditFactorAlignment {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-026B: False positive / false negative breakdown
    AuditFalsePositiveBreakdown {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-026C: Counterfactual regime replay (7 variants)
    AuditCounterfactualRegime {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-027: Economic replay validation (alignment + economic metrics)
    AuditEconomicReplay {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-028A: Economic attribution audit (which factor predicts returns)
    AuditEconomicAttribution {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-028B: Pareto frontier analysis (Alignment vs Economic Separation)
    AuditParetoFrontier {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-029: Economic regime prototype (independent economic-prediction layer)
    AuditEconomicRegimePrototype {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-030: Dual layer validation (orthogonality + cross-matrix + stability)
    AuditDualLayerValidation {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-032: Allocation prototype (4-strategy backtest)
    AuditAllocationPrototype {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-033: State signal decomposition audit (why State Layer wins)
    AuditStateSignalDecomposition {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-090A: State machine transition attribution audit (which triggers drive each state)
    AuditStateTransitions {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-034: Persistence frontier audit (0/1/2/3/5/7/10 days)
    AuditPersistenceFrontier {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-034B: Persistence mechanics audit (Q1/Q2/Q3)
    AuditPersistenceMechanics {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-034C: Episode survival audit (episode length distribution)
    AuditEpisodeSurvival {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-035A.0: Label distribution audit (Wave 8 baseline panel)
    AuditLabelDistribution {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-035A.1: Score distribution audit (threshold hit rates)
    AuditScoreDistribution {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-035A Phase 2: Wave 8 revalidation (1d vs 10d comparison)
    AuditWave8Revalidation {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-035B: Ground truth audit (Alignment paradox investigation)
    AuditGroundTruth {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        /// Persistence days for predicted labels
        #[arg(long, default_value_t = 1)]
        persistence_days: usize,
    },
    /// TASK-060A.1: Forward return distribution audit (Wave 9)
    AuditForwardReturnDistribution {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
    },
    /// TASK-060B: Generate 3 Ground Truth label sets
    GenerateGroundTruthLabels {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        /// Forward return horizon in days (default: 60)
        #[arg(long, default_value_t = 60)]
        horizon: usize,
    },
    /// TASK-060C: Redesign Alignment and compare against all GT variants
    AuditAlignmentRedesign {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
        /// Forward return horizon in days (default: 60)
        #[arg(long, default_value_t = 60)]
        horizon: usize,
    },
    /// TASK-070B: State persistence economics audit
    AuditStatePersistenceEconomics {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
    },
    /// TASK-071A: State Layer Ground Truth validation demo
    ValidateStateLayerGt {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
    },
    /// TASK-092: Explainability Layer — Single-symbol signal attribution breakdown
    SymbolDiagnostics {
        #[arg(long)]
        symbol: String,
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-092: Explainability Layer — Full symbol scoreboard (Research Surface)
    SymbolScoreboard {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Research Surface: Full rotation ranking (not part of Shadow Production observation chain)
    RotationRanking {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// TASK-071B: State lead/lag analysis (before/during/after episode returns)
    AuditLeadLag {
        #[arg(long)]
        from: NaiveDate,
        #[arg(long)]
        to: NaiveDate,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    let context = AppContext::new(StorageConfig::default());

    match cli.command {
        Command::Status => commands::config::handle_status(&context)?,
        Command::InitStorage => commands::config::handle_init_storage(&context)?,
        Command::SeedUniverse => commands::config::handle_seed_universe(&context)?,
        Command::IngestDaily { from, to } => commands::pipeline::handle_ingest_daily(&context, from, to, cli.quiet)?,
        Command::ComputeIndicators => commands::pipeline::handle_compute_indicators(&context, cli.quiet)?,
        Command::ComputeMacro { from, to } => commands::pipeline::handle_compute_macro(&context, from, to, cli.quiet)?,
        Command::ComputeRotation => commands::pipeline::handle_compute_rotation(&context, cli.quiet)?,
        Command::ComputeStrategyPreferences => commands::pipeline::handle_compute_strategy_preferences(&context, cli.quiet)?,
        Command::ComputeSignals => commands::pipeline::handle_compute_signals(&context, cli.quiet)?,
        Command::RefreshAll { to, scope, run_backtests } => commands::pipeline::handle_refresh_all(&context, to, scope, run_backtests, cli.quiet)?,
        Command::ExplainLatestGate { scope } => commands::diagnostics::handle_explain_latest_gate(&context, scope)?,
        Command::PipelineDates { scope } => commands::diagnostics::handle_pipeline_dates(&context, scope)?,
        Command::CheckDataHealth => commands::diagnostics::handle_check_data_health(&context)?,
        Command::RunBacktest { initial_capital, max_holdings, fee_rate, slippage_rate, scope, use_state_sizing, max_drawdown } => {
            commands::backtest::handle_run_backtest(&context, initial_capital, max_holdings, fee_rate, slippage_rate, scope, use_state_sizing, max_drawdown)?
        }
        Command::DashboardSnapshot { date, scope } => commands::dashboard::handle_dashboard_snapshot(&context, date, scope)?,
        Command::DashboardDates { scope } => commands::dashboard::handle_dashboard_dates(&context, scope)?,
        Command::ExportReport { date, scope, concise } => commands::dashboard::handle_export_report(&context, date, scope, concise)?,
        Command::ExportDataHealthReport => commands::dashboard::handle_export_data_health_report(&context)?,
        Command::SyncAndExport { date, scope, to, run_backtests } => commands::dashboard::handle_sync_and_export(&context, date, scope, to, run_backtests, cli.quiet)?,
        Command::ResearchContext { scope } => commands::dashboard::handle_research_context(&context, scope)?,
        Command::SetLlmConfig { base_url, model, timeout_secs } => commands::llm::handle_set_llm_config(&context, base_url, model, timeout_secs)?,
        Command::SetLlmApiKey { key } => commands::llm::handle_set_llm_api_key(&context, key)?,
        Command::AnalyzeWithLlm { scope, date } => commands::llm::handle_analyze_with_llm(&context, scope, date, cli.quiet)?,
        Command::ShowLlmConfig { validate } => commands::llm::handle_show_llm_config(&context, validate)?,
        Command::MigrateLlmConfig { force } => commands::llm::handle_migrate_llm_config(&context, force)?,
        Command::SetFredConfig { enabled, disabled, api_key } => commands::llm::handle_set_fred_config(&context, enabled, disabled, api_key)?,
        Command::ShowFredConfig => commands::llm::handle_show_fred_config(&context)?,
        Command::ListSkills => commands::research::handle_list_skills()?,
        Command::BenchmarkSkill { skill, provider_config, runs, format, scope } => {
            commands::research::handle_benchmark_skill(&context, skill, provider_config, runs, format, scope, cli.quiet)?
        }
        Command::Analyze { skill, scope, agent, format, deterministic, seed } => {
            commands::research::handle_analyze(context, skill, scope, agent, format, deterministic, seed)?
        }
        Command::ValidateRegimeAccuracy { from, to, scope, benchmark, lookforward_days, risk_on_threshold, risk_off_threshold, output_dir } => {
            commands::audit::handle_validate_regime_accuracy(&context, from, to, scope, benchmark, lookforward_days, risk_on_threshold, risk_off_threshold, output_dir)?
        }
        Command::InspectGroundTruth { from, to, scope } => commands::audit::handle_inspect_ground_truth(&context, from, to, scope)?,
        Command::GenerateRegimeLabels { from, to, scope, min_days, confirmation_days, use_percentile } => {
            commands::audit::handle_generate_regime_labels(from, to, scope, min_days, confirmation_days, use_percentile)?
        }
        Command::AuditGtRegime { from, to, scope, confirmation_days, min_days } => {
            commands::audit::handle_audit_gt_regime(&context, from, to, scope, confirmation_days, min_days)?
        }
        Command::AuditGtTransitions { from, to, scope, confirmation_days, min_days } => {
            commands::audit::handle_audit_gt_transitions(from, to, scope, confirmation_days, min_days)?
        }
        Command::AuditGtCandidates { from, to, scope } => commands::audit::handle_audit_gt_candidates(from, to, scope)?,
        Command::ValidateGtRegimes { from, to, scope, confirmation_days, min_days } => {
            commands::audit::handle_validate_gt_regimes(from, to, scope, confirmation_days, min_days)?
        }
        Command::AuditObservationLayer { from, to, scope } => commands::audit::handle_audit_observation_layer(&context, from, to, scope)?,
        Command::ReplayTrendSensitivity { from, to, scope } => commands::audit::handle_replay_trend_sensitivity(&context, from, to, scope)?,
        Command::GtSensitivityReplay { from, to, scope } => commands::audit::handle_gt_sensitivity_replay(&context, from, to, scope)?,
        Command::AuditAttribution { from, to, scope } => commands::audit::handle_audit_attribution(&context, from, to, scope)?,
        Command::AuditPersistenceSensitivity { from, to, scope } => commands::audit::handle_audit_persistence_sensitivity(&context, from, to, scope)?,
        Command::AuditMarketStructure { from, to, scope } => commands::audit::handle_audit_market_structure(&context, from, to, scope)?,
        Command::AuditRegimeAlignment { from, to, scope, drawdown_thresholds: _, tolerance_days } => {
            commands::audit::handle_audit_regime_alignment(&context, from, to, scope, tolerance_days)?
        }
        Command::AuditFactorAlignment { from, to, scope } => commands::audit::handle_audit_factor_alignment(&context, from, to, scope)?,
        Command::AuditFalsePositiveBreakdown { from, to, scope } => commands::audit::handle_audit_false_positive_breakdown(&context, from, to, scope)?,
        Command::AuditCounterfactualRegime { from, to, scope } => commands::audit::handle_audit_counterfactual_regime(&context, from, to, scope)?,
        Command::AuditEconomicReplay { from, to, scope } => commands::audit::handle_audit_economic_replay(&context, from, to, scope)?,
        Command::AuditEconomicAttribution { from, to, scope } => commands::audit::handle_audit_economic_attribution(&context, from, to, scope)?,
        Command::AuditParetoFrontier { from, to, scope } => commands::audit::handle_audit_pareto_frontier(&context, from, to, scope)?,
        Command::AuditEconomicRegimePrototype { from, to, scope } => commands::audit::handle_audit_economic_regime_prototype(&context, from, to, scope)?,
        Command::AuditDualLayerValidation { from, to, scope } => commands::audit::handle_audit_dual_layer_validation(&context, from, to, scope)?,
        Command::AuditAllocationPrototype { from, to, scope } => commands::audit::handle_audit_allocation_prototype(&context, from, to, scope)?,
        Command::AuditStateSignalDecomposition { from, to, scope } => commands::audit::handle_audit_state_signal_decomposition(&context, from, to, scope)?,
        Command::AuditStateTransitions { from, to, scope } => commands::audit::handle_audit_state_transitions(&context, from, to, scope)?,
        Command::AuditPersistenceFrontier { from, to, scope } => commands::audit::handle_audit_persistence_frontier(&context, from, to, scope)?,
        Command::AuditPersistenceMechanics { from, to, scope } => commands::audit::handle_audit_persistence_mechanics(&context, from, to, scope)?,
        Command::AuditEpisodeSurvival { from, to, scope } => commands::audit::handle_audit_episode_survival(&context, from, to, scope)?,
        Command::AuditLabelDistribution { from, to, scope } => commands::audit::handle_audit_label_distribution(&context, from, to, scope)?,
        Command::AuditScoreDistribution { from, to, scope } => commands::audit::handle_audit_score_distribution(&context, from, to, scope)?,
        Command::AuditWave8Revalidation { from, to, scope } => commands::audit::handle_audit_wave8_revalidation(&context, from, to, scope)?,
        Command::AuditGroundTruth { from, to, scope, persistence_days } => commands::audit::handle_audit_ground_truth(&context, from, to, scope, persistence_days)?,
        Command::AuditForwardReturnDistribution { from, to } => commands::audit::handle_audit_forward_return_distribution(&context, from, to)?,
        Command::GenerateGroundTruthLabels { from, to, horizon } => commands::audit::handle_generate_ground_truth_labels(&context, from, to, horizon)?,
        Command::AuditAlignmentRedesign { from, to, horizon } => commands::audit::handle_audit_alignment_redesign(&context, from, to, horizon)?,
        Command::AuditStatePersistenceEconomics { from, to } => commands::audit::handle_audit_state_persistence_economics(&context, from, to)?,
        Command::ValidateStateLayerGt { from, to } => commands::audit::handle_validate_state_layer_gt(&context, from, to)?,
        Command::RotationRanking { date, scope } => commands::audit::handle_rotation_ranking(&context, date, scope)?,
        Command::SymbolDiagnostics { symbol, date, scope } => commands::audit::handle_symbol_diagnostics(&context, symbol, date, scope)?,
        Command::SymbolScoreboard { date, scope } => commands::audit::handle_symbol_scoreboard(&context, date, scope)?,
        Command::AuditLeadLag { from, to } => commands::audit::handle_audit_lead_lag(&context, from, to)?,
    }
    Ok(())
}
