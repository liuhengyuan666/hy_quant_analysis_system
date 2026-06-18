use anyhow::Result;

/// Determines whether `sync_and_export` should attempt a pipeline refresh.
/// Returns `true` when the gate is not yet advanced (behind or unknown).
pub(crate) fn sync_gate_needs_refresh(gate_before_advanced: Option<bool>) -> bool {
    gate_before_advanced != Some(true)
}

/// Validates that a refresh pipeline result is acceptable for proceeding.
/// Returns `Ok(())` if refresh succeeded, `Err` with blocking alerts if it failed.
pub(crate) fn validate_sync_refresh_result(success: bool, blocking_alerts: &[String]) -> Result<()> {
    if !success {
        anyhow::bail!(
            "sync-and-export aborted because refresh_pipeline failed. {}",
            blocking_alerts.join(" | ")
        );
    }
    Ok(())
}
