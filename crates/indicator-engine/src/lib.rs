use core_domain::{DailyBar, IndicatorSnapshot};

fn rolling_mean(values: &[f64], period: usize, index: usize) -> Option<f64> {
    if index + 1 < period {
        return None;
    }
    let window = &values[index + 1 - period..=index];
    Some(window.iter().sum::<f64>() / period as f64)
}

fn ema_series(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut result = vec![None; values.len()];
    if values.len() < period {
        return result;
    }
    let seed = values[..period].iter().sum::<f64>() / period as f64;
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut previous = seed;
    result[period - 1] = Some(seed);
    for index in period..values.len() {
        previous = ((values[index] - previous) * multiplier) + previous;
        result[index] = Some(previous);
    }
    result
}

fn rsi_series(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut result = vec![None; values.len()];
    if values.len() <= period {
        return result;
    }

    let mut gains = 0.0;
    let mut losses = 0.0;
    for index in 1..=period {
        let change = values[index] - values[index - 1];
        if change >= 0.0 {
            gains += change;
        } else {
            losses += -change;
        }
    }

    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    result[period] = Some(if avg_loss == 0.0 {
        100.0
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    });

    for index in period + 1..values.len() {
        let change = values[index] - values[index - 1];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };
        avg_gain = ((avg_gain * (period as f64 - 1.0)) + gain) / period as f64;
        avg_loss = ((avg_loss * (period as f64 - 1.0)) + loss) / period as f64;
        result[index] = Some(if avg_loss == 0.0 {
            100.0
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - (100.0 / (1.0 + rs))
        });
    }

    result
}

fn atr_series(bars: &[DailyBar], period: usize) -> Vec<Option<f64>> {
    let mut true_ranges = Vec::with_capacity(bars.len());
    for (index, bar) in bars.iter().enumerate() {
        let tr = if index == 0 {
            bar.high - bar.low
        } else {
            let prev_close = bars[index - 1].close;
            let high_low = bar.high - bar.low;
            let high_close = (bar.high - prev_close).abs();
            let low_close = (bar.low - prev_close).abs();
            high_low.max(high_close).max(low_close)
        };
        true_ranges.push(tr);
    }

    let mut result = vec![None; bars.len()];
    if bars.len() < period {
        return result;
    }
    let seed = true_ranges[..period].iter().sum::<f64>() / period as f64;
    let mut previous = seed;
    result[period - 1] = Some(seed);
    for index in period..bars.len() {
        previous = ((previous * (period as f64 - 1.0)) + true_ranges[index]) / period as f64;
        result[index] = Some(previous);
    }
    result
}

pub fn build_indicator_snapshots(bars: &[DailyBar]) -> Vec<IndicatorSnapshot> {
    if bars.is_empty() {
        return Vec::new();
    }

    let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
    let volumes: Vec<f64> = bars.iter().map(|bar| bar.volume).collect();
    let ema12 = ema_series(&closes, 12);
    let ema26 = ema_series(&closes, 26);
    let rsi14 = rsi_series(&closes, 14);
    let atr14 = atr_series(bars, 14);

    let macd_line: Vec<Option<f64>> = ema12
        .iter()
        .zip(ema26.iter())
        .map(|(fast, slow)| match (fast, slow) {
            (Some(fast), Some(slow)) => Some(fast - slow),
            _ => None,
        })
        .collect();

    let macd_values: Vec<f64> = macd_line.iter().flatten().copied().collect();
    let macd_signal_values = ema_series(&macd_values, 9);
    let mut macd_signal = vec![None; bars.len()];
    let mut cursor = 0usize;
    for (index, value) in macd_line.iter().enumerate() {
        if value.is_some() {
            macd_signal[index] = macd_signal_values[cursor];
            cursor += 1;
        }
    }

    bars.iter()
        .enumerate()
        .map(|(index, bar)| {
            let macd = macd_line[index];
            let signal = macd_signal[index];
            IndicatorSnapshot {
                date: bar.date,
                symbol: bar.symbol.clone(),
                ma10: rolling_mean(&closes, 10, index),
                ma20: rolling_mean(&closes, 20, index),
                ma30: rolling_mean(&closes, 30, index),
                ma60: rolling_mean(&closes, 60, index),
                ma120: rolling_mean(&closes, 120, index),
                ema12: ema12[index],
                ema26: ema26[index],
                macd,
                macd_signal: signal,
                macd_hist: match (macd, signal) {
                    (Some(macd), Some(signal)) => Some(macd - signal),
                    _ => None,
                },
                rsi14: rsi14[index],
                atr14: atr14[index],
                vol_ma20: rolling_mean(&volumes, 20, index),
                vol_ma60: rolling_mean(&volumes, 60, index),
            }
        })
        .collect()
}
