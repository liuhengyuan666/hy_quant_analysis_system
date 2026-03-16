use anyhow::Result;
use app_service::AppContext;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use market_store::StorageConfig;

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
    },
    DashboardSnapshot {
        #[arg(long)]
        date: Option<NaiveDate>,
    },
    DashboardDates,
    ExportReport {
        #[arg(long)]
        date: Option<NaiveDate>,
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
        Command::CheckDataHealth => {
            let result = context.check_data_health()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::RunBacktest {
            initial_capital,
            max_holdings,
            fee_rate,
            slippage_rate,
        } => {
            let result =
                context.run_backtest(initial_capital, max_holdings, fee_rate, slippage_rate)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::DashboardSnapshot { date } => {
            let result = context.dashboard_snapshot(date)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::DashboardDates => {
            let result = context.dashboard_available_dates()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ExportReport { date } => {
            let result = context.export_report(date)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::ExportDataHealthReport => {
            let result = context.export_data_health_report()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
