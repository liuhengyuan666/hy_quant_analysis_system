use anyhow::Result;
use chrono::NaiveDate;
use core_domain::{DailyBar, Instrument, InstrumentType, Market};
use macro_engine::MacroFactorSeries;
use attohttpc::Session;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
#[derive(Debug, Clone)]
pub struct MacroFetchOutcome {
    pub series: MacroFactorSeries,
    pub transport: String,
}

#[derive(Debug, Clone, Copy)]
enum DailyAdjustmentMode {
    Forward,
}

impl DailyAdjustmentMode {
    fn eastmoney_param(self) -> &'static str {
        match self {
            Self::Forward => "1",
        }
    }

    fn tencent_param(self) -> &'static str {
        match self {
            Self::Forward => "qfq",
        }
    }
}

const CANONICAL_DAILY_ADJUSTMENT: DailyAdjustmentMode = DailyAdjustmentMode::Forward;

fn http_client() -> &'static Session {
    static CACHED: OnceLock<Session> = OnceLock::new();
    CACHED.get_or_init(|| Session::new())
}

fn fetch_text_via_curl(url: &str) -> Result<String> {
    let output = Command::new("curl.exe")
        .args(["-L", "-s", "--noproxy", "*", url])
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "curl fallback failed (exit code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        anyhow::bail!("curl fallback returned empty response")
    }
    Ok(stdout)
}

fn fetch_text_with_fallback(
    url: &str,
    accept_header: Option<&str>,
    referer: Option<&str>,
) -> Result<(String, String)> {
    let mut request = http_client().get(url);
    if let Some(accept) = accept_header {
        request = request.header("Accept", accept);
    }
    if let Some(referer) = referer {
        request = request.header("Referer", referer);
    }

    match request.send() {
        Ok(response) => {
            if !response.is_success() {
                anyhow::bail!("HTTP error: {}", response.status());
            }
            Ok((response.text()?, "attohttpc".to_string()))
        }
        Err(primary_error) => {
            #[cfg(target_os = "windows")]
            {
                match fetch_text_via_curl(url) {
                    Ok(body) => Ok((body, "curl".to_string())),
                    Err(fallback_error) => Err(anyhow::anyhow!(
                        "primary attohttpc fetch failed: {}; curl fallback failed: {}",
                        primary_error,
                        fallback_error
                    )),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(primary_error.into())
            }
        }
    }
}

pub fn normalize_daily_bar(bar: DailyBar) -> Result<DailyBar> {
    anyhow::ensure!(bar.open.is_finite(), "invalid open for {}", bar.symbol);
    anyhow::ensure!(bar.high.is_finite(), "invalid high for {}", bar.symbol);
    anyhow::ensure!(bar.low.is_finite(), "invalid low for {}", bar.symbol);
    anyhow::ensure!(bar.close.is_finite(), "invalid close for {}", bar.symbol);
    anyhow::ensure!(bar.volume.is_finite(), "invalid volume for {}", bar.symbol);
    anyhow::ensure!(bar.open > 0.0, "non-positive open for {}", bar.symbol);
    anyhow::ensure!(bar.high > 0.0, "non-positive high for {}", bar.symbol);
    anyhow::ensure!(bar.low > 0.0, "non-positive low for {}", bar.symbol);
    anyhow::ensure!(bar.close > 0.0, "non-positive close for {}", bar.symbol);
    anyhow::ensure!(bar.volume >= 0.0, "negative volume for {}", bar.symbol);
    anyhow::ensure!(
        bar.high >= bar.low,
        "high lower than low for {}",
        bar.symbol
    );
    anyhow::ensure!(
        bar.high >= bar.open.min(bar.close),
        "high inconsistent for {}",
        bar.symbol
    );
    anyhow::ensure!(
        bar.low <= bar.open.max(bar.close),
        "low inconsistent for {}",
        bar.symbol
    );
    if let Some(turnover) = bar.turnover {
        anyhow::ensure!(turnover.is_finite(), "invalid turnover for {}", bar.symbol);
        anyhow::ensure!(turnover >= 0.0, "negative turnover for {}", bar.symbol);
    }
    Ok(bar)
}

#[derive(Debug, Clone, Deserialize)]
pub struct UniverseRecord {
    pub symbol: String,
    pub name: String,
    pub display_symbol: Option<String>,
    pub instrument_type: String,
    pub market: String,
    pub category: String,
    pub eastmoney_secid: String,
    pub tencent_symbol: Option<String>,
    pub enabled: bool,
    #[serde(default = "default_latest_gate_required")]
    pub latest_gate_required: bool,
}

fn default_latest_gate_required() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct UniverseFile {
    pub instruments: Vec<UniverseRecord>,
}

pub fn load_universe(path: &Path) -> Result<Vec<Instrument>> {
    let content = fs::read_to_string(path)?;
    let file: UniverseFile = serde_json::from_str(&content)?;
    let instruments = file
        .instruments
        .into_iter()
        .map(|record| Instrument {
            symbol: record.symbol,
            name: record.name,
            display_symbol: record.display_symbol,
            instrument_type: match record.instrument_type.as_str() {
                "INDEX" => InstrumentType::Index,
                _ => InstrumentType::Etf,
            },
            market: match record.market.as_str() {
                "HK" => Market::Hk,
                _ => Market::Cn,
            },
            category: record.category,
            eastmoney_secid: record.eastmoney_secid,
            tencent_symbol: record.tencent_symbol,
            enabled: record.enabled,
            latest_gate_required: record.latest_gate_required,
        })
        .filter(|instrument| instrument.enabled)
        .collect();
    Ok(instruments)
}

fn latest_bar_date(bars: &[DailyBar]) -> Option<NaiveDate> {
    bars.iter().map(|bar| bar.date).max()
}

#[derive(Debug, Deserialize)]
struct EastmoneyResponse {
    data: Option<EastmoneyData>,
}

#[derive(Debug, Deserialize)]
struct EastmoneyData {
    klines: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TencentResponse {
    data: HashMap<String, TencentSymbolData>,
}

#[derive(Debug, Deserialize)]
struct TencentSymbolData {
    day: Option<Vec<Vec<String>>>,
    qfqday: Option<Vec<Vec<String>>>,
}

fn parse_eastmoney_kline(symbol: &str, line: &str) -> Result<DailyBar> {
    let columns = line.split(',').collect::<Vec<_>>();
    anyhow::ensure!(
        columns.len() >= 7,
        "unexpected Eastmoney kline columns for {symbol}"
    );
    Ok(DailyBar {
        date: NaiveDate::parse_from_str(columns[0], "%Y-%m-%d")?,
        symbol: symbol.to_string(),
        open: columns[1].parse()?,
        close: columns[2].parse()?,
        high: columns[3].parse()?,
        low: columns[4].parse()?,
        volume: columns[5].parse()?,
        turnover: Some(columns[6].parse()?),
    })
}

pub fn fetch_eastmoney_daily_bars(
    symbol: &str,
    secid: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyBar>> {
    let url = format!(
        "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&ut=fa5fd1943c7b386f172d6893dbfba10b&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt=101&fqt={}&beg={}&end={}",
        CANONICAL_DAILY_ADJUSTMENT.eastmoney_param(),
        from.format("%Y%m%d"),
        to.format("%Y%m%d")
    );
    let (body, _) = fetch_text_with_fallback(&url, Some("application/json,text/plain,*/*"), None)?;
    let payload: EastmoneyResponse = serde_json::from_str(&body)?;
    let lines = payload
        .data
        .and_then(|data| data.klines)
        .unwrap_or_default();
    lines
        .iter()
        .map(|line| parse_eastmoney_kline(symbol, line))
        .collect()
}

pub fn fetch_tencent_daily_bars(
    symbol: &str,
    tencent_symbol: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyBar>> {
    let url = format!(
        "https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?_var=&param={tencent_symbol},day,{},{},400,{}",
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d"),
        CANONICAL_DAILY_ADJUSTMENT.tencent_param()
    );
    let response = http_client().get(url).send()?;
    if !response.is_success() {
        anyhow::bail!("Tencent fetch failed: {}", response.status());
    }
    let payload: TencentResponse = response.json()?;
    let rows = payload
        .data
        .get(tencent_symbol)
        .and_then(|entry| entry.qfqday.clone().or_else(|| entry.day.clone()))
        .unwrap_or_default();

    rows.into_iter()
        .filter(|row| row.len() >= 6)
        .map(|row| {
            normalize_daily_bar(DailyBar {
                date: NaiveDate::parse_from_str(&row[0], "%Y-%m-%d")?,
                symbol: symbol.to_string(),
                open: row[1].parse()?,
                close: row[2].parse()?,
                high: row[3].parse()?,
                low: row[4].parse()?,
                volume: row[5].parse()?,
                turnover: row.get(6).and_then(|v| v.parse().ok()),
            })
        })
        .collect()
}

pub fn fetch_daily_bars(
    instrument: &Instrument,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyBar>> {
    match fetch_eastmoney_daily_bars(&instrument.symbol, &instrument.eastmoney_secid, from, to) {
        Ok(bars) if !bars.is_empty() => {
            if let Some(tencent_symbol) = &instrument.tencent_symbol {
                let primary_latest = latest_bar_date(&bars);
                if matches!(primary_latest, Some(latest) if latest < to) {
                    if let Ok(fallback_bars) =
                        fetch_tencent_daily_bars(&instrument.symbol, tencent_symbol, from, to)
                    {
                        let fallback_latest = latest_bar_date(&fallback_bars);
                        if fallback_latest > primary_latest {
                            return Ok(fallback_bars);
                        }
                    }
                }
            }
            Ok(bars)
        }
        Ok(_) | Err(_) => {
            if let Some(tencent_symbol) = &instrument.tencent_symbol {
                fetch_tencent_daily_bars(&instrument.symbol, tencent_symbol, from, to)
            } else {
                Ok(Vec::new())
            }
        }
    }
}

pub fn fetch_fred_series(
    factor_name: &'static str,
    series_id: &'static str,
    invert_score: bool,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<MacroFactorSeries> {
    Ok(fetch_fred_series_with_status(factor_name, series_id, invert_score, from, to)?.series)
}

pub fn fetch_fred_series_with_status(
    factor_name: &'static str,
    series_id: &'static str,
    invert_score: bool,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<MacroFetchOutcome> {
    let url = format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}");
    let (response, transport) = fetch_text_with_fallback(
        &url,
        Some("text/csv,*/*;q=0.1"),
        Some("https://fred.stlouisfed.org/"),
    )?;
    let expected_headers = [
        format!("DATE,{series_id}"),
        format!("observation_date,{series_id}"),
    ];
    let actual_header = response
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    anyhow::ensure!(
        expected_headers
            .iter()
            .any(|expected| actual_header.eq_ignore_ascii_case(expected)),
        "unexpected FRED response header for {series_id}: {actual_header}"
    );
    let mut observations = Vec::new();
    for line in response.lines().skip(1) {
        let mut parts = line.split(',');
        let Some(date_raw) = parts.next() else {
            continue;
        };
        let Some(value_raw) = parts.next() else {
            continue;
        };
        let value_raw = value_raw.trim();
        if value_raw.is_empty() || value_raw == "." {
            continue;
        }
        let date = NaiveDate::parse_from_str(date_raw.trim(), "%Y-%m-%d")?;
        if date < from || date > to {
            continue;
        }
        observations.push((date, value_raw.parse()?));
    }
    anyhow::ensure!(
        !observations.is_empty(),
        "no FRED observations available for {series_id} in range {from}..={to}"
    );
    Ok(MacroFetchOutcome {
        series: MacroFactorSeries {
            factor_name,
            source: "FRED",
            invert_score,
            observations,
        },
        transport,
    })
}

pub fn sample_bar(symbol: &str, date: NaiveDate) -> DailyBar {
    DailyBar {
        date,
        symbol: symbol.to_string(),
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        volume: 1_000_000.0,
        turnover: Some(10_000_000.0),
    }
}
