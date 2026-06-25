# V5 优化方案（修订版）

> 基于用户确认：1. 统一17:00判断收盘时间；2. 无数据或缺口超1个月提示手动操作，1个月内自动补全；3. 静默后台补全带进度条；4. 优先实施 indicator-engine + rotation-engine 的 Rayon 并行化；5. 数据拉取使用 Tokio 并发，前期用小池子控制。

---

## 一、数据缺口自动检测与补全（桌面端启动时）

### 1.1 核心逻辑：判断"期望最新数据日期"

```
期望最新数据日期 = 离当前时间 t 最近的一个已收盘交易日
判断规则：
  - 若 t 时间 >= 17:00，且 t 当日是交易日（非周末/非节假日），则期望日期 = t 当日
  - 否则，期望日期 = t 的前一个交易日
```

**实现位置**：`crates/core-domain/src/calendar.rs`

新增方法：
```rust
impl TradingCalendar {
    /// 给定当前时间，返回"期望最新数据日期"
    /// 统一按 17:00 判断收盘时间
    pub fn expected_latest_tradable_date(&self, now: chrono::DateTime<chrono::Local>) -> Option<NaiveDate> {
        let today = now.date_naive();
        let is_after_close = now.time() >= chrono::NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        
        if is_after_close && self.is_trading_day(&super::Market::Cn, today) {
            // 统一使用 CN 市场交易日判断（CN/HK 交易日历差异极小，以 CN 为准）
            Some(today)
        } else {
            self.prev_trading_day(&super::Market::Cn, today)
        }
    }
}
```

### 1.2 数据缺口检测

**实现位置**：`crates/app-service/src/lib.rs`

新增方法：
```rust
#[derive(Debug, Clone, Serialize)]
pub struct StartupFreshnessCheck {
    pub has_data: bool,              // 数据库中是否有 daily_bar 数据
    pub latest_db_date: Option<NaiveDate>,  // 数据库最新日期
    pub expected_date: Option<NaiveDate>,   // 期望最新日期
    pub gap_days: i64,               // 缺口天数（可为负数，表示数据超前）
    pub auto_ingest_eligible: bool,  // 是否允许自动补全
    pub requires_manual_action: bool, // 是否需要用户手动操作
    pub message: String,             // 提示信息
}

impl AppContext {
    /// 启动时检查数据新鲜度
    pub fn check_startup_freshness(&self) -> Result<StartupFreshnessCheck> {
        let now = chrono::Local::now();
        let expected_date = self.calendar.expected_latest_tradable_date(now);
        let latest_db_date = market_store::fetch_latest_daily_bar_date(&self.storage)?;
        
        let (has_data, gap_days, auto_ingest_eligible, requires_manual_action) = 
            match (latest_db_date, expected_date) {
                (None, _) => {
                    // 数据库中无数据
                    (false, 0, false, true)
                }
                (Some(latest), Some(expected)) => {
                    let gap = (expected - latest).num_days();
                    if gap <= 0 {
                        // 数据已是最新或超前
                        (true, gap, false, false)
                    } else if gap > 30 {
                        // 缺口超过 30 天，提示手动操作
                        (true, gap, false, true)
                    } else {
                        // 1-30 天缺口，允许自动补全
                        (true, gap, true, false)
                    }
                }
                (Some(_), None) => {
                    // 无法确定期望日期（通常发生在年末/年初 holidays 密集期）
                    (true, 0, false, false)
                }
            };
        
        let message = if !has_data {
            "数据库中无数据，请手动运行初始化流程".to_string()
        } else if requires_manual_action {
            format!("数据缺口 {} 天，超过自动补全上限，请手动运行刷新", gap_days)
        } else if auto_ingest_eligible {
            format!("检测到 {} 天数据缺口，将自动补全", gap_days)
        } else {
            "数据已是最新".to_string()
        };
        
        Ok(StartupFreshnessCheck { ... })
    }
    
    /// 自动补全数据缺口（1-30天）
    pub fn auto_ingest_gap(
        &self,
        progress_callback: Option<Box<dyn Fn(&str) + Send>>,
    ) -> Result<IngestSummary> {
        let now = chrono::Local::now();
        let expected_date = self.calendar
            .expected_latest_tradable_date(now)
            .ok_or_else(|| anyhow::anyhow!("无法确定期望最新日期"))?;
        let latest_db_date = market_store::fetch_latest_daily_bar_date(&self.storage)?
            .ok_or_else(|| anyhow::anyhow!("数据库中无数据，无法自动补全"))?;
        
        let gap_days = (expected_date - latest_db_date).num_days();
        if gap_days <= 0 {
            anyhow::bail!("数据已是最新，无需补全");
        }
        if gap_days > 30 {
            anyhow::bail!("数据缺口 {} 天超过自动补全上限，请手动操作", gap_days);
        }
        
        let from = latest_db_date + chrono::Duration::days(1); // 从数据库最新日期的次日开始
        let to = expected_date;
        
        self.ingest_daily(from, to, progress_callback.as_ref().map(|cb| cb.as_ref()))
    }
}
```

### 1.3 桌面端启动时触发

**Tauri 命令层**：`apps/desktop/src-tauri/src/lib.rs`

新增命令：
```rust
#[tauri::command]
fn check_startup_freshness() -> Result<serde_json::Value, String> {
    let context = AppContext::new(StorageConfig::default());
    let check = context.check_startup_freshness().map_err(|e| e.to_string())?;
    serde_json::to_value(check).map_err(|e| e.to_string())
}

#[tauri::command]
async fn auto_ingest_on_startup(
    refresh: tauri::State<'_, RefreshCoordinator>,
) -> Result<DashboardRefreshStatus, String> {
    // 复用现有的 RefreshCoordinator 机制
    // 但只运行 ingest 阶段，不运行后续计算阶段
    let context = AppContext::new(StorageConfig::default());
    let check = context.check_startup_freshness().map_err(|e| e.to_string())?;
    
    if !check.auto_ingest_eligible {
        return Ok(DashboardRefreshStatus {
            status: "skipped".to_string(),
            message: check.message,
            ..Default::default()
        });
    }
    
    // 启动后台 ingest 任务
    spawn_auto_ingest(refresh.inner().clone())
}
```

**前端启动时**：`apps/desktop/frontend/src/main.js`

在 `loadAndApplyPreferences().then(() => loadDashboard())` 之前，插入：
```javascript
async function startupFreshnessCheck() {
  try {
    const check = await invoke('check_startup_freshness');
    if (check.requires_manual_action) {
      // 显示顶部提示条，告知用户需要手动操作
      dashboardStore.startupNotice = {
        type: 'warning',
        message: check.message,
        action: 'manual_refresh',
      };
    } else if (check.auto_ingest_eligible) {
      // 显示"正在自动补全数据"提示，启动后台刷新
      dashboardStore.startupNotice = {
        type: 'info',
        message: check.message,
        action: 'auto_ingesting',
      };
      // 启动自动 ingest（复用 refresh 进度机制）
      await invoke('auto_ingest_on_startup');
      // 开始轮询进度
      scheduleRefreshPoll(500);
    }
  } catch (error) {
    console.error('[Startup] Freshness check failed:', error);
  }
}

// 修改启动顺序
loadAndApplyPreferences()
  .then(() => startupFreshnessCheck())
  .then(() => loadDashboard());
```

**进度条集成**：复用现有的 `RefreshProgress` 组件和 `pollRefreshStatus` 机制，但需要在 `DashboardRefreshStatus` 中新增 `is_auto_ingest` 字段，以便前端区分"用户手动 refresh"和"启动时自动 ingest"的UI表现。

---

## 二、Rayon 并行化（CPU密集型计算）

### 2.1 引入 Rayon

在 `Cargo.toml` 中新增 workspace dependency：
```toml
rayon = "1.10"
```

### 2.2 indicator-engine 并行化

**文件**：`crates/indicator-engine/src/lib.rs`

当前 `build_indicator_snapshots` 的调用方式：
```rust
// app-service 中：
let all_bars = market_store::fetch_daily_bars_for_symbols_in_range(...)?;
// all_bars 是 Vec<DailyBar>，需要按 symbol 分组
let mut snapshots = Vec::new();
for bars in symbol_bars {
    snapshots.extend(build_indicator_snapshots(&bars));
}
```

优化方案：
```rust
// indicator-engine 中新增按 symbol 并行入口：
use rayon::prelude::*;

pub fn build_indicator_snapshots_for_symbols(
    bars_by_symbol: &HashMap<String, Vec<DailyBar>>,
) -> Vec<IndicatorSnapshot> {
    bars_by_symbol
        .par_iter()  // Rayon 并行迭代
        .flat_map(|(_symbol, bars)| build_indicator_snapshots(bars))
        .collect()
}
```

### 2.3 rotation-engine 并行化

**文件**：`crates/rotation-engine/src/lib.rs`

优化方案：
```rust
use rayon::prelude::*;

pub fn build_rotation_ranks_parallel(
    series_by_symbol: &BTreeMap<String, Vec<DailyBar>>,
) -> Vec<RotationRankSnapshot> {
    // Step 1: per-symbol RS 计算并行
    let mut daily_rows: BTreeMap<chrono::NaiveDate, Vec<RotationRankSnapshot>> = BTreeMap::new();
    
    let per_symbol_results: Vec<Vec<RotationRankSnapshot>> = series_by_symbol
        .par_iter()
        .map(|(symbol, bars)| {
            let mut rows = Vec::new();
            let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
            for (index, bar) in bars.iter().enumerate() {
                let Some(rs_20) = compute_rs_window(&closes, index, 20) else { continue };
                let rs_60 = compute_rs_window(&closes, index, 60).unwrap_or(rs_20);
                let rs_120 = compute_rs_window(&closes, index, 120).unwrap_or(rs_60);
                let momentum_score = rs_20 * 0.5 + rs_60 * 0.3 + rs_120 * 0.2;
                rows.push(RotationRankSnapshot { ... });
            }
            rows
        })
        .collect();
    
    // Step 2: 合并结果到 daily_rows
    for rows in per_symbol_results {
        for row in rows {
            daily_rows.entry(row.date).or_default().push(row);
        }
    }
    
    // Step 3: per-day 排名并行
    let mut ranked: Vec<RotationRankSnapshot> = daily_rows
        .into_par_iter()  // 按日期并行排名
        .flat_map(|(_date, mut rows)| {
            rows.sort_by(...);
            for (index, row) in rows.iter_mut().enumerate() {
                row.rank = (index + 1) as u32;
            }
            rows
        })
        .collect();
    
    ranked
}
```

### 2.4 风险控制

- **线程池大小**：Rayon 默认使用全局线程池，线程数 = CPU 核心数。需要确保不会压垮 ClickHouse 连接。
- **与 Tokio 的兼容性**：在 `tokio::task::spawn_blocking` 中调用 Rayon 代码，避免阻塞 Tokio 的异步运行时。

---

## 三、Tokio 并发拉取（数据获取阶段）

### 3.1 并发控制策略

**目标**：并行化 HTTP 数据拉取，但控制并发数以避免触发 provider 的 rate limit。

**方案**：使用 `tokio::sync::Semaphore` 控制并发数。

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

const MAX_CONCURRENT_FETCH: usize = 4; // 前期先用小池子

pub async fn ingest_daily_parallel(
    &self,
    from: NaiveDate,
    to: NaiveDate,
    progress_callback: Option<&dyn Fn(&str)>,
) -> Result<IngestSummary> {
    let instruments = load_universe(&self.storage.universe_abspath()?)?;
    let total = instruments.len();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FETCH));
    let mut tasks = Vec::new();
    
    for (idx, instrument) in instruments.iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await?;
        let instrument = instrument.clone();
        let task = tokio::spawn(async move {
            let _permit = permit; // 持有 permit 直到任务完成
            let result = fetch_daily_bars(&instrument, from, to).await;
            (instrument.symbol.clone(), result)
        });
        tasks.push(task);
    }
    
    let mut total_rows = 0usize;
    let mut failed_symbols = Vec::new();
    let mut all_bars = Vec::new();
    
    for task in tasks {
        let (symbol, result) = task.await?;
        match result {
            Ok(bars) => {
                total_rows += bars.len();
                all_bars.extend(bars);
            }
            Err(error) => {
                failed_symbols.push(format!("{}: {}", symbol, error));
            }
        }
    }
    
    // 批量写入 ClickHouse
    market_store::insert_daily_bars(&self.storage, &all_bars)?;
    
    Ok(IngestSummary { total_rows, failed_symbols })
}
```

### 3.2 关键问题

1. **`fetch_daily_bars` 当前是同步函数**：需要检查 `data-ingestion` 中的 `fetch_daily_bars` 是否是 async。
2. **ClickHouse 写入批量优化**：当前是逐 symbol 写入，可以改为先收集所有 bars，然后批量写入。

---

## 四、实施优先级与风险

### 优先级
1. **Phase 1**：数据缺口自动检测（`core-domain` + `app-service`）— 影响产品体验
2. **Phase 2**：桌面端启动时自动触发（`src-tauri` + 前端）— 影响产品体验
3. **Phase 3**：indicator-engine Rayon 并行化 — 性能提升最大
4. **Phase 4**：rotation-engine Rayon 并行化 — 性能提升次之
5. **Phase 5**：data-ingestion Tokio 并发拉取 — 风险最高（需确认 provider rate limit）

### 风险
1. **Rayon 与 Tokio 混用**：在 async 上下文中直接调用 Rayon 可能阻塞 Tokio 运行时。必须确保 Rayon 调用在 `spawn_blocking` 中。
2. **ClickHouse 连接压力**：并行计算后写入 ClickHouse 的并发增加，需要确认 ClickHouse 的 HTTP 连接池配置。
3. **Provider Rate Limit**：Tencent 的并发限制未知。前期使用 `MAX_CONCURRENT_FETCH = 4`，后续可根据实际测试调整。

---

## 五、文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `Cargo.toml` | 修改 | 新增 `rayon = "1.10"` workspace dependency |
| `crates/core-domain/src/calendar.rs` | 修改 | 新增 `expected_latest_tradable_date` |
| `crates/core-domain/src/lib.rs` | 修改 | 导出 `StartupFreshnessCheck`（如需要） |
| `crates/app-service/src/lib.rs` | 修改 | 新增 `check_startup_freshness` + `auto_ingest_gap` |
| `apps/desktop/src-tauri/src/lib.rs` | 修改 | 新增 `check_startup_freshness` + `auto_ingest_on_startup` 命令 |
| `apps/desktop/frontend/src/main.js` | 修改 | 启动时调用 freshness check |
| `apps/desktop/frontend/src/store.js` | 修改 | 新增 `startupNotice` 状态 |
| `crates/indicator-engine/Cargo.toml` | 修改 | 新增 `rayon` dependency |
| `crates/indicator-engine/src/lib.rs` | 修改 | 新增 `build_indicator_snapshots_for_symbols` 并行入口 |
| `crates/rotation-engine/Cargo.toml` | 修改 | 新增 `rayon` dependency |
| `crates/rotation-engine/src/lib.rs` | 修改 | 新增 `build_rotation_ranks_parallel` |
| `crates/data-ingestion/src/lib.rs` | 修改 | 新增 `fetch_daily_bars_parallel`（可选） |
| `crates/app-service/src/lib.rs` | 修改 | `compute_indicators` 调用并行化入口 |
| `crates/app-service/src/lib.rs` | 修改 | `compute_rotation` 调用并行化入口 |
| `crates/app-service/src/lib.rs` | 修改 | `ingest_daily` 可选使用 Tokio 并发（Phase 5） |
