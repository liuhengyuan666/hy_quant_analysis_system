use anyhow::{Context, Result};
use app_service::{pipeline_stages, AppContext, ReportScope};
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand, ValueEnum};
use market_store::StorageConfig;

fn stage_label(stage: &str) -> String {
    let total = pipeline_stages::ALL.len();
    match pipeline_stages::ALL.iter().position(|&s| s == stage) {
        Some(idx) => format!("[{}/{}] {}", idx + 1, total, stage),
        None => stage.to_string(),
    }
}

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
    AnalyzeWithLlm {
        #[arg(long, value_enum, default_value_t = ReportScopeArg::Global)]
        scope: ReportScopeArg,
        #[arg(long)]
        date: Option<NaiveDate>,
    },
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
            let progress_fn = |msg: &str| eprintln!("[ingest] {}", msg);
            let result = if cli.quiet {
                context.ingest_daily(from, to, None)?
            } else {
                context.ingest_daily(from, to, Some(&progress_fn))?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeIndicators => {
            let label = stage_label(pipeline_stages::STAGE_INDICATORS);
            let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
            let result = if cli.quiet {
                context.compute_indicators(None)?
            } else {
                context.compute_indicators(Some(&progress_fn))?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeMacro { from, to } => {
            let label = stage_label(pipeline_stages::STAGE_MACRO);
            let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
            let result = if cli.quiet {
                context.compute_macro_regime(from, to, None)?
            } else {
                context.compute_macro_regime(from, to, Some(&progress_fn))?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeRotation => {
            let label = stage_label(pipeline_stages::STAGE_ROTATION);
            let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
            let result = if cli.quiet {
                context.compute_rotation(None)?
            } else {
                context.compute_rotation(Some(&progress_fn))?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeStrategyPreferences => {
            let label = stage_label(pipeline_stages::STAGE_STRATEGY);
            let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
            let result = if cli.quiet {
                context.compute_strategy_preferences(None)?
            } else {
                context.compute_strategy_preferences(Some(&progress_fn))?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ComputeSignals => {
            let label = stage_label(pipeline_stages::STAGE_SIGNALS);
            let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
            let result = if cli.quiet {
                context.compute_signals(None)?
            } else {
                context.compute_signals(Some(&progress_fn))?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::RefreshAll {
            to,
            scope,
            run_backtests,
        } => {
            let progress_callback: Option<Box<dyn Fn(&str) + Send>> = if cli.quiet {
                None
            } else {
                Some(Box::new(|msg: &str| {
                    eprintln!("[refresh] {}", msg);
                }))
            };
            let result = context.refresh_pipeline(
                to.unwrap_or_else(|| Local::now().date_naive()),
                scope.into(),
                run_backtests,
                None,
                None,
                progress_callback,
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
        Command::SyncAndExport {
            date,
            scope,
            to,
            run_backtests,
        } => {
            if !cli.quiet {
                eprintln!("[sync-and-export] Starting...");
            }
            let progress_callback: Option<Box<dyn Fn(&str) + Send>> = if cli.quiet {
                None
            } else {
                Some(Box::new(|msg: &str| {
                    eprintln!("[sync-and-export] {}", msg);
                }))
            };
            let result = context.sync_and_export(
                date,
                to.unwrap_or_else(|| Local::now().date_naive()),
                scope.into(),
                run_backtests,
                progress_callback,
            )?;
            if !cli.quiet {
                eprintln!("[sync-and-export] Done.");
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ResearchContext { scope } => {
            let result = context.research_context(scope.into())?;
            let features = context.research_features(scope.into())?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "context": result,
                "features": features,
            }))?);
        }
        Command::SetLlmConfig {
            base_url,
            model,
            timeout_secs,
        } => {
            context.set_llm_config(&base_url, &model, timeout_secs)?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "status": "ok",
                "base_url": base_url,
                "model": model,
                "timeout_secs": timeout_secs,
            }))?);
        }
        Command::SetLlmApiKey { key } => {
            context.set_llm_api_key(&key)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "message": "LLM API key stored successfully"
                }))?
            );
        }
        Command::AnalyzeWithLlm { scope, date } => {
            let report_date = match date {
                Some(d) => d,
                None => {
                    let dates =
                        context.dashboard_available_dates_with_scope(scope.into())?;
                    let latest = dates.first().context(
                        "no dashboard dates available; run refresh-all first",
                    )?;
                    NaiveDate::parse_from_str(latest, "%Y-%m-%d")
                        .context("failed to parse latest dashboard date")?
                }
            };
            if !cli.quiet {
                eprintln!("[analyze-with-llm] Analyzing report for {report_date}...");
            }
            let result = context.analyze_report_with_llm(report_date, scope.into())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
