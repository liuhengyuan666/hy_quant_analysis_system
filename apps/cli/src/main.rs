use anyhow::Result;
use app_service::{AppContext, ReportScope};
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand, ValueEnum};
use market_store::StorageConfig;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReportScopeArg {
    Global,
    Cn,
    Hk,
}

impl From<ReportScopeArg> for ReportScope {
    fn from(value: ReportScopeArg) -> Self {
        match value {
            ReportScopeArg::Global => ReportScope::Global,
            ReportScopeArg::Cn => ReportScope::Cn,
            ReportScopeArg::Hk => ReportScope::Hk,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "quant-cli")]
#[command(about = "Rust quant analysis system CLI")]
struct Cli {
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
        #[arg(long, help = "Scope used for latest-date diagnostics and gate explanation only")]
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long, default_value_t = true, help = "Whether to include standard-scope backtests in the aggregate refresh (default: true)")]
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
    },
    ExportDataHealthReport,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    let context = AppContext::new(StorageConfig::default());

    match cli.command {
        Command::Status => {
            let status = context.status()?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        Command::InitStorage => {
            context.init_storage()?;
            println!("storage initialized");
        }
        Command::SeedUniverse => {
            let instruments = context.seed_universe()?;
            println!("{}", serde_json::to_string_pretty(&instruments)?);
        }
        Command::IngestDaily { from, to } => {
            let result = context.ingest_daily(from, to)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeIndicators => {
            let result = context.compute_indicators()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeMacro { from, to } => {
            let result = context.compute_macro_regime(from, to)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeRotation => {
            let result = context.compute_rotation()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeStrategyPreferences => {
            let result = context.compute_strategy_preferences()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeSignals => {
            let result = context.compute_signals()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::RefreshAll {
            to,
            scope,
            run_backtests,
        } => {
            let result = context.refresh_pipeline(
                to.unwrap_or_else(|| Local::now().date_naive()),
                scope.into(),
                run_backtests,
                None,
                None,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if !result.success {
                std::process::exit(1);
            }
        }
        Command::ExplainLatestGate { scope } => {
            let result = context.explain_latest_gate(scope.into())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::PipelineDates { scope } => {
            let result = context.pipeline_date_diagnostics_with_scope(scope.into())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CheckDataHealth => {
            let result = context.check_data_health()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::RunBacktest {
            initial_capital,
            max_holdings,
            fee_rate,
            slippage_rate,
            scope,
            use_state_sizing,
            max_drawdown,
        } => {
            let result = context.run_backtest(
                initial_capital,
                max_holdings,
                fee_rate,
                slippage_rate,
                scope.into(),
                use_state_sizing,
                max_drawdown,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::DashboardSnapshot { date, scope } => {
            let result = context.dashboard_snapshot_with_scope(date, scope.into())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::DashboardDates { scope } => {
            let result = context.dashboard_available_dates_with_scope(scope.into())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ExportReport { date, scope } => {
            let result = context.export_report_with_scope(date, scope.into())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ExportDataHealthReport => {
            let result = context.export_data_health_report()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
