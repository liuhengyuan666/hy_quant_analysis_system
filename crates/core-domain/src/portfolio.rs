//! Portfolio Context — user facts input layer (RV1 post-merge, P0).
//!
//! Pure config model for the user's real-world holdings
//! (`config/portfolio.toml`). NOT consumed by any engine, signal, or
//! decision path. Parse + validate only; no computation, no I/O.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetType {
    EtfLink,
    IndexFund,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum MappingQuality {
    Exact,
    Proxy,
    Unmapped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub fund_code: String,
    pub fund_name: String,
    pub asset_type: AssetType,
    #[serde(default)]
    pub underlying_symbol: String, // empty = none
    #[serde(default)]
    pub proxy_symbol: String, // empty = none
    pub mapping_quality: MappingQuality,
    pub cost_basis: f64,
    #[serde(default)]
    pub monitor_only: bool,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    #[serde(default)]
    pub positions: Vec<Position>,
}

impl PortfolioConfig {
    /// Validate user-fact consistency rules. Returns Err with a
    /// descriptive message on the first violation.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_enabled: Vec<&str> = Vec::new();
        for pos in &self.positions {
            if pos.cost_basis <= 0.0 {
                return Err(format!(
                    "position {}: cost_basis must be > 0.0 (got {})",
                    pos.fund_code, pos.cost_basis
                ));
            }
            match pos.mapping_quality {
                MappingQuality::Exact => {
                    if pos.underlying_symbol.is_empty() {
                        return Err(format!(
                            "position {}: EXACT mapping requires non-empty underlying_symbol",
                            pos.fund_code
                        ));
                    }
                    if !pos.proxy_symbol.is_empty() {
                        return Err(format!(
                            "position {}: EXACT mapping requires empty proxy_symbol (got '{}')",
                            pos.fund_code, pos.proxy_symbol
                        ));
                    }
                }
                MappingQuality::Proxy => {
                    if pos.proxy_symbol.is_empty() {
                        return Err(format!(
                            "position {}: PROXY mapping requires non-empty proxy_symbol",
                            pos.fund_code
                        ));
                    }
                    if !pos.underlying_symbol.is_empty() {
                        return Err(format!(
                            "position {}: PROXY mapping requires empty underlying_symbol (got '{}')",
                            pos.fund_code, pos.underlying_symbol
                        ));
                    }
                }
                MappingQuality::Unmapped => {
                    if !pos.underlying_symbol.is_empty() {
                        return Err(format!(
                            "position {}: UNMAPPED mapping requires empty underlying_symbol (got '{}')",
                            pos.fund_code, pos.underlying_symbol
                        ));
                    }
                }
            }
            if pos.enabled {
                if seen_enabled.contains(&pos.fund_code.as_str()) {
                    return Err(format!(
                        "duplicate fund_code '{}' across enabled positions",
                        pos.fund_code
                    ));
                }
                seen_enabled.push(pos.fund_code.as_str());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(
        fund_code: &str,
        mapping_quality: MappingQuality,
        underlying_symbol: &str,
        proxy_symbol: &str,
        cost_basis: f64,
    ) -> Position {
        Position {
            fund_code: fund_code.to_string(),
            fund_name: format!("fund-{fund_code}"),
            asset_type: AssetType::EtfLink,
            underlying_symbol: underlying_symbol.to_string(),
            proxy_symbol: proxy_symbol.to_string(),
            mapping_quality,
            cost_basis,
            monitor_only: false,
            enabled: true,
        }
    }

    #[test]
    fn real_portfolio_toml_parses_and_validates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("config")
            .join("portfolio.toml");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let config: PortfolioConfig =
            toml::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse portfolio.toml: {e}"));
        assert_eq!(config.positions.len(), 18, "expected 18 positions");
        config
            .validate()
            .unwrap_or_else(|e| panic!("portfolio.toml failed validation: {e}"));
    }

    #[test]
    fn exact_with_missing_underlying_symbol_fails() {
        let config = PortfolioConfig {
            positions: vec![position("F1", MappingQuality::Exact, "", "", 1.0)],
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("EXACT"), "unexpected error: {err}");
    }

    #[test]
    fn proxy_with_non_empty_underlying_symbol_fails() {
        let config = PortfolioConfig {
            positions: vec![position("F1", MappingQuality::Proxy, "000300", "513050", 1.0)],
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("PROXY"), "unexpected error: {err}");
    }

    #[test]
    fn proxy_with_empty_proxy_symbol_fails() {
        let config = PortfolioConfig {
            positions: vec![position("F1", MappingQuality::Proxy, "", "", 1.0)],
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("PROXY"), "unexpected error: {err}");
    }

    #[test]
    fn unmapped_with_non_empty_underlying_symbol_fails() {
        let config = PortfolioConfig {
            positions: vec![position("F1", MappingQuality::Unmapped, "000300", "", 1.0)],
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("UNMAPPED"), "unexpected error: {err}");
    }

    #[test]
    fn duplicate_fund_code_fails() {
        let config = PortfolioConfig {
            positions: vec![
                position("F1", MappingQuality::Exact, "000300", "", 1.0),
                position("F1", MappingQuality::Exact, "000300", "", 2.0),
            ],
        };
        let err = config.validate().unwrap_err();
        assert!(err.contains("duplicate"), "unexpected error: {err}");
    }

    #[test]
    fn duplicate_fund_code_allowed_when_one_disabled() {
        let mut disabled = position("F1", MappingQuality::Exact, "000300", "", 1.0);
        disabled.enabled = false;
        let config = PortfolioConfig {
            positions: vec![
                disabled,
                position("F1", MappingQuality::Exact, "000300", "", 2.0),
            ],
        };
        config.validate().expect("should validate: only enabled positions count for uniqueness");
    }

    #[test]
    fn non_positive_cost_basis_fails() {
        for bad in [0.0_f64, -1.5] {
            let config = PortfolioConfig {
                positions: vec![position("F1", MappingQuality::Exact, "000300", "", bad)],
            };
            let err = config.validate().unwrap_err();
            assert!(err.contains("cost_basis"), "unexpected error: {err}");
        }
    }
}
