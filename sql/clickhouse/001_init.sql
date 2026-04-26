CREATE DATABASE IF NOT EXISTS quant;

CREATE TABLE IF NOT EXISTS quant.instrument
(
    symbol String,
    name String,
    display_symbol Nullable(String),
    instrument_type LowCardinality(String),
    market LowCardinality(String),
    category LowCardinality(String),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
ORDER BY (symbol);

CREATE TABLE IF NOT EXISTS quant.daily_bar
(
    date Date,
    symbol String,
    open Float64,
    high Float64,
    low Float64,
    close Float64,
    volume Float64,
    turnover Nullable(Float64),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (symbol, date);

CREATE TABLE IF NOT EXISTS quant.indicator_snapshot
(
    date Date,
    symbol String,
    ma10 Nullable(Float64),
    ma20 Nullable(Float64),
    ma30 Nullable(Float64),
    ma60 Nullable(Float64),
    ma120 Nullable(Float64),
    ema12 Nullable(Float64),
    ema26 Nullable(Float64),
    macd Nullable(Float64),
    macd_signal Nullable(Float64),
    macd_hist Nullable(Float64),
    rsi14 Nullable(Float64),
    atr14 Nullable(Float64),
    vol_ma20 Nullable(Float64),
    vol_ma60 Nullable(Float64),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (symbol, date);

CREATE TABLE IF NOT EXISTS quant.macro_snapshot
(
    date Date,
    factor_name LowCardinality(String),
    factor_value Float64,
    factor_score Float64,
    factor_source LowCardinality(String),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (factor_name, date);

CREATE TABLE IF NOT EXISTS quant.market_regime
(
    date Date,
    macro_as_of_date Date DEFAULT date,
    market LowCardinality(String),
    trend_score Float64,
    liquidity_score Float64,
    risk_score Float64,
    regime_label LowCardinality(String),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (market, date);

CREATE TABLE IF NOT EXISTS quant.environment_snapshot
(
    date Date,
    scope LowCardinality(String),
    regime_as_of_date Date,
    breadth_as_of_date Date,
    stress_as_of_date Date,
    breadth_eligible_count UInt32,
    breadth_above_count UInt32,
    breadth_pct Float64,
    breadth_pct_sma5 Nullable(Float64),
    breadth_5d_delta Nullable(Float64),
    breadth_state LowCardinality(String),
    volume_expansion_pct Nullable(Float64),
    turnover_coverage_pct Nullable(Float64),
    liquidity_proxy_score Float64,
    stress_proxy_score Float64,
    environment_score Float64,
    environment_label LowCardinality(String),
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (scope, date);

CREATE TABLE IF NOT EXISTS quant.rotation_rank
(
    date Date,
    symbol String,
    rs_20 Float64,
    rs_60 Float64,
    rs_120 Float64,
    momentum_score Float64,
    rank UInt32,
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (date, rank, symbol);

CREATE TABLE IF NOT EXISTS quant.strategy_preference
(
    date Date,
    symbol String,
    analysis_scope LowCardinality(String) DEFAULT 'GLOBAL',
    regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL',
    value_left_score Float64,
    trend_pullback_score Float64,
    trend_breakout_score Float64,
    momentum_right_score Float64,
    best_strategy LowCardinality(String),
    confidence Float64,
    alignment UInt8,
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (symbol, date);

CREATE TABLE IF NOT EXISTS quant.signal_snapshot
(
    date Date,
    symbol String,
    final_score Float64,
    signal_label LowCardinality(String),
    analysis_scope LowCardinality(String) DEFAULT 'GLOBAL',
    regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL',
    explanation String,
    updated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (symbol, date);

CREATE TABLE IF NOT EXISTS quant.backtest_run
(
    run_id String,
    strategy_name LowCardinality(String),
    analysis_scope LowCardinality(String) DEFAULT 'GLOBAL',
    signal_scope LowCardinality(String) DEFAULT 'GLOBAL',
    regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL',
    signal_start_date Nullable(Date),
    signal_end_date Nullable(Date),
    config_summary String DEFAULT '',
    started_at DateTime,
    finished_at Nullable(DateTime),
    cagr Nullable(Float64),
    max_drawdown Nullable(Float64),
    sharpe Nullable(Float64)
)
ENGINE = MergeTree
ORDER BY (run_id, started_at);

CREATE TABLE IF NOT EXISTS quant.backtest_trade
(
    run_id String,
    trade_date Date,
    symbol String,
    action LowCardinality(String),
    price Float64,
    quantity Float64,
    trade_value Float64
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(trade_date)
ORDER BY (run_id, trade_date, symbol);

CREATE TABLE IF NOT EXISTS quant.backtest_equity_curve
(
    run_id String,
    date Date,
    equity Float64,
    drawdown Float64
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(date)
ORDER BY (run_id, date);

CREATE TABLE IF NOT EXISTS quant.report_snapshot
(
    report_date Date,
    report_type LowCardinality(String),
    artifact_path String,
    generated_at DateTime DEFAULT now()
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(report_date)
ORDER BY (report_type, report_date);
