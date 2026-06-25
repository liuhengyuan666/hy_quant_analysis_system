// market-store: persistence boundary for ClickHouse + SQLite.
// This file is a thin module hub. All domain logic lives in sibling modules;
// `pub use` re-exports preserve the original `market_store::fn_name` API.
pub mod core;
pub mod sqlite;
pub mod instruments;
pub mod bars;
pub mod indicators;
pub mod r#macro;
pub mod regime;
pub mod environment;
pub mod strategy;
pub mod rotation;
pub mod signals;
pub mod backtest;
pub mod reports;
pub mod dates;

pub use rusqlite::Connection;
pub use core::*;
pub use sqlite::*;
pub use instruments::*;
pub use bars::*;
pub use indicators::*;
pub use r#macro::*;
pub use regime::*;
pub use environment::*;
pub use strategy::*;
pub use rotation::*;
pub use signals::*;
pub use backtest::*;
pub use reports::*;
pub use dates::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        let dir = std::env::temp_dir();
        dir.join(format!("market_store_test_{pid}_{nanos}.db"))
            .to_string_lossy()
            .to_string()
    }

    fn temp_config() -> (String, StorageConfig) {
        let path = temp_db_path();
        let config = StorageConfig {
            sqlite_path: path.clone(),
            ..StorageConfig::default()
        };
        let conn = Connection::open(&path).expect("open temp sqlite");
        ensure_app_config_table(&conn).expect("create app_config table");
        ensure_credential_store_table(&conn).expect("create credential_store table");
        ensure_refresh_jobs_table(&conn).expect("create refresh_jobs table");
        ensure_user_preferences_table(&conn).expect("create user_preferences table");
        (path, config)
    }

    #[test]
    fn fetch_app_config_returns_none_for_missing_key() {
        let (_path, config) = temp_config();
        let result = fetch_app_config(&config, "nonexistent_key").expect("query should succeed");
        assert!(result.is_none());
    }

    #[test]
    fn app_config_round_trip() {
        let (_path, config) = temp_config();
        let key = "test.setting";
        let value = "hello world 42";

        insert_app_config(&config, key, value).expect("insert should succeed");
        let fetched = fetch_app_config(&config, key).expect("fetch should succeed");
        assert_eq!(fetched.as_deref(), Some(value));
    }

    #[test]
    fn app_config_upsert_updates_value() {
        let (_path, config) = temp_config();
        let key = "upsert.key";

        insert_app_config(&config, key, "first").expect("first insert");
        insert_app_config(&config, key, "second").expect("second insert");

        let fetched = fetch_app_config(&config, key).expect("fetch");
        assert_eq!(fetched.as_deref(), Some("second"));
    }

    #[test]
    fn credential_round_trip() {
        let (_path, config) = temp_config();
        let key = "api.test_token";
        let value = "secret-value-abc123";

        insert_credential(&config, key, value).expect("insert should succeed");
        let fetched = fetch_credential(&config, key).expect("fetch should succeed");
        assert_eq!(fetched.as_deref(), Some(value));
    }

    #[test]
    fn credential_upsert_updates_value() {
        let (_path, config) = temp_config();
        let key = "api.rotate_token";

        insert_credential(&config, key, "old_secret").expect("first insert");
        insert_credential(&config, key, "new_secret").expect("second insert");

        let fetched = fetch_credential(&config, key).expect("fetch");
        assert_eq!(fetched.as_deref(), Some("new_secret"));
    }

    #[test]
    fn app_config_sets_updated_at() {
        let (_path, config) = temp_config();
        let key = "ts.check";

        insert_app_config(&config, key, "value").expect("insert");

        let conn = Connection::open(&config.sqlite_path).expect("open for verification");
        let ts: String = conn
            .query_row(
                "SELECT updated_at FROM app_config WHERE config_key = ?1",
                [key],
                |row| row.get(0),
            )
            .expect("row should exist");
        assert!(!ts.is_empty(), "updated_at should be a non-empty timestamp");
    }

    #[test]
    fn credential_sets_updated_at() {
        let (_path, config) = temp_config();
        let key = "cred.ts";

        insert_credential(&config, key, "val").expect("insert");

        let conn = Connection::open(&config.sqlite_path).expect("open for verification");
        let ts: String = conn
            .query_row(
                "SELECT updated_at FROM credential_store WHERE credential_key = ?1",
                [key],
                |row| row.get(0),
            )
            .expect("row should exist");
        assert!(!ts.is_empty(), "updated_at should be a non-empty timestamp");
    }
}
