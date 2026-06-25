use anyhow::Result;
use app_service::AppContext;

pub fn handle_status(context: &AppContext) -> Result<()> {
    let status = context.status()?;
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

pub fn handle_init_storage(context: &AppContext) -> Result<()> {
    context.init_storage()?;
    println!("storage initialized");
    Ok(())
}

pub fn handle_seed_universe(context: &AppContext) -> Result<()> {
    let instruments = context.seed_universe()?;
    println!("{}", serde_json::to_string_pretty(&instruments)?);
    Ok(())
}
