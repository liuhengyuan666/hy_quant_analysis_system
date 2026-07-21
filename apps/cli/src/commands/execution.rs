use anyhow::Result;
use app_service::{AppContext, ReportScope};
use chrono::Utc;
use serde_json;
use std::fs;
use std::path::PathBuf;

pub fn handle_portfolio_decision(context: &AppContext, scope: ReportScope) -> Result<()> {
    let decisions = context.analyze_preclose(scope)?;
    
    // Print table to stdout
    println!("Scope: {}", scope);
    println!("Date: {}", Utc::now().format("%Y-%m-%d"));
    println!("Candidates: {}\n", decisions.len());
    
    println!("{:<12} {:<12} {:<12} {}", "Symbol", "Signal", "State", "Reasons");
    println!("{}", "-".repeat(60));
    
    for d in &decisions {
        let reasons = if d.reasons.is_empty() {
            "(no pattern match)".to_string()
        } else {
            d.reasons.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", ")
        };
        println!(
            "{:<12} {:<12} {:<12} {}",
            d.symbol,
            "-", // signal not stored in ExecutionDecision
            d.state.as_str(),
            reasons
        );
    }
    
    // Write JSON sample to reports/execution-samples/
    let date_str = Utc::now().format("%Y-%m-%d").to_string();
    let output_dir = PathBuf::from("reports/execution-samples");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join(format!("{}.json", date_str));
    
    let json = serde_json::to_string_pretty(&decisions)?;
    fs::write(&output_path, json)?;
    
    println!("\nSample written to: {}", output_path.display());
    
    Ok(())
}
