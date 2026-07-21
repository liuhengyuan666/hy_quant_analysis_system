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

#[derive(Debug, Subcommand)]
enum ResearchCommand {
    /// SRD (Signal-Regime Divergence): observation tool
    Srd {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Historical date to analyze (defaults to latest available)")]
        date: Option<NaiveDate>,
    },
    /// Market Stretch analysis — measures market crowding/extremity in 4 dimensions
    Stretch {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Historical date to analyze (defaults to latest available)")]
        date: Option<NaiveDate>,
    },
    /// Conditional forward-return analytics — historical statistics only
    Analytics {
        #[arg(long)]
        condition: String,
        #[arg(long, default_value_t = 20)]
        horizon: usize,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Save the computed evidence to the workspace")]
        save_evidence: bool,
    },
    /// Quarterly Review — aggregate SRD/Stretch/Analytics over a window into a Markdown report
    Review {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Window start date (defaults to 90 days before --to)")]
        from: Option<NaiveDate>,
        #[arg(long, help = "Window end date (defaults to latest available daily date)")]
        to: Option<NaiveDate>,
        #[arg(long, help = "Output Markdown file path")]
        output: Option<std::path::PathBuf>,
    },
    /// V7 Workflow — Observe: aggregate SRD, Stretch, Analytics, and Health into a single observation report
    Observe {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Historical date to analyze (defaults to latest available)")]
        date: Option<NaiveDate>,
        #[arg(long, default_value_t = String::from("srd-strong"), help = "Condition for analytics")]
        condition: String,
        #[arg(long, default_value_t = 20)]
        horizon: usize,
        #[arg(long, help = "Output Markdown file path")]
        output: Option<std::path::PathBuf>,
    },
    /// Market Confirmation analysis — quantifies Trend, Participation, Risk confirmation
    Confirmation {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Historical date to analyze (defaults to latest available)")]
        date: Option<NaiveDate>,
    },
    /// Recovery Index analysis — measures market recovery from drawdown/stress
    Recovery {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Historical date to analyze (defaults to latest available)")]
        date: Option<NaiveDate>,
    },
    /// V7.2C Research Calibration — run the Research Calibration framework over a historical window
    Calibration {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Window start date (defaults to last 60 trading days)")]
        from: Option<NaiveDate>,
        #[arg(long, help = "Window end date (defaults to latest available)")]
        to: Option<NaiveDate>,
        #[arg(long, default_value_t = 20)]
        horizon: usize,
        #[arg(long, default_value_t = 5, help = "Number of top matches to return")]
        top_n: usize,
        #[arg(long, default_value_t = 252, help = "Number of historical trading days to search")]
        lookback: usize,
    },
    /// V7 Workflow — Replay: run historical analytics across conditions and horizons, saving Evidence to workspace
    Replay {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Window start date (defaults to 90 days before --to)")]
        from: Option<NaiveDate>,
        #[arg(long, help = "Window end date (defaults to latest available)")]
        to: Option<NaiveDate>,
        #[arg(long, help = "Output directory for replay index files", default_value_t = String::from("shadow-production/historical-replay"))]
        output_dir: String,
    },
    /// V7.2B Historical Analogue Search — find similar market conditions and profile outcomes
    Analogues {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Target date (defaults to latest available)")]
        date: Option<NaiveDate>,
        #[arg(long, default_value_t = 20)]
        horizon: usize,
        #[arg(long, default_value_t = 5, help = "Number of top matches to return")]
        top_n: usize,
        #[arg(long, default_value_t = 252, help = "Number of historical trading days to search")]
        lookback: usize,
    },
    /// V7.4 / ADR-078 Research Explanation — explain why a condition performs differently across regimes
    Explain {
        #[arg(long, help = "Condition to explain (e.g., srd-strong)")]
        condition: String,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Historical date to analyze (defaults to latest available)")]
        date: Option<NaiveDate>,
        #[arg(long, default_value_t = 20, help = "Forward-return horizon for evidence")]
        horizon: usize,
    },
    /// V7.3 Research Consensus — synthesize Observation, Evolution, and Historical Evidence into a research interpretation
    Consensus {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Target date (defaults to latest available)")]
        date: Option<NaiveDate>,
        #[arg(long, default_value_t = 20)]
        horizon: usize,
        #[arg(long, default_value_t = 5, help = "Number of top matches to return")]
        top_n: usize,
        #[arg(long, default_value_t = 252, help = "Number of historical trading days to search")]
        lookback: usize,
    },
}

#[derive(Debug, Parser)]
#[command(name = "quant-cli")]
#[command(about = "Daily Portfolio Decision Assistant")]
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

    // ── Daily Pipeline ──────────────────────────────────────────────

    /// Full pipeline refresh: ingest → indicators → macro → rotation → strategy → signals → backtests
    MarketRefresh {
        #[arg(long)]
        to: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = true)]
        run_backtests: bool,
    },
    /// One-stop daily analysis: Integrity Gate → Research Evidence → Strategy Scores → Risk → Portfolio Action
    DailyAnalysis {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Export daily report
    DailyReport {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        /// Export concise daily report (Insight First format)
        #[arg(long, default_value_t = false)]
        concise: bool,
    },

    // ── Deep Analysis ───────────────────────────────────────────────

    /// Multi-strategy independent scoring + scenario comparison + attribution
    StrategyPerspectives {
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value = "detail", help = "Mode: scoreboard, detail")]
        mode: String,
        #[arg(long, help = "Scenario name (from config/scenarios.toml)")]
        scenario: Option<String>,
    },
    /// Portfolio action recommendation for today
    PortfolioDecision {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Check provider health and data completeness
    DataHealth,

    // ── Evidence & Validation ───────────────────────────────────────

    /// View Evidence Asset status in workspace
    EvidenceStatus,
    /// Run calibration baseline validation
    ValidationCheck {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Window start date (defaults to last 60 trading days)")]
        from: Option<NaiveDate>,
        #[arg(long, help = "Window end date (defaults to latest available)")]
        to: Option<NaiveDate>,
    },
    /// Run historical analytics replay across conditions and horizons
    HistoricalReplay {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, help = "Window start date (defaults to 90 days before --to)")]
        from: Option<NaiveDate>,
        #[arg(long, help = "Window end date (defaults to latest available)")]
        to: Option<NaiveDate>,
        #[arg(long, help = "Output directory for replay index files", default_value_t = String::from("shadow-production/historical-replay"))]
        output_dir: String,
    },

    // ── LLM ─────────────────────────────────────────────────────────

    /// LLM market analysis
    LlmAnalyze {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value = "market_story")]
        action: String,
    },

    // ── Hidden / Advanced ───────────────────────────────────────────

    /// Research surface commands (observation-only tools)
    #[command(subcommand)]
    Research(ResearchCommand),
    /// Pipeline dates diagnostic
    PipelineDates {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Explain why latest gate hasn't advanced
    ExplainLatestGate {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Single-symbol signal attribution breakdown
    SymbolDiagnostics {
        #[arg(long)]
        symbol: String,
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Full symbol scoreboard
    SymbolScoreboard {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Rotation ranking
    RotationRanking {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Dashboard snapshot
    DashboardSnapshot {
        #[arg(long)]
        date: Option<NaiveDate>,
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Dashboard dates
    DashboardDates {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
    },
    /// Run backtest
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
    /// Sync and export in one step
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

    // ── Config ──────────────────────────────────────────────────────

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
    ExportDataHealthReport,
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
    ShowLlmConfig {
        #[arg(long)]
        validate: bool,
    },
    MigrateLlmConfig {
        #[arg(long)]
        force: bool,
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

        // Daily pipeline
        Command::MarketRefresh { to, scope, run_backtests } => commands::pipeline::handle_refresh_all(&context, to, scope, run_backtests, cli.quiet)?,
        Command::DailyAnalysis { scope } => commands::pipeline::handle_daily_analysis(&context, scope, cli.quiet)?,
        Command::DailyReport { date, scope, concise } => commands::dashboard::handle_export_report(&context, date, scope, concise)?,

        // Deep analysis
        Command::StrategyPerspectives { symbol, date, scope, mode, scenario } => commands::research::handle_strategy_perspectives(&context, symbol, date, scope, mode, scenario)?,
        Command::PortfolioDecision { scope } => commands::execution::handle_portfolio_decision(&context, scope.into())?,
        Command::DataHealth => commands::diagnostics::handle_data_health(&context)?,

        // Evidence & validation
        Command::EvidenceStatus => commands::research::handle_evidence_status()?,
        Command::ValidationCheck { scope, from, to } => commands::research::handle_validation_check(&context, scope, from, to)?,
        Command::HistoricalReplay { scope, from, to, output_dir } => commands::research::handle_historical_replay(&context, scope, from, to, output_dir)?,

        // LLM
        Command::LlmAnalyze { scope, action } => commands::llm::handle_llm_analyze(&context, scope, action, cli.quiet)?,

        // Research subcommands
        Command::Research(cmd) => match cmd {
            ResearchCommand::Srd { scope, date } => commands::research::handle_research_srd(&context, scope, date)?,
            ResearchCommand::Stretch { scope, date } => commands::research::handle_research_stretch(&context, scope, date)?,
            ResearchCommand::Analytics { condition, horizon, scope, save_evidence } => {
                commands::research::handle_research_analytics(&context, condition, horizon, scope, save_evidence)?
            }
            ResearchCommand::Review { scope, from, to, output } => {
                commands::research::handle_research_review(&context, scope, from, to, output)?
            }
            ResearchCommand::Observe { scope, date, condition, horizon, output } => {
                commands::research::handle_research_observe(&context, scope, date, condition, horizon, output)?
            }
            ResearchCommand::Confirmation { scope, date } => commands::research::handle_research_confirmation(&context, scope, date)?,
            ResearchCommand::Recovery { scope, date } => commands::research::handle_research_recovery(&context, scope, date)?,
            ResearchCommand::Calibration { scope, from, to, horizon, top_n, lookback } => {
                commands::research::handle_research_calibration(&context, scope, from, to, horizon, top_n, lookback)?
            }
            ResearchCommand::Replay { scope, from, to, output_dir } => {
                commands::research::handle_research_replay(&context, scope, from, to, output_dir)?
            }
            ResearchCommand::Analogues { scope, date, horizon, top_n, lookback } => {
                commands::research::handle_research_analogues(&context, scope, date, horizon, top_n, lookback)?
            }
            ResearchCommand::Consensus { scope, date, horizon, top_n, lookback } => {
                commands::research::handle_research_consensus(&context, scope, date, horizon, top_n, lookback)?
            }
            ResearchCommand::Explain { condition, scope, date, horizon } => {
                commands::research::handle_research_explain(&context, condition, scope, date, horizon)?
            }
        },

        // Hidden/advanced
        Command::PipelineDates { scope } => commands::diagnostics::handle_pipeline_dates(&context, scope)?,
        Command::ExplainLatestGate { scope } => commands::diagnostics::handle_explain_latest_gate(&context, scope)?,
        Command::SymbolDiagnostics { symbol, date, scope } => commands::audit::handle_symbol_diagnostics(&context, symbol, date, scope)?,
        Command::SymbolScoreboard { date, scope } => commands::audit::handle_symbol_scoreboard(&context, date, scope)?,
        Command::RotationRanking { date, scope } => commands::audit::handle_rotation_ranking(&context, date, scope)?,
        Command::DashboardSnapshot { date, scope } => commands::dashboard::handle_dashboard_snapshot(&context, date, scope)?,
        Command::DashboardDates { scope } => commands::dashboard::handle_dashboard_dates(&context, scope)?,
        Command::RunBacktest { initial_capital, max_holdings, fee_rate, slippage_rate, scope, use_state_sizing, max_drawdown } => {
            commands::backtest::handle_run_backtest(&context, initial_capital, max_holdings, fee_rate, slippage_rate, scope, use_state_sizing, max_drawdown)?
        }
        Command::SyncAndExport { date, scope, to, run_backtests } => commands::dashboard::handle_sync_and_export(&context, date, scope, to, run_backtests, cli.quiet)?,

        // Config
        Command::IngestDaily { from, to } => commands::pipeline::handle_ingest_daily(&context, from, to, cli.quiet)?,
        Command::ComputeIndicators => commands::pipeline::handle_compute_indicators(&context, cli.quiet)?,
        Command::ComputeMacro { from, to } => commands::pipeline::handle_compute_macro(&context, from, to, cli.quiet)?,
        Command::ComputeRotation => commands::pipeline::handle_compute_rotation(&context, cli.quiet)?,
        Command::ComputeStrategyPreferences => commands::pipeline::handle_compute_strategy_preferences(&context, cli.quiet)?,
        Command::ComputeSignals => commands::pipeline::handle_compute_signals(&context, cli.quiet)?,
        Command::ExportDataHealthReport => commands::dashboard::handle_export_data_health_report(&context)?,
        Command::ResearchContext { scope } => commands::dashboard::handle_research_context(&context, scope)?,
        Command::SetLlmConfig { base_url, model, timeout_secs } => commands::llm::handle_set_llm_config(&context, base_url, model, timeout_secs)?,
        Command::SetLlmApiKey { key } => commands::llm::handle_set_llm_api_key(&context, key)?,
        Command::SetFredConfig { enabled, disabled, api_key } => commands::llm::handle_set_fred_config(&context, enabled, disabled, api_key)?,
        Command::ShowFredConfig => commands::llm::handle_show_fred_config(&context)?,
        Command::ShowLlmConfig { validate } => commands::llm::handle_show_llm_config(&context, validate)?,
        Command::MigrateLlmConfig { force } => commands::llm::handle_migrate_llm_config(&context, force)?,
    }
    Ok(())
}
