use anyhow::{Context, Result};
use core_domain::RefreshJobRecord;

use crate::core::*;

pub fn insert_refresh_job(config: &StorageConfig, job: &RefreshJobRecord) -> Result<()> {
    let connection = sqlite_connection(config)?;
    connection
        .execute(
            "INSERT INTO refresh_jobs (id, started_at, finished_at, status, stages_json, last_successful_stage, error, refresh_from, refresh_to)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &job.id,
                &job.started_at,
                &job.finished_at,
                &job.status,
                &job.stages_json,
                &job.last_successful_stage,
                &job.error,
                &job.refresh_from,
                &job.refresh_to,
            ],
        )
        .context("failed to insert refresh job")?;
    Ok(())
}

pub fn update_refresh_job(config: &StorageConfig, job: &RefreshJobRecord) -> Result<()> {
    let connection = sqlite_connection(config)?;
    connection
        .execute(
            "UPDATE refresh_jobs
             SET started_at = ?2,
                 finished_at = ?3,
                 status = ?4,
                 stages_json = ?5,
                 last_successful_stage = ?6,
                 error = ?7,
                 refresh_from = ?8,
                 refresh_to = ?9
             WHERE id = ?1",
            rusqlite::params![
                &job.id,
                &job.started_at,
                &job.finished_at,
                &job.status,
                &job.stages_json,
                &job.last_successful_stage,
                &job.error,
                &job.refresh_from,
                &job.refresh_to,
            ],
        )
        .context("failed to update refresh job")?;
    Ok(())
}

pub fn fetch_latest_refresh_job(config: &StorageConfig) -> Result<Option<RefreshJobRecord>> {
    let mut jobs = fetch_refresh_jobs(config, 1)?;
    Ok(jobs.pop())
}

pub fn fetch_refresh_jobs(config: &StorageConfig, limit: usize) -> Result<Vec<RefreshJobRecord>> {
    let connection = sqlite_connection(config)?;
    let limit = i64::try_from(limit).unwrap_or(i64::MAX).max(0);
    let mut statement = connection
        .prepare(
            "SELECT id, started_at, finished_at, status, stages_json, last_successful_stage, error, refresh_from, refresh_to
             FROM refresh_jobs
             ORDER BY started_at DESC
             LIMIT ?1",
        )
        .context("failed to prepare refresh jobs query")?;
    let rows = statement
        .query_map([limit], |row| {
            Ok(RefreshJobRecord {
                id: row.get(0)?,
                started_at: row.get(1)?,
                finished_at: row.get(2)?,
                status: row.get(3)?,
                stages_json: row.get(4)?,
                last_successful_stage: row.get(5)?,
                error: row.get(6)?,
                refresh_from: row.get(7)?,
                refresh_to: row.get(8)?,
            })
        })
        .context("failed to query refresh jobs")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to decode refresh jobs")
}

pub fn get_user_preference(config: &StorageConfig, key: &str) -> Result<Option<String>> {
    let connection = sqlite_connection(config)?;
    let mut statement = connection
        .prepare("SELECT value FROM user_preferences WHERE key = ?1")
        .context("failed to prepare get_user_preference query")?;
    let mut rows = statement
        .query_map([key], |row| row.get::<_, String>(0))
        .context("failed to query user preference")?;
    match rows.next() {
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(error)) => Err(error).context("failed to read user preference value"),
        None => Ok(None),
    }
}

pub fn set_user_preference(config: &StorageConfig, key: &str, value: &str) -> Result<()> {
    let connection = sqlite_connection(config)?;
    let now = chrono::Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO user_preferences (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            rusqlite::params![key, value, now],
        )
        .context("failed to set user preference")?;
    Ok(())
}

pub fn fetch_app_config(config: &StorageConfig, key: &str) -> Result<Option<String>> {
    let connection = sqlite_connection(config)?;
    let mut statement = connection
        .prepare("SELECT config_value FROM app_config WHERE config_key = ?1")
        .context("failed to prepare fetch_app_config query")?;
    let mut rows = statement
        .query_map([key], |row| row.get::<_, String>(0))
        .context("failed to query app_config")?;
    match rows.next() {
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(error)) => Err(error).context("failed to read app_config value"),
        None => Ok(None),
    }
}

pub fn insert_app_config(config: &StorageConfig, key: &str, value: &str) -> Result<()> {
    let connection = sqlite_connection(config)?;
    let now = chrono::Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO app_config (config_key, config_value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(config_key) DO UPDATE SET config_value = ?2, updated_at = ?3",
            rusqlite::params![key, value, now],
        )
        .context("failed to insert app_config")?;
    Ok(())
}

pub fn fetch_credential(config: &StorageConfig, key: &str) -> Result<Option<String>> {
    let connection = sqlite_connection(config)?;
    let mut statement = connection
        .prepare("SELECT credential_value FROM credential_store WHERE credential_key = ?1")
        .context("failed to prepare fetch_credential query")?;
    let mut rows = statement
        .query_map([key], |row| row.get::<_, String>(0))
        .context("failed to query credential_store")?;
    match rows.next() {
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(error)) => Err(error).context("failed to read credential value"),
        None => Ok(None),
    }
}

pub fn insert_credential(config: &StorageConfig, key: &str, value: &str) -> Result<()> {
    let connection = sqlite_connection(config)?;
    let now = chrono::Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO credential_store (credential_key, credential_value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(credential_key) DO UPDATE SET credential_value = ?2, updated_at = ?3",
            rusqlite::params![key, value, now],
        )
        .context("failed to insert credential")?;
    Ok(())
}

pub fn get_all_user_preferences(
    config: &StorageConfig,
) -> Result<std::collections::BTreeMap<String, String>> {
    let connection = sqlite_connection(config)?;
    let mut statement = connection
        .prepare("SELECT key, value FROM user_preferences")
        .context("failed to prepare get_all_user_preferences query")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .context("failed to query all user preferences")?;
    let mut map = std::collections::BTreeMap::new();
    for row in rows {
        let (key, value) = row.context("failed to read user preference row")?;
        map.insert(key, value);
    }
    Ok(map)
}
