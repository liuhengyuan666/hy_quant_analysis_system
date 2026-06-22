use crate::types::{ExecutionDecision, IntradaySnapshot, SkipReason};
use chrono::Utc;
use std::collections::HashMap;

/// Tencent API response for a single symbol.
/// Format: v_sz000300="1~000300~...~price~prev_close~open~high~low~volume~...";
#[derive(Debug, Clone)]
struct TencentQuote {
    symbol: String,
    #[allow(dead_code)]
    name: String,
    price: f64,
    prev_close: f64,
    open: f64,
    high: f64,
    low: f64,
    volume: f64,
}

impl TencentQuote {
    fn parse_line(line: &str) -> Option<Self> {
        // Expected prefix: v_sz000300="1~000300~name~..."
        let content = line.strip_prefix("v_")?;
        let (_code_part, data_part) = content.split_once("=\"")?;
        let data = data_part.trim_end_matches("\";");
        let parts: Vec<&str> = data.split('~').collect();
        if parts.len() < 45 {
            return None;
        }
        let symbol = parts[2].to_string();
        let name = parts[1].to_string();
        let price = parts[3].parse::<f64>().ok()?;
        let prev_close = parts[4].parse::<f64>().ok()?;
        let open = parts[5].parse::<f64>().ok()?;
        let high = parts[6].parse::<f64>().ok()?;
        let low = parts[33].parse::<f64>().ok()?;
        let volume = parts[36].parse::<f64>().ok()?;
        Some(Self {
            symbol,
            name,
            price,
            prev_close,
            open,
            high,
            low,
            volume,
        })
    }
}

fn compute_close_position(high: f64, low: f64, close: f64) -> f64 {
    if (high - low).abs() < f64::EPSILON {
        return 0.5;
    }
    ((close - low) / (high - low)).clamp(0.0, 1.0)
}

/// Fetch intraday snapshots from Tencent API for a batch of symbols.
/// Returns a HashMap keyed by symbol; missing symbols are omitted.
/// On total failure, returns an empty map.
pub fn fetch_tencent_snapshots(
    symbols: &[impl AsRef<str>],
) -> Result<HashMap<String, IntradaySnapshot>, FetchError> {
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }

    // Build Tencent symbol codes: correct prefix based on exchange rules
    let tencent_codes: Vec<String> = symbols
        .iter()
        .map(|s| {
            let s = s.as_ref();
            if s.starts_with("000") || s.starts_with('6') || s.starts_with('5') || s.starts_with('9') {
                format!("sh{}", s)
            } else {
                format!("sz{}", s)
            }
        })
        .collect();

    let url = format!(
        "https://qt.gtimg.cn/q={}",
        tencent_codes.join(",")
    );

    let response = attohttpc::get(&url).send().map_err(FetchError::Http)?;
    if !response.is_success() {
        return Err(FetchError::HttpStatus(response.status().as_u16()));
    }
    let bytes = response.bytes().map_err(FetchError::Http)?;
    // Tencent API returns GBK (GB18030) encoded response, not UTF-8
    let (decoded, _, had_errors) = encoding_rs::GB18030.decode(&bytes);
    let body = if had_errors {
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        decoded.into_owned()
    };

    let mut snapshots = HashMap::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(quote) = TencentQuote::parse_line(line) else {
            continue;
        };
        let today_return = if quote.prev_close > 0.0 {
            (quote.price / quote.prev_close) - 1.0
        } else {
            0.0
        };
        let close_position = compute_close_position(quote.high, quote.low, quote.price);
        let snapshot = IntradaySnapshot {
            symbol: quote.symbol.clone(),
            ts: Utc::now(),
            today_return,
            distance_ma5: 0.0, // placeholder; will be enriched by caller
            volume_ratio: 0.0, // placeholder; will be enriched by caller
            close_position,
            open: quote.open,
            high: quote.high,
            low: quote.low,
            close: quote.price,
            volume: quote.volume,
            prev_close: quote.prev_close,
        };
        snapshots.insert(quote.symbol, snapshot);
    }

    Ok(snapshots)
}

/// Enrich snapshots with MA5 and volume ratio using historical data from the store.
/// Takes the raw snapshots and a map of (symbol, (ma5, vol_ma20)) to fill in.
pub fn enrich_snapshots(
    snapshots: &mut HashMap<String, IntradaySnapshot>,
    ma5_map: &HashMap<String, f64>,
    vol_ma20_map: &HashMap<String, f64>,
) {
    for (symbol, snapshot) in snapshots.iter_mut() {
        if let Some(&ma5) = ma5_map.get(symbol) {
            if ma5 > 0.0 {
                snapshot.distance_ma5 = (snapshot.close / ma5) - 1.0;
            }
        }
        if let Some(&vol_ma20) = vol_ma20_map.get(symbol) {
            if vol_ma20 > 0.0 {
                snapshot.volume_ratio = snapshot.volume / vol_ma20;
            }
        }
    }
}

/// Total fetch pipeline: fetch + enrich + return decisions for all symbols.
/// On any failure, returns a single "global" skip decision so CLI never panics.
pub fn fetch_and_analyze(
    symbols: &[impl AsRef<str>],
    ma5_map: &HashMap<String, f64>,
    vol_ma20_map: &HashMap<String, f64>,
) -> Vec<ExecutionDecision> {
    let mut snapshots = match fetch_tencent_snapshots(symbols) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Tencent snapshot fetch failed: {}", e);
            // Return Skip for every symbol
            return symbols
                .iter()
                .map(|s| ExecutionDecision::skipped(s.as_ref(), SkipReason::DataUnavailable))
                .collect();
        }
    };

    enrich_snapshots(&mut snapshots, ma5_map, vol_ma20_map);

    crate::engine::analyze_batch(&snapshots)
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("HTTP error: {0}")]
    Http(#[from] attohttpc::Error),
    #[error("HTTP status {0}")]
    HttpStatus(u16),
}
