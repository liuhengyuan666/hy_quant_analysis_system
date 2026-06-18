use anyhow::Result;
use app_service::AppContext;
use crate::ReportScopeArg;

pub fn handle_run_backtest(
    context: &AppContext,
    initial_capital: f64,
    max_holdings: usize,
    fee_rate: f64,
    slippage_rate: f64,
    scope: ReportScopeArg,
    use_state_sizing: bool,
    max_drawdown: Option<f64>,
) -> Result<()> {
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
    Ok(())
}
